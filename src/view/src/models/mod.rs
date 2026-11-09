//! Data models for the CANdata application

pub mod chart;
pub mod library;

use serde::{Deserialize, Serialize};

// Re-export library types
pub use library::{ChannelDatabase, LibraryVersion, SignalLibrary};

// Re-export chart types
pub use chart::{DataPoint, Series};

/// Channel type enumeration
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
pub enum ChannelType {
    CAN,
    LIN,
}

impl Default for ChannelType {
    fn default() -> Self {
        ChannelType::CAN
    }
}

impl ChannelType {
    pub fn is_can(&self) -> bool {
        matches!(self, ChannelType::CAN)
    }

    pub fn is_lin(&self) -> bool {
        matches!(self, ChannelType::LIN)
    }

    pub fn icon(&self) -> &str {
        match self {
            ChannelType::CAN => "🟦",
            ChannelType::LIN => "🟨",
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::CAN => write!(f, "CAN"),
            ChannelType::LIN => write!(f, "LIN"),
        }
    }
}

/// Channel mapping configuration
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ChannelMapping {
    #[serde(default = "default_channel_type")]
    pub channel_type: ChannelType,
    #[serde(alias = "channel")]
    pub channel_id: u16,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub description: String,
    /// Linked signal library ID
    #[serde(default)]
    pub library_id: Option<String>,
    /// Active version name
    #[serde(default)]
    pub version_name: Option<String>,
}

fn default_channel_type() -> ChannelType {
    ChannelType::CAN
}

/// How the Plot view's signal list is laid out.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalListMode {
    /// Channels and messages that expand to show their signals.
    #[default]
    Grouped,
    /// One alphabetical list of signals, without message rows.
    Alphabetical,
}

/// Application configuration
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct AppConfig {
    /// Signal libraries
    #[serde(default)]
    pub libraries: Vec<SignalLibrary>,
    /// Channel mappings
    #[serde(default)]
    pub mappings: Vec<ChannelMapping>,
    /// Active library ID
    #[serde(default)]
    pub active_library_id: Option<String>,
    /// Active version name
    #[serde(default)]
    pub active_version_name: Option<String>,
    /// Layout of the Plot view's signal list
    #[serde(default)]
    pub signal_list_mode: SignalListMode,
}
