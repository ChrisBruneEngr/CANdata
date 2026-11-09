//! Signal library data models
//!
//! Signal library data structures: libraries, versions and channel mappings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database file type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    DBC,
    LDF,
}

impl DatabaseType {
    pub fn extension(&self) -> &str {
        match self {
            DatabaseType::DBC => "dbc",
            DatabaseType::LDF => "ldf",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            DatabaseType::DBC => "📋",
            DatabaseType::LDF => "🔗",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "dbc" => Some(DatabaseType::DBC),
            "ldf" => Some(DatabaseType::LDF),
            _ => None,
        }
    }
}

/// Per-channel database configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelDatabase {
    /// Channel type (CAN/LIN)
    #[serde(default)]
    pub channel_type: crate::models::ChannelType,
    /// Channel ID
    pub channel_id: u16,
    /// Channel name
    pub channel_name: String,
    /// Database file path
    pub database_path: String,
}

impl ChannelDatabase {
    /// Create a new channel database configuration.
    pub fn new(
        channel_type: crate::models::ChannelType,
        channel_id: u16,
        channel_name: String,
        database_path: String,
    ) -> Self {
        Self {
            channel_type,
            channel_id,
            channel_name,
            database_path,
        }
    }

    /// Validate the channel configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Check that the channel ID is valid (1-255)
        if self.channel_id == 0 || self.channel_id > 255 {
            return Err(format!(
                "Invalid channel ID: {}. Must be between 1 and 255",
                self.channel_id
            ));
        }

        // Check that the channel name is not empty
        if self.channel_name.trim().is_empty() {
            return Err("Channel name cannot be empty".to_string());
        }

        // Check that the database file path is not empty
        if self.database_path.trim().is_empty() {
            return Err("Database path cannot be empty".to_string());
        }

        Ok(())
    }

    /// Get the database type.
    pub fn database_type(&self) -> Option<DatabaseType> {
        std::path::Path::new(&self.database_path)
            .extension()
            .and_then(|e| e.to_str())
            .and_then(|ext| DatabaseType::from_extension(ext))
    }
}

/// Signal library version
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryVersion {
    /// Version name/number (e.g. "v1.0", "v2.1")
    pub name: String,
    /// Default database file path (backward compatibility)
    pub path: String,
    /// Creation date
    pub date: String,
    /// Version description
    #[serde(default)]
    pub description: String,
    /// Database file configuration for each channel.
    /// Stored grouped by channel type.
    #[serde(default)]
    pub channel_databases: Vec<ChannelDatabase>,
}

impl LibraryVersion {
    /// Create a new version.
    pub fn new(name: String, path: String, date: String) -> Self {
        Self {
            name,
            path,
            date,
            description: String::new(),
            channel_databases: Vec::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Add a channel database configuration.
    pub fn add_channel_database(&mut self, channel_db: ChannelDatabase) -> Result<(), String> {
        // Validate the channel configuration
        channel_db.validate()?;

        // Check whether the channel ID already exists
        if self
            .channel_databases
            .iter()
            .any(|db| db.channel_id == channel_db.channel_id)
        {
            return Err(format!(
                "Channel ID {} already exists in this version",
                channel_db.channel_id
            ));
        }

        self.channel_databases.push(channel_db);
        Ok(())
    }

    /// Get the database configuration for a channel.
    pub fn get_channel_database(&self, channel_id: u16) -> Option<&ChannelDatabase> {
        self.channel_databases
            .iter()
            .find(|db| db.channel_id == channel_id)
    }

    /// Get all CAN channels.
    pub fn get_can_channels(&self) -> Vec<&ChannelDatabase> {
        self.channel_databases
            .iter()
            .filter(|db| db.database_type() == Some(DatabaseType::DBC))
            .collect()
    }

    /// Get all LIN channels.
    pub fn get_lin_channels(&self) -> Vec<&ChannelDatabase> {
        self.channel_databases
            .iter()
            .filter(|db| db.database_type() == Some(DatabaseType::LDF))
            .collect()
    }

    /// Get the channel database list (for backward compatibility).
    pub fn get_channel_map(&self) -> HashMap<u16, String> {
        let mut map = HashMap::new();
        for db in &self.channel_databases {
            map.insert(db.channel_id, db.database_path.clone());
        }
        // If no channel databases are configured, use the default path
        if map.is_empty() && !self.path.is_empty() {
            // Try to infer the channel ID
            if let Some(1) = Some(1) {
                map.insert(1, self.path.clone());
            }
        }
        map
    }

    /// Check whether a channel ID is already in use.
    pub fn is_channel_id_used(&self, channel_id: u16) -> bool {
        self.channel_databases
            .iter()
            .any(|db| db.channel_id == channel_id)
    }

    /// Get the list of channel IDs in use.
    pub fn get_used_channel_ids(&self) -> Vec<u16> {
        self.channel_databases
            .iter()
            .map(|db| db.channel_id)
            .collect()
    }

    /// Get version statistics.
    pub fn get_stats(&self) -> VersionStats {
        let can_count = self.get_can_channels().len();
        let lin_count = self.get_lin_channels().len();

        VersionStats {
            total_channels: self.channel_databases.len(),
            can_channels: can_count,
            lin_channels: lin_count,
        }
    }
}

/// Version statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionStats {
    pub total_channels: usize,
    pub can_channels: usize,
    pub lin_channels: usize,
}

/// Signal library
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalLibrary {
    /// Unique library identifier
    pub id: String,
    /// Library name
    pub name: String,
    /// Library type (CAN/LIN)
    #[serde(default = "default_channel_type")]
    pub channel_type: super::ChannelType,
    /// Versions, sorted by version number descending (newest first)
    pub versions: Vec<LibraryVersion>,
}

fn default_channel_type() -> super::ChannelType {
    super::ChannelType::CAN
}

impl SignalLibrary {
    /// Create a new library.
    pub fn new(id: String, name: String, channel_type: super::ChannelType) -> Self {
        Self {
            id,
            name,
            channel_type,
            versions: Vec::new(),
        }
    }

    /// Get the latest version.
    pub fn latest_version(&self) -> Option<&LibraryVersion> {
        self.versions.first()
    }

    /// Get the version with the given name.
    pub fn get_version(&self, name: &str) -> Option<&LibraryVersion> {
        self.versions.iter().find(|v| v.name == name)
    }

    /// Get a mutable reference to a version.
    pub fn get_version_mut(&mut self, name: &str) -> Option<&mut LibraryVersion> {
        self.versions.iter_mut().find(|v| v.name == name)
    }

    /// Get the database type.
    pub fn database_type(&self) -> DatabaseType {
        match self.channel_type {
            super::ChannelType::CAN => DatabaseType::DBC,
            super::ChannelType::LIN => DatabaseType::LDF,
        }
    }

    /// Add a new version.
    pub fn add_version(&mut self, version: LibraryVersion) {
        // Check whether a version with the same name already exists
        if !self.versions.iter().any(|v| v.name == version.name) {
            self.versions.push(version);
            self.sort_versions();
        }
    }

    /// Remove a version.
    pub fn remove_version(&mut self, name: &str) -> bool {
        if let Some(pos) = self.versions.iter().position(|v| v.name == name) {
            self.versions.remove(pos);
            true
        } else {
            false
        }
    }

    /// Sort versions (by version number, descending).
    pub fn sort_versions(&mut self) {
        self.versions.sort_by(|a, b| {
            let v_a = extract_version_number(&a.name);
            let v_b = extract_version_number(&b.name);
            v_b.partial_cmp(&v_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Check whether the library is in use.
    pub fn is_used(&self, mappings: &[super::ChannelMapping]) -> bool {
        mappings
            .iter()
            .any(|m| m.library_id.as_ref() == Some(&self.id))
    }

    /// Get the channels that use this library.
    pub fn used_channels(&self, mappings: &[super::ChannelMapping]) -> Vec<u16> {
        mappings
            .iter()
            .filter(|m| m.library_id.as_ref() == Some(&self.id))
            .map(|m| m.channel_id)
            .collect()
    }

    /// Get the name of the active version.
    pub fn active_version_name(&self, mappings: &[super::ChannelMapping]) -> Option<String> {
        mappings
            .iter()
            .filter(|m| m.library_id.as_ref() == Some(&self.id))
            .find_map(|m| m.version_name.clone())
    }
}

/// Extract the numbers from a version string for sorting.
fn extract_version_number(version: &str) -> f64 {
    let cleaned = version
        .trim_start_matches('v')
        .trim_start_matches('V')
        .replace('_', ".");

    let parts: Vec<&str> = cleaned.split('.').collect();
    if parts.is_empty() {
        return 0.0;
    }

    let major = parts[0].parse::<f64>().unwrap_or(0.0);
    let minor = if parts.len() > 1 {
        parts[1].parse::<f64>().unwrap_or(0.0) / 100.0
    } else {
        0.0
    };

    major + minor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_number() {
        assert_eq!(extract_version_number("v1.0"), 1.0);
        assert_eq!(extract_version_number("v2.1"), 2.01);
        assert_eq!(extract_version_number("v10.5"), 10.05);
        assert_eq!(extract_version_number("1.0"), 1.0);
        assert_eq!(extract_version_number("invalid"), 0.0);
    }

    #[test]
    fn test_library_version_sorting() {
        let mut lib = SignalLibrary::new(
            "test".to_string(),
            "Test Library".to_string(),
            crate::models::ChannelType::CAN,
        );

        lib.add_version(LibraryVersion::new(
            "v1.0".to_string(),
            "/path1".to_string(),
            "2024-01-01".to_string(),
        ));
        lib.add_version(LibraryVersion::new(
            "v2.0".to_string(),
            "/path2".to_string(),
            "2024-01-02".to_string(),
        ));
        lib.add_version(LibraryVersion::new(
            "v1.5".to_string(),
            "/path3".to_string(),
            "2024-01-03".to_string(),
        ));

        // Should be sorted: v2.0, v1.5, v1.0
        assert_eq!(lib.versions[0].name, "v2.0");
        assert_eq!(lib.versions[1].name, "v1.5");
        assert_eq!(lib.versions[2].name, "v1.0");
    }

    #[test]
    fn test_database_type() {
        assert_eq!(DatabaseType::from_extension("dbc"), Some(DatabaseType::DBC));
        assert_eq!(DatabaseType::from_extension("ldf"), Some(DatabaseType::LDF));
        assert_eq!(DatabaseType::from_extension("xyz"), None);
    }
}
