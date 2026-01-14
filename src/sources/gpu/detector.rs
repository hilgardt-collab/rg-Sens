//! GPU detection and backend management

use super::amd::AmdBackend;
use super::backend::{GpuBackend, GpuBackendEnum, GpuInfo};
use super::nvidia::NvidiaBackend;

/// GPU detection result
pub struct DetectedGpus {
    pub gpus: Vec<GpuBackendEnum>,
    pub info: Vec<GpuInfo>,
}

/// Detect all available GPUs and create backends
pub fn detect_gpus() -> DetectedGpus {
    let mut gpus: Vec<GpuBackendEnum> = Vec::new();
    let mut info: Vec<GpuInfo> = Vec::new();

    log::warn!("=== Detecting GPUs ===");

    // Detect NVIDIA GPUs
    detect_nvidia_gpus(&mut gpus, &mut info);

    // Detect AMD GPUs
    detect_amd_gpus(&mut gpus, &mut info);

    // Log summary
    if gpus.is_empty() {
        log::warn!("No GPUs detected");
    } else {
        log::warn!("Total GPUs detected: {}", gpus.len());
        for gpu_info in &info {
            log::info!(
                "  [{}] {} - {}",
                gpu_info.index,
                gpu_info.vendor.as_str(),
                gpu_info.name
            );
        }
    }

    DetectedGpus { gpus, info }
}

/// Detect NVIDIA GPUs using NVML
fn detect_nvidia_gpus(gpus: &mut Vec<GpuBackendEnum>, info: &mut Vec<GpuInfo>) {
    #[cfg(feature = "nvidia")]
    {
        use nvml_wrapper::Nvml;

        match Nvml::init() {
            Ok(nvml) => match nvml.device_count() {
                Ok(count) => {
                    log::info!("NVML: Found {} NVIDIA GPU(s)", count);
                    for i in 0..count {
                        match NvidiaBackend::new(i) {
                            Ok(backend) => {
                                let gpu_info = backend.info().clone();
                                log::info!("  NVIDIA GPU {}: {}", i, gpu_info.name);
                                info.push(gpu_info);
                                gpus.push(GpuBackendEnum::Nvidia(Box::new(backend)));
                            }
                            Err(e) => {
                                log::warn!("  Failed to initialize NVIDIA GPU {}: {}", i, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("NVML: Failed to get GPU count: {}", e);
                }
            },
            Err(e) => {
                log::info!("NVML: Not available ({})", e);
            }
        }
    }

    #[cfg(not(feature = "nvidia"))]
    {
        log::info!("NVML: NVIDIA support not compiled in");
    }
}

/// Detect AMD GPUs using sysfs
fn detect_amd_gpus(gpus: &mut Vec<GpuBackendEnum>, info: &mut Vec<GpuInfo>) {
    log::info!("Detecting AMD GPUs via sysfs...");

    // Try up to 16 DRM cards (card0 through card15)
    let mut amd_count = 0;
    for i in 0..16 {
        match AmdBackend::new(i) {
            Ok(backend) => {
                let gpu_info = backend.info().clone();
                log::info!("  AMD GPU {}: {}", i, gpu_info.name);
                info.push(gpu_info);
                gpus.push(GpuBackendEnum::Amd(Box::new(backend)));
                amd_count += 1;
            }
            Err(_) => {
                // Silently skip - not all card indices will be AMD GPUs
            }
        }
    }

    if amd_count == 0 {
        log::info!("  No AMD GPUs found");
    } else {
        log::info!("  Found {} AMD GPU(s)", amd_count);
    }
}

// Note: GPU name/count queries should use GpuSource::get_cached_gpu_names() and
// GpuSource::get_cached_gpu_count() which use the cached GPU_MANAGER instead of
// performing a full hardware scan.
