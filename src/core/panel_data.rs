//! Unified panel data structure - single source of truth for panel configuration.
//!
//! This module re-exports types from the `rg_sens_types` crate and provides
//! convenience re-exports of individual config types.

// Re-export the main panel types from the types crate
pub use rg_sens_types::panel::{DisplayerConfig, PanelAppearance, PanelData, SourceConfig};

// Re-export source configs for convenience

// Re-export displayer configs for convenience

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PanelGeometry;
    use rg_sens_types::display_configs::BarDisplayConfig;
    use rg_sens_types::source_configs::CpuSourceConfig;

    #[test]
    fn test_source_config_serialization() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"source_type\":\"cpu\""));

        let deserialized: SourceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_type(), "cpu");
    }

    #[test]
    fn test_displayer_config_serialization() {
        let config = DisplayerConfig::Bar(BarDisplayConfig::default());
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"displayer_type\":\"bar\""));

        let deserialized: DisplayerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_panel_data_serialization() {
        let data = PanelData::with_types(
            "test-panel".to_string(),
            PanelGeometry {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
            "cpu",
            "bar",
        );

        let json = serde_json::to_string_pretty(&data).unwrap();
        let deserialized: PanelData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test-panel");
        assert_eq!(deserialized.source_type(), "cpu");
        assert_eq!(deserialized.displayer_type(), "bar");
    }

    #[test]
    fn test_to_hashmap_compatibility() {
        let config = SourceConfig::Cpu(CpuSourceConfig::default());
        let map = config.to_hashmap();

        assert!(map.contains_key("cpu_config"));
    }
}
