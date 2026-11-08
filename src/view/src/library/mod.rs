//! Signal library management module
//!
//! 提供信号库的CRUD操作、版本管理和验证功能

mod storage;
pub mod bundle;
pub mod signal_sets;

pub use storage::{
    build_library_file_subdir, copy_database_to_libraries,
    delete_channel_from_libraries, delete_library_from_libraries,
    delete_version_from_libraries, libraries_base_path, sanitize_filename,
};

pub use signal_sets::save_signal_set_store;

use crate::models::{
    ChannelMapping, ChannelType, SignalLibrary,
};
use parser::dbc::{DbcDatabase, DbcParser};
use parser::ldf::{LdfDatabase, LdfParser};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 信号库管理器
#[derive(Clone)]
pub struct LibraryManager {
    libraries: Vec<SignalLibrary>,
}

impl LibraryManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
        }
    }

    /// 从库列表创建管理器
    pub fn from_libraries(libraries: Vec<SignalLibrary>) -> Self {
        Self { libraries }
    }

    /// 获取所有库
    pub fn libraries(&self) -> &[SignalLibrary] {
        &self.libraries
    }

    /// 根据ID查找库
    pub fn find_library(&self, id: &str) -> Option<&SignalLibrary> {
        self.libraries.iter().find(|lib| lib.id == id)
    }

    /// 根据ID查找可变库
    pub fn find_library_mut(&mut self, id: &str) -> Option<&mut SignalLibrary> {
        self.libraries.iter_mut().find(|lib| lib.id == id)
    }

    /// 创建新库
    pub fn create_library(
        &mut self,
        name: String,
        channel_type: ChannelType,
    ) -> Result<&SignalLibrary, String> {
        let id = generate_library_id(&name);

        // 检查是否已存在
        if self.find_library(&id).is_some() {
            return Err("Library already exists".to_string());
        }

        let library = SignalLibrary::new(id.clone(), name, channel_type);
        self.libraries.push(library);
        Ok(self.find_library(&id).unwrap())
    }

    /// Add an already-built library (e.g. one read from a `.cvlib` file).
    pub fn add_library(&mut self, library: SignalLibrary) -> Result<(), String> {
        if self.find_library(&library.id).is_some() {
            return Err(format!("Library '{}' already exists", library.name));
        }
        self.libraries.push(library);
        Ok(())
    }

    /// 重命名库 — 返回新生成的 library ID
    pub fn rename_library(&mut self, old_id: &str, new_name: String) -> Result<String, String> {
        let new_name = new_name.trim().to_string();
        if new_name.is_empty() {
            return Err("Library name cannot be empty".to_string());
        }

        let new_id = generate_library_id(&new_name);

        // Reject if new name collides with a different library
        if new_id != old_id {
            if self.find_library(&new_id).is_some() {
                return Err(format!("Library '{}' already exists", new_name));
            }
        }

        let library = self
            .find_library_mut(old_id)
            .ok_or("Library not found")?;

        library.id = new_id.clone();
        library.name = new_name;

        Ok(new_id)
    }

    /// 重命名库版本
    pub fn rename_version(
        &mut self,
        library_id: &str,
        old_name: &str,
        new_name: String,
    ) -> Result<(), String> {
        let new_name = new_name.trim().to_string();
        if new_name.is_empty() {
            return Err("Version name cannot be empty".to_string());
        }

        let library = self
            .find_library_mut(library_id)
            .ok_or("Library not found")?;

        // Reject duplicate version name within the same library
        if library.versions.iter().any(|v| v.name == new_name && v.name != old_name) {
            return Err(format!("Version '{}' already exists", new_name));
        }

        let version = library
            .versions
            .iter_mut()
            .find(|v| v.name == old_name)
            .ok_or("Version not found")?;

        version.name = new_name;

        Ok(())
    }

    /// 删除库
    pub fn delete_library(&mut self, id: &str, mappings: &[ChannelMapping]) -> Result<(), String> {
        let library = self.find_library(id).ok_or("Library not found")?;

        // 检查是否被使用
        if library.is_used(mappings) {
            let channels = library.used_channels(mappings);
            return Err(format!("Library is in use by channels: {:?}", channels));
        }

        let pos = self.libraries.iter().position(|lib| lib.id == id).unwrap();
        self.libraries.remove(pos);
        Ok(())
    }

    /// 删除版本
    pub fn remove_version(
        &mut self,
        library_id: &str,
        version_name: &str,
        mappings: &[ChannelMapping],
    ) -> Result<(), String> {
        let library = self
            .find_library_mut(library_id)
            .ok_or("Library not found")?;

        // 检查版本是否被使用
        if mappings.iter().any(|m| {
            m.library_id.as_ref().map(|s| s.as_str()) == Some(library_id)
                && m.version_name.as_ref().map(|s| s.as_str()) == Some(version_name)
        }) {
            return Err("Version is currently in use".to_string());
        }

        if !library.remove_version(version_name) {
            return Err("Version not found".to_string());
        }

        Ok(())
    }

    /// 加载数据库文件
    ///
    /// Channel type is auto-detected from the file extension (.dbc → CAN,
    /// .ldf → LIN) so a library created with the wrong channel_type still
    /// loads its database correctly. The passed `channel_type` is used only
    /// as a fallback when the extension is ambiguous.
    pub fn load_database(&self, path: &str, channel_type: ChannelType) -> Result<Database, String> {
        if is_remote_path(path) {
            return Err(remote_path_error(path));
        }
        let detected = detect_channel_type_from_path(path);
        let effective = detected.unwrap_or(channel_type);
        match effective {
            ChannelType::CAN => self.load_dbc(path),
            ChannelType::LIN => self.load_ldf(path),
        }
    }

    /// 加载DBC文件
    fn load_dbc(&self, path: &str) -> Result<Database, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let parser = DbcParser::new();
        let db = parser
            .parse(&content)
            .map_err(|e| format!("DBC parse error: {}", e))?;

        Ok(Database::Dbc(db))
    }

    /// 加载LDF文件
    fn load_ldf(&self, path: &str) -> Result<Database, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let parser = LdfParser::new();
        let db = parser
            .parse(&content)
            .map_err(|e| format!("LDF parse error: {}", e))?;

        Ok(Database::Ldf(db))
    }
}

/// 数据库枚举（包装DBC和LDF）
pub enum Database {
    Dbc(DbcDatabase),
    Ldf(LdfDatabase),
}

/// True when a database path is an http(s) URL.
///
/// CANdata only reads databases from local files; remote paths (left over
/// from the removed LAN sharing feature) are treated as errors.
pub fn is_remote_path(path: &str) -> bool {
    let lowered = path.trim().to_ascii_lowercase();
    lowered.starts_with("http://") || lowered.starts_with("https://")
}

/// Error message for a remote (http/https) database path.
pub fn remote_path_error(path: &str) -> String {
    format!(
        "Remote database paths are not supported: {}. Re-add the database from a local file in the Library view.",
        path.trim()
    )
}

/// Detect `ChannelType` from a database file's extension.
/// Returns `Some(CAN)` for `.dbc`, `Some(LIN)` for `.ldf`, `None` for unknown.
/// Used by `load_database` to override a library's `channel_type` when the
/// caller passed a stale value (e.g. a LIN library created via the legacy
/// hardcoded-CAN path).
pub fn detect_channel_type_from_path(path: &str) -> Option<ChannelType> {
    let lowered = path.to_lowercase();
    if lowered.ends_with(".ldf") {
        Some(ChannelType::LIN)
    } else if lowered.ends_with(".dbc") {
        Some(ChannelType::CAN)
    } else {
        None
    }
}

/// 生成唯一的库ID
pub fn generate_library_id(name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    format!("lib_{:x}", hasher.finish())
}

impl Default for LibraryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_library_id() {
        let id1 = generate_library_id("test");
        let id2 = generate_library_id("test");
        let id3 = generate_library_id("other");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_create_library() {
        let mut manager = LibraryManager::new();
        let result = manager.create_library("Test Library".to_string(), ChannelType::CAN);

        assert!(result.is_ok());
        assert_eq!(manager.libraries().len(), 1);
    }

    #[test]
    fn test_load_database_auto_detects_ldf_from_extension() {
        // Write a minimal LDF to a temp file with .ldf extension
        let ldf_content = r#"
LIN_description_file = "2.1";
Signals {
    SysSt: 8, 0, BCM, IPC;
}
Frames {
    BCM_St: 0x10, BCM, 2 {
        SysSt, 0;
    }
}
"#;
        let dir = tempfile::tempdir().unwrap();
        let ldf_path = dir.path().join("test.ldf");
        std::fs::write(&ldf_path, ldf_content).unwrap();

        let manager = LibraryManager::new();
        // Caller says CAN, but file extension is .ldf — must still load as LDF.
        let result = manager.load_database(ldf_path.to_str().unwrap(), ChannelType::CAN);
        assert!(result.is_ok(), "load_database failed: {:?}", result.err());
        assert!(
            matches!(result.unwrap(), Database::Ldf(_)),
            "expected Ldf variant when path ends with .ldf"
        );
    }

    #[test]
    fn test_load_database_auto_detects_dbc_from_extension() {
        let dbc_content = r#"
VERSION ""

NS_ :

BS_:

BU_: DBG

BO_ 256 EngineStatus: 8 DBG
 SG_ EngineRPM : 0|16@1+ (1,0) [0|65535] "rpm" DBG
"#;
        let dir = tempfile::tempdir().unwrap();
        let dbc_path = dir.path().join("test.dbc");
        std::fs::write(&dbc_path, dbc_content).unwrap();

        let manager = LibraryManager::new();
        // Caller says LIN, but file extension is .dbc — must still load as DBC.
        let result = manager.load_database(dbc_path.to_str().unwrap(), ChannelType::LIN);
        assert!(result.is_ok(), "load_database failed: {:?}", result.err());
        assert!(
            matches!(result.unwrap(), Database::Dbc(_)),
            "expected Dbc variant when path ends with .dbc"
        );
    }
}
