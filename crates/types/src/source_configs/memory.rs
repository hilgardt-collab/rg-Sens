//! Memory source configuration types.

use serde::{Deserialize, Serialize};

use super::gpu::MemoryUnit;

/// Memory field selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MemoryField {
    Used,
    Free,
    Available,
    #[default]
    Percent,
    SwapUsed,
    SwapPercent,
}

impl MemoryField {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryField::Used => "Used",
            MemoryField::Free => "Free",
            MemoryField::Available => "Available",
            MemoryField::Percent => "Percent",
            MemoryField::SwapUsed => "Swap Used",
            MemoryField::SwapPercent => "Swap Percent",
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            0 => MemoryField::Used,
            1 => MemoryField::Free,
            2 => MemoryField::Available,
            3 => MemoryField::Percent,
            4 => MemoryField::SwapUsed,
            5 => MemoryField::SwapPercent,
            _ => MemoryField::Percent,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            MemoryField::Used => 0,
            MemoryField::Free => 1,
            MemoryField::Available => 2,
            MemoryField::Percent => 3,
            MemoryField::SwapUsed => 4,
            MemoryField::SwapPercent => 5,
        }
    }
}

fn default_update_interval() -> u64 {
    1000
}

/// Memory source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySourceConfig {
    pub field: MemoryField,
    #[serde(default)]
    pub memory_unit: MemoryUnit,
    #[serde(default)]
    pub custom_caption: Option<String>,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

impl Default for MemorySourceConfig {
    fn default() -> Self {
        Self {
            field: MemoryField::Percent,
            memory_unit: MemoryUnit::GB,
            custom_caption: None,
            update_interval_ms: default_update_interval(),
        }
    }
}
