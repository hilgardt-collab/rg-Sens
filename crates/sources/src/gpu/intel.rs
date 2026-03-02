//! Intel GPU backend using sysfs (i915 and xe drivers)

use super::backend::{GpuBackend, GpuInfo, GpuMetrics, GpuVendor};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Intel kernel driver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntelDriver {
    I915,
    Xe,
}

/// Intel GPU backend
pub struct IntelBackend {
    info: GpuInfo,
    metrics: GpuMetrics,
    device_path: PathBuf,
    card_path: PathBuf,
    hwmon_path: Option<PathBuf>,
    driver: IntelDriver,
    is_discrete: bool,
    /// Cached path for temperature reading
    cached_temp_path: Option<PathBuf>,
    /// Cached path for power reading (μW direct)
    cached_power_path: Option<PathBuf>,
    /// Cached fan reading method: Some(true) = pwm, Some(false) = fan1_input, None = not discovered
    cached_fan_method: Option<bool>,
    /// Cached path for current frequency
    cached_freq_cur_path: Option<PathBuf>,
    /// Cached path for max frequency
    cached_freq_max_path: Option<PathBuf>,
    /// Previous energy counter reading (μJ) for power calculation fallback
    last_energy_uj: Option<u64>,
    /// Timestamp of previous energy reading
    last_energy_time: Option<Instant>,
}

impl IntelBackend {
    /// Create a new Intel backend for the specified DRM card index
    pub fn new(card_index: u32) -> Result<Self> {
        let device_path = PathBuf::from(format!("/sys/class/drm/card{}/device", card_index));
        let card_path = PathBuf::from(format!("/sys/class/drm/card{}", card_index));

        if !device_path.exists() {
            return Err(anyhow!("Intel GPU card{} not found", card_index));
        }

        // Verify it's an Intel GPU by checking vendor
        let vendor_id = Self::read_hex_file(&device_path.join("vendor"))?;
        if vendor_id != 0x8086 {
            return Err(anyhow!(
                "card{} is not an Intel GPU (vendor ID: 0x{:04x})",
                card_index,
                vendor_id
            ));
        }

        // Detect kernel driver from symlink
        let driver = Self::detect_driver(&device_path)?;

        // Get device ID for name lookup and discrete detection
        let device_id = Self::read_hex_file(&device_path.join("device"))?;
        let is_discrete = Self::is_discrete_gpu(device_id);
        let name = Self::get_gpu_name(device_id)
            .unwrap_or_else(|| format!("Intel GPU {}", card_index));

        // Find hwmon directory for sensors
        let hwmon_path = Self::find_hwmon_path(&device_path)?;

        Ok(Self {
            info: GpuInfo {
                index: card_index,
                name,
                vendor: GpuVendor::Intel,
            },
            metrics: GpuMetrics::default(),
            device_path,
            card_path,
            hwmon_path,
            driver,
            is_discrete,
            cached_temp_path: None,
            cached_power_path: None,
            cached_fan_method: None,
            cached_freq_cur_path: None,
            cached_freq_max_path: None,
            last_energy_uj: None,
            last_energy_time: None,
        })
    }

    /// Read a hexadecimal value from a sysfs file
    fn read_hex_file(path: &Path) -> Result<u32> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let trimmed = content.trim().trim_start_matches("0x");
        u32::from_str_radix(trimmed, 16)
            .with_context(|| format!("Failed to parse hex value from {}", path.display()))
    }

    /// Read an integer value from a sysfs file
    fn read_int_file(path: &Path) -> Result<i64> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        content
            .trim()
            .parse::<i64>()
            .with_context(|| format!("Failed to parse integer from {}", path.display()))
    }

    /// Read an unsigned integer value from a sysfs file
    fn read_uint_file(path: &Path) -> Result<u64> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        content
            .trim()
            .parse::<u64>()
            .with_context(|| format!("Failed to parse unsigned integer from {}", path.display()))
    }

    /// Find the hwmon directory for this GPU
    fn find_hwmon_path(device_path: &Path) -> Result<Option<PathBuf>> {
        let hwmon_dir = device_path.join("hwmon");
        if !hwmon_dir.exists() {
            return Ok(None);
        }

        for entry in fs::read_dir(&hwmon_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("hwmon"))
            {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Detect kernel driver (i915 or xe) from the driver symlink
    fn detect_driver(device_path: &Path) -> Result<IntelDriver> {
        let driver_link = device_path.join("driver");
        if !driver_link.exists() {
            return Err(anyhow!("No driver symlink found for Intel GPU"));
        }

        let target = fs::read_link(&driver_link)
            .with_context(|| format!("Failed to read driver symlink at {}", driver_link.display()))?;

        let driver_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match driver_name {
            "xe" => Ok(IntelDriver::Xe),
            "i915" => Ok(IntelDriver::I915),
            other => {
                log::warn!("Unknown Intel GPU driver '{}', assuming i915", other);
                Ok(IntelDriver::I915)
            }
        }
    }

    /// Check if a device ID corresponds to a discrete (Arc) GPU
    fn is_discrete_gpu(device_id: u32) -> bool {
        matches!(
            device_id,
            // Arc A-series (Alchemist) desktop
            0x56A0..=0x56AF |
            // Arc A-series mobile
            0x56B0..=0x56BF |
            // Arc B-series (Battlemage)
            0xE202 | 0xE20B | 0xE20C | 0xE20D |
            // Arc Pro series
            0x56C0..=0x56CF
        )
    }

    /// Get a human-readable GPU name from device ID
    fn get_gpu_name(device_id: u32) -> Option<String> {
        let name = match device_id {
            // Arc B-series (Battlemage)
            0xE202 => "Arc B580",
            0xE20B => "Arc B570",
            0xE20C | 0xE20D => "Arc B-Series",

            // Arc A-series (Alchemist) desktop
            0x56A0 => "Arc A770",
            0x56A1 => "Arc A750",
            0x56A5 => "Arc A580",
            0x56A6 => "Arc A380",

            // Arc A-series mobile
            0x56B0 => "Arc A770M",
            0x56B1 => "Arc A730M",
            0x56B2 => "Arc A550M",
            0x56B3 => "Arc A370M",

            // Arc Pro series
            0x56C0 => "Arc Pro A60",
            0x56C1 => "Arc Pro A40/A50",

            // Arrow Lake / Lunar Lake integrated
            0xE20E..=0xE20F => "Arrow Lake Graphics",
            0x6420..=0x642F => "Arrow Lake Graphics",
            0x64A0..=0x64AF => "Lunar Lake Graphics",

            // Meteor Lake integrated
            0x7D40..=0x7D67 => "Meteor Lake Graphics",

            // Raptor Lake / Alder Lake integrated
            0xA780..=0xA78F => "Raptor Lake Graphics",
            0xA720..=0xA72F => "Raptor Lake Graphics",
            0x4680..=0x468F => "Alder Lake Graphics",
            0x46A0..=0x46AF => "Alder Lake Graphics",

            // Tiger Lake
            0x9A40..=0x9A4F => "Iris Xe (Tiger Lake)",
            0x9A60..=0x9A6F => "Iris Xe (Tiger Lake)",
            0x9A70..=0x9A7F => "Iris Xe (Tiger Lake)",

            // Rocket Lake
            0x4C80..=0x4C8F => "UHD 750 (Rocket Lake)",

            // UHD 770/730 (Alder Lake)
            0x4690..=0x469F => "UHD 770",
            0x46B0..=0x46BF => "UHD 730",

            _ => return None,
        };
        Some(format!("Intel {}", name))
    }

    /// Try to read temperature from hwmon (caches successful path)
    fn read_temperature(&mut self) -> Option<f32> {
        let hwmon = self.hwmon_path.as_ref()?;

        // Use cached path if available
        if let Some(ref cached_path) = self.cached_temp_path {
            if let Ok(value) = Self::read_int_file(cached_path) {
                return Some(value as f32 / 1000.0);
            }
            self.cached_temp_path = None;
        }

        // xe driver tends to have the GPU temp at temp2, i915 at temp1
        let temp_files: &[&str] = match self.driver {
            IntelDriver::Xe => &["temp2_input", "temp1_input", "temp3_input"],
            IntelDriver::I915 => &["temp1_input", "temp2_input", "temp3_input"],
        };

        for temp_file in temp_files {
            let path = hwmon.join(temp_file);
            if let Ok(value) = Self::read_int_file(&path) {
                self.cached_temp_path = Some(path);
                return Some(value as f32 / 1000.0);
            }
        }
        None
    }

    /// Try to read current core clock frequency in MHz (caches successful path)
    fn read_core_clock(&mut self) -> Option<u32> {
        // Use cached path if available
        if let Some(ref cached_path) = self.cached_freq_cur_path {
            if let Ok(value) = Self::read_int_file(cached_path) {
                return Some(value.max(0) as u32);
            }
            self.cached_freq_cur_path = None;
        }

        let paths: Vec<PathBuf> = match self.driver {
            IntelDriver::I915 => vec![
                self.card_path.join("gt_cur_freq_mhz"),
                self.card_path.join("gt/gt0/rps_act_freq_mhz"),
            ],
            IntelDriver::Xe => vec![
                self.device_path.join("tile0/gt0/freq0/cur_freq"),
            ],
        };

        for path in paths {
            if let Ok(value) = Self::read_int_file(&path) {
                self.cached_freq_cur_path = Some(path);
                return Some(value.max(0) as u32);
            }
        }
        None
    }

    /// Try to read max core clock frequency in MHz (caches successful path)
    fn read_max_clock(&mut self) -> Option<u32> {
        // Use cached path if available
        if let Some(ref cached_path) = self.cached_freq_max_path {
            if let Ok(value) = Self::read_int_file(cached_path) {
                return Some(value.max(0) as u32);
            }
            self.cached_freq_max_path = None;
        }

        let paths: Vec<PathBuf> = match self.driver {
            IntelDriver::I915 => vec![
                self.card_path.join("gt_max_freq_mhz"),
                self.card_path.join("gt/gt0/rps_RP0_freq_mhz"),
            ],
            IntelDriver::Xe => vec![
                self.device_path.join("tile0/gt0/freq0/max_freq"),
            ],
        };

        for path in paths {
            if let Ok(value) = Self::read_int_file(&path) {
                self.cached_freq_max_path = Some(path);
                return Some(value.max(0) as u32);
            }
        }
        None
    }

    /// Estimate GPU utilization from frequency ratio (cur_freq / max_freq * 100)
    ///
    /// Intel GPUs don't expose a direct utilization counter via sysfs.
    /// The frequency ratio is a reasonable proxy: the GPU driver scales frequency
    /// based on workload, so high frequency ≈ high utilization.
    fn read_utilization(&mut self) -> Option<u32> {
        let cur = self.read_core_clock()?;
        let max = self.read_max_clock()?;
        if max == 0 {
            return None;
        }
        Some(((cur as f64 / max as f64) * 100.0).clamp(0.0, 100.0) as u32)
    }

    /// Try to read VRAM usage (discrete GPUs only)
    fn read_memory_info(&mut self) {
        if !self.is_discrete {
            return;
        }

        let vram_used_path = self.device_path.join("mem_info_vram_used");
        if let Ok(used) = Self::read_int_file(&vram_used_path) {
            self.metrics.memory_used = Some(used.max(0) as u64);
        }

        let vram_total_path = self.device_path.join("mem_info_vram_total");
        if let Ok(total) = Self::read_int_file(&vram_total_path) {
            self.metrics.memory_total = Some(total.max(0) as u64);
        }
    }

    /// Try to read power usage in watts (caches successful path).
    /// Falls back to energy counter delta if direct power reading is unavailable.
    fn read_power_usage(&mut self) -> Option<f32> {
        let hwmon = self.hwmon_path.as_ref()?;

        // Use cached path if available
        if let Some(ref cached_path) = self.cached_power_path {
            if let Ok(value) = Self::read_int_file(cached_path) {
                // power1_input is in microwatts
                return Some(value as f32 / 1_000_000.0);
            }
            self.cached_power_path = None;
        }

        // Try direct power reading (μW)
        let power_path = hwmon.join("power1_input");
        if let Ok(value) = Self::read_int_file(&power_path) {
            self.cached_power_path = Some(power_path);
            return Some(value as f32 / 1_000_000.0);
        }

        // Fallback: compute power from energy counter delta (μJ)
        let energy_path = hwmon.join("energy1_input");
        if let Ok(energy_uj) = Self::read_uint_file(&energy_path) {
            let now = Instant::now();
            let power = if let (Some(prev_energy), Some(prev_time)) =
                (self.last_energy_uj, self.last_energy_time)
            {
                let dt = now.duration_since(prev_time).as_secs_f64();
                if dt > 0.0 && energy_uj >= prev_energy {
                    let delta_uj = energy_uj - prev_energy;
                    Some(delta_uj as f32 / (dt as f32 * 1_000_000.0))
                } else {
                    None
                }
            } else {
                None
            };
            self.last_energy_uj = Some(energy_uj);
            self.last_energy_time = Some(now);
            return power;
        }

        None
    }

    /// Try to read fan speed (discrete GPUs only, caches successful method)
    fn read_fan_speed(&mut self) -> Option<u32> {
        if !self.is_discrete {
            return None;
        }

        let hwmon = self.hwmon_path.as_ref()?;

        // Use cached method if available
        if let Some(use_pwm) = self.cached_fan_method {
            if use_pwm {
                let pwm_path = hwmon.join("pwm1");
                if let Ok(pwm) = Self::read_int_file(&pwm_path) {
                    return Some((pwm as f32 / 255.0 * 100.0) as u32);
                }
            } else {
                let fan_path = hwmon.join("fan1_input");
                let fan_max_path = hwmon.join("fan1_max");
                if let Ok(rpm) = Self::read_int_file(&fan_path) {
                    if let Ok(max_rpm) = Self::read_int_file(&fan_max_path) {
                        if max_rpm > 0 {
                            return Some(
                                ((rpm as f64 / max_rpm as f64) * 100.0).clamp(0.0, 100.0) as u32,
                            );
                        }
                    }
                }
            }
            self.cached_fan_method = None;
        }

        // Try PWM (0-255)
        let pwm_path = hwmon.join("pwm1");
        if let Ok(pwm) = Self::read_int_file(&pwm_path) {
            self.cached_fan_method = Some(true);
            return Some((pwm as f32 / 255.0 * 100.0) as u32);
        }

        // Try fan1_input (RPM) with fan1_max for percentage
        let fan_path = hwmon.join("fan1_input");
        let fan_max_path = hwmon.join("fan1_max");
        if let Ok(rpm) = Self::read_int_file(&fan_path) {
            if let Ok(max_rpm) = Self::read_int_file(&fan_max_path) {
                if max_rpm > 0 {
                    self.cached_fan_method = Some(false);
                    return Some(((rpm as f64 / max_rpm as f64) * 100.0).clamp(0.0, 100.0) as u32);
                }
            }
        }

        None
    }
}

impl GpuBackend for IntelBackend {
    fn info(&self) -> &GpuInfo {
        &self.info
    }

    fn update(&mut self) -> Result<()> {
        self.metrics.temperature = self.read_temperature();

        // Utilization reads clocks internally (cur/max ratio)
        self.metrics.utilization = self.read_utilization();
        // Store the core clock from the cached path (populated by read_utilization)
        self.metrics.clock_core = self.read_core_clock();

        self.read_memory_info();
        self.metrics.power_usage = self.read_power_usage();
        self.metrics.fan_speed = self.read_fan_speed();

        // Memory clock is not reliably exposed via sysfs for Intel GPUs
        self.metrics.clock_memory = None;

        Ok(())
    }

    fn metrics(&self) -> &GpuMetrics {
        &self.metrics
    }

    fn is_available(&self) -> bool {
        self.device_path.exists()
    }
}
