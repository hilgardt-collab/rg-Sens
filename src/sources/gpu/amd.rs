//! AMD GPU backend using sysfs

use super::backend::{GpuBackend, GpuInfo, GpuMetrics, GpuVendor};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// AMD GPU backend
pub struct AmdBackend {
    info: GpuInfo,
    metrics: GpuMetrics,
    device_path: PathBuf,
    hwmon_path: Option<PathBuf>,
}

impl AmdBackend {
    /// Create a new AMD backend for the specified DRM card index
    pub fn new(card_index: u32) -> Result<Self> {
        let device_path = PathBuf::from(format!("/sys/class/drm/card{}/device", card_index));

        if !device_path.exists() {
            return Err(anyhow!("AMD GPU card{} not found", card_index));
        }

        // Verify it's an AMD GPU by checking vendor
        let vendor_id = Self::read_hex_file(&device_path.join("vendor"))?;
        if vendor_id != 0x1002 {
            return Err(anyhow!("card{} is not an AMD GPU (vendor ID: 0x{:04x})", card_index, vendor_id));
        }

        // Get GPU name from device ID
        let device_id = Self::read_hex_file(&device_path.join("device"))?;
        let name = Self::get_gpu_name(device_id).unwrap_or_else(|| format!("AMD GPU {}", card_index));

        // Find hwmon directory for sensors
        let hwmon_path = Self::find_hwmon_path(&device_path)?;

        Ok(Self {
            info: GpuInfo {
                index: card_index,
                name,
                vendor: GpuVendor::Amd,
            },
            metrics: GpuMetrics::default(),
            device_path,
            hwmon_path,
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
        content.trim().parse::<i64>()
            .with_context(|| format!("Failed to parse integer from {}", path.display()))
    }

    /// Find the hwmon directory for this GPU
    fn find_hwmon_path(device_path: &Path) -> Result<Option<PathBuf>> {
        let hwmon_dir = device_path.join("hwmon");
        if !hwmon_dir.exists() {
            return Ok(None);
        }

        // Look for hwmon* subdirectory
        for entry in fs::read_dir(&hwmon_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with("hwmon")) {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Get a human-readable GPU name from device ID (basic mapping)
    fn get_gpu_name(device_id: u32) -> Option<String> {
        // This is a small subset - you can expand this mapping
        let name = match device_id {
            0x67DF => "RX 480/470",
            0x67EF => "RX 460",
            0x687F => "Vega 56/64",
            0x6863 => "Vega Frontier Edition",
            0x731F => "RX 5700 XT",
            0x7340 => "RX 5700",
            0x73BF => "RX 6900 XT",
            0x73DF => "RX 6950 XT",
            0x73FF => "RX 6900 XT",
            0x7480 => "RX 7900 XTX",
            0x7900 => "RX 7900 XT",
            _ => return None,
        };
        Some(format!("AMD Radeon {}", name))
    }

    /// Try to read temperature from hwmon
    fn read_temperature(&self) -> Option<f32> {
        let hwmon = self.hwmon_path.as_ref()?;

        // Try different temperature input files
        for temp_file in &["temp1_input", "temp2_input", "temp3_input"] {
            let path = hwmon.join(temp_file);
            if let Ok(value) = Self::read_int_file(&path) {
                // Temperature is in millidegrees Celsius
                return Some(value as f32 / 1000.0);
            }
        }
        None
    }

    /// Try to read GPU utilization
    fn read_utilization(&self) -> Option<u32> {
        let path = self.device_path.join("gpu_busy_percent");
        Self::read_int_file(&path).ok().map(|v| v as u32)
    }

    /// Try to read VRAM usage
    fn read_memory_info(&mut self) -> Result<()> {
        // Try to read VRAM used
        let vram_used_path = self.device_path.join("mem_info_vram_used");
        if let Ok(used) = Self::read_int_file(&vram_used_path) {
            self.metrics.memory_used = Some(used as u64);
        }

        // Try to read total VRAM
        let vram_total_path = self.device_path.join("mem_info_vram_total");
        if let Ok(total) = Self::read_int_file(&vram_total_path) {
            self.metrics.memory_total = Some(total as u64);
        }

        Ok(())
    }

    /// Try to read power usage
    fn read_power_usage(&self) -> Option<f32> {
        let hwmon = self.hwmon_path.as_ref()?;

        // Try different power input files
        for power_file in &["power1_average", "power1_input"] {
            let path = hwmon.join(power_file);
            if let Ok(value) = Self::read_int_file(&path) {
                // Power is in microwatts
                return Some(value as f32 / 1_000_000.0);
            }
        }
        None
    }

    /// Try to read fan speed
    fn read_fan_speed(&self) -> Option<u32> {
        let hwmon = self.hwmon_path.as_ref()?;

        // Try to read PWM value (0-255)
        let pwm_path = hwmon.join("pwm1");
        if let Ok(pwm) = Self::read_int_file(&pwm_path) {
            // Convert 0-255 range to 0-100 percentage
            return Some((pwm as f32 / 255.0 * 100.0) as u32);
        }

        // Alternative: try fan1_input (RPM)
        let fan_path = hwmon.join("fan1_input");
        if let Ok(rpm) = Self::read_int_file(&fan_path) {
            // Can't convert RPM to percentage without max RPM, but return raw value
            // This would need calibration per GPU model
            return Some((rpm / 100).max(0).min(100) as u32);
        }

        None
    }

    /// Try to read core (graphics) clock speed in MHz
    fn read_core_clock(&self) -> Option<u32> {
        // AMD exposes current clock in pp_dpm_sclk file
        // Format: "0: 300Mhz *\n1: 1200Mhz\n" where * indicates active level
        let path = self.device_path.join("pp_dpm_sclk");
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Find the line with the asterisk (active clock level)
            for line in content.lines() {
                if line.contains('*') {
                    // Extract the clock value (e.g., "1200Mhz")
                    if let Some(clock_str) = line.split(':').nth(1) {
                        let clock_str = clock_str.trim().replace("Mhz", "").replace("*", "").trim().to_string();
                        if let Ok(clock) = clock_str.parse::<u32>() {
                            return Some(clock);
                        }
                    }
                }
            }
        }
        None
    }

    /// Try to read memory clock speed in MHz
    fn read_memory_clock(&self) -> Option<u32> {
        // AMD exposes memory clock in pp_dpm_mclk file
        let path = self.device_path.join("pp_dpm_mclk");
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Find the line with the asterisk (active clock level)
            for line in content.lines() {
                if line.contains('*') {
                    // Extract the clock value
                    if let Some(clock_str) = line.split(':').nth(1) {
                        let clock_str = clock_str.trim().replace("Mhz", "").replace("*", "").trim().to_string();
                        if let Ok(clock) = clock_str.parse::<u32>() {
                            return Some(clock);
                        }
                    }
                }
            }
        }
        None
    }
}

impl GpuBackend for AmdBackend {
    fn info(&self) -> &GpuInfo {
        &self.info
    }

    fn update(&mut self) -> Result<()> {
        // Update temperature
        self.metrics.temperature = self.read_temperature();

        // Update utilization
        self.metrics.utilization = self.read_utilization();

        // Update memory info
        let _ = self.read_memory_info();

        // Update power usage
        self.metrics.power_usage = self.read_power_usage();

        // Update fan speed
        self.metrics.fan_speed = self.read_fan_speed();

        // Update clock speeds
        self.metrics.clock_core = self.read_core_clock();
        self.metrics.clock_memory = self.read_memory_clock();

        Ok(())
    }

    fn metrics(&self) -> &GpuMetrics {
        &self.metrics
    }

    fn is_available(&self) -> bool {
        self.device_path.exists()
    }
}
