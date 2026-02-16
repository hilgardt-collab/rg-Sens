//! Data field metadata for describing what data sources provide

use serde::{Deserialize, Serialize};

/// Type of data a field contains
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// Text data (e.g., "CPU", "Temperature")
    Text,
    /// Numerical data (e.g., 45.2, 3.4)
    Numerical,
    /// Percentage (0.0 to 100.0)
    Percentage,
    /// Boolean value
    Boolean,
}

/// Purpose/role of a field in the data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldPurpose {
    /// Caption or label (e.g., "CPU")
    Caption,
    /// Primary value (e.g., temperature, usage percentage)
    Value,
    /// Unit of measurement (e.g., "°C", "%", "GHz")
    Unit,
    /// Additional/secondary value
    SecondaryValue,
    /// Status or state information
    Status,
    /// Generic/other purpose
    Other,
}

/// Metadata describing a single data field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
    /// Unique identifier for this field
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this field represents
    pub description: String,
    /// Type of data this field contains
    pub field_type: FieldType,
    /// Purpose/role of this field
    pub purpose: FieldPurpose,
}

impl FieldMetadata {
    /// Create a new field metadata
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        field_type: FieldType,
        purpose: FieldPurpose,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            field_type,
            purpose,
        }
    }
}
