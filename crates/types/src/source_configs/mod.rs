//! Source configuration types for all data sources.

pub mod clock;
pub mod combo;
pub mod cpu;
pub mod disk;
pub mod fan_speed;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod static_text;
pub mod system_temp;
pub mod test;

// Re-export all source config types for convenience
pub use clock::{ClockSourceConfig, DateFormat, TimeFormat};
pub use combo::{ComboSourceConfig, GroupConfig, SlotConfig};
pub use cpu::{CoreSelection, CpuField, CpuSourceConfig, FrequencyUnit, TemperatureUnit};
pub use disk::{DiskField, DiskSourceConfig, DiskUnit};
pub use fan_speed::{FanCategory, FanInfo, FanSpeedConfig};
pub use gpu::{GpuField, GpuSourceConfig, MemoryUnit};
pub use memory::{MemoryField, MemorySourceConfig};
pub use network::{NetworkField, NetworkSourceConfig, NetworkSpeedUnit, NetworkTotalUnit};
pub use static_text::{StaticTextLine, StaticTextSourceConfig};
pub use system_temp::{SensorCategory, SensorInfo, SystemTempConfig};
pub use test::{TestMode, TestSourceConfig};
