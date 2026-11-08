//! Library export / import files (`.cvlib`).
//!
//! A `.cvlib` file is a single JSON document that carries one or more signal
//! libraries together with the contents of every database file they use, so a
//! library can be moved to another machine by copying one file.
//!
//! ```json
//! {
//!   "format": "can-viewer-library",
//!   "format_version": 1,
//!   "exported_at": "2026-11-08 14:03:12",
//!   "libraries": [
//!     {
//!       "name": "Powertrain",
//!       "channel_type": "CAN",
//!       "versions": [
//!         {
//!           "name": "v1.0", "date": "2026-10-01", "description": "",
//!           "default_file": null,
//!           "channels": [
//!             { "channel_type": "CAN", "channel_id": 1, "channel_name": "PT",
//!               "file": { "file_name": "pt.dbc", "content_base64": "..." } }
//!           ]
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! File contents are base64-encoded so DBC/LDF files in any text encoding
//! survive the round trip byte for byte.

use super::storage::{build_library_file_subdir, sanitize_filename};
use super::{generate_library_id, is_remote_path};
use crate::models::{ChannelDatabase, ChannelType, LibraryVersion, SignalLibrary};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File extension for library export files.
pub const BUNDLE_EXTENSION: &str = "cvlib";

const BUNDLE_FORMAT: &str = "can-viewer-library";
const BUNDLE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct LibraryBundle {
    format: String,
    format_version: u32,
    #[serde(default)]
    exported_at: String,
    libraries: Vec<BundledLibrary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundledLibrary {
    name: String,
    channel_type: ChannelType,
    versions: Vec<BundledVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundledVersion {
    name: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    description: String,
    /// The version's single database file, for versions that have no
    /// per-channel databases (older libraries).
    #[serde(default)]
    default_file: Option<BundledFile>,
    #[serde(default)]
    channels: Vec<BundledChannel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundledChannel {
    channel_type: ChannelType,
    channel_id: u16,
    channel_name: String,
    file: BundledFile,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundledFile {
    file_name: String,
    content_base64: String,
}

impl BundledFile {
    fn read(path: &str, context: &str) -> Result<Self, String> {
        if is_remote_path(path) {
            return Err(format!(
                "{} uses a remote database path ({}), which is not supported",
                context, path
            ));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Cannot read database for {} ('{}'): {}", context, path, e))?;
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("database.dbc")
            .to_string();
        Ok(Self {
            file_name,
            content_base64: BASE64.encode(bytes),
        })
    }

    fn decode(&self, context: &str) -> Result<Vec<u8>, String> {
        BASE64
            .decode(self.content_base64.as_bytes())
            .map_err(|e| format!("Database for {} is corrupted: {}", context, e))
    }
}

/// Write `library` and the contents of all its database files to `dest`.
pub fn export_library(library: &SignalLibrary, dest: &Path) -> Result<(), String> {
    let mut versions = Vec::with_capacity(library.versions.len());
    for version in &library.versions {
        let mut channels = Vec::with_capacity(version.channel_databases.len());
        for channel in &version.channel_databases {
            let context = format!(
                "version '{}', channel {}",
                version.name, channel.channel_id
            );
            channels.push(BundledChannel {
                channel_type: channel.channel_type,
                channel_id: channel.channel_id,
                channel_name: channel.channel_name.clone(),
                file: BundledFile::read(&channel.database_path, &context)?,
            });
        }
        let default_file = if channels.is_empty() && !version.path.trim().is_empty() {
            Some(BundledFile::read(
                &version.path,
                &format!("version '{}'", version.name),
            )?)
        } else {
            None
        };
        versions.push(BundledVersion {
            name: version.name.clone(),
            date: version.date.clone(),
            description: version.description.clone(),
            default_file,
            channels,
        });
    }

    let bundle = LibraryBundle {
        format: BUNDLE_FORMAT.to_string(),
        format_version: BUNDLE_FORMAT_VERSION,
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        libraries: vec![BundledLibrary {
            name: library.name.clone(),
            channel_type: library.channel_type,
            versions,
        }],
    };

    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("Failed to build export file: {}", e))?;
    std::fs::write(dest, json)
        .map_err(|e| format!("Failed to write '{}': {}", dest.display(), e))
}

/// Read a `.cvlib` file, copy its database files into `libraries_dir`, and
/// return the libraries ready to add to the library manager.
///
/// A library whose name is already taken (by `existing` or an earlier library
/// in the same file) is renamed `name_1`, `name_2`, ...
pub fn import_libraries(
    src: &Path,
    existing: &[SignalLibrary],
    libraries_dir: &Path,
) -> Result<Vec<SignalLibrary>, String> {
    let content = std::fs::read_to_string(src)
        .map_err(|e| format!("Cannot read '{}': {}", src.display(), e))?;
    let bundle: LibraryBundle = serde_json::from_str(&content)
        .map_err(|e| format!("'{}' is not a can-viewer library file: {}", src.display(), e))?;

    if bundle.format != BUNDLE_FORMAT {
        return Err(format!(
            "'{}' is not a can-viewer library file",
            src.display()
        ));
    }
    if bundle.format_version > BUNDLE_FORMAT_VERSION {
        return Err(format!(
            "'{}' was made by a newer can-viewer (format {}); this version reads format {}",
            src.display(),
            bundle.format_version,
            BUNDLE_FORMAT_VERSION
        ));
    }
    if bundle.libraries.is_empty() {
        return Err(format!("'{}' contains no libraries", src.display()));
    }

    // Decode everything before writing any files, so a corrupted bundle
    // leaves nothing half-imported on disk.
    let mut planned: Vec<(SignalLibrary, Vec<(PathBuf, Vec<u8>)>)> = Vec::new();
    for bundled in bundle.libraries {
        let taken: Vec<&str> = existing
            .iter()
            .map(|l| l.name.as_str())
            .chain(planned.iter().map(|(l, _)| l.name.as_str()))
            .collect();
        let name = unique_name(&bundled.name, &taken);
        let mut library =
            SignalLibrary::new(generate_library_id(&name), name.clone(), bundled.channel_type);
        let mut files = Vec::new();

        for bv in bundled.versions {
            let mut channel_databases = Vec::with_capacity(bv.channels.len());
            for bc in bv.channels {
                let context = format!("version '{}', channel {}", bv.name, bc.channel_id);
                let channel_dir = if bc.channel_name.trim().is_empty() {
                    format!("channel_{}", bc.channel_id)
                } else {
                    bc.channel_name.clone()
                };
                let dest = libraries_dir
                    .join(build_library_file_subdir(&name, &bv.name, &channel_dir))
                    .join(sanitize_filename(&bc.file.file_name));
                files.push((dest.clone(), bc.file.decode(&context)?));
                channel_databases.push(ChannelDatabase::new(
                    bc.channel_type,
                    bc.channel_id,
                    bc.channel_name,
                    dest.to_string_lossy().to_string(),
                ));
            }

            let path = if let Some(first) = channel_databases.first() {
                first.database_path.clone()
            } else if let Some(file) = &bv.default_file {
                let dest = libraries_dir
                    .join(build_library_file_subdir(&name, &bv.name, "default"))
                    .join(sanitize_filename(&file.file_name));
                files.push((dest.clone(), file.decode(&format!("version '{}'", bv.name))?));
                dest.to_string_lossy().to_string()
            } else {
                String::new()
            };

            let mut version = LibraryVersion::new(bv.name, path, bv.date)
                .with_description(bv.description);
            version.channel_databases = channel_databases;
            library.versions.push(version);
        }
        planned.push((library, files));
    }

    let mut imported = Vec::with_capacity(planned.len());
    for (library, files) in planned {
        for (dest, bytes) in files {
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    format!("Failed to create folder '{}': {}", dir.display(), e)
                })?;
            }
            std::fs::write(&dest, bytes)
                .map_err(|e| format!("Failed to write '{}': {}", dest.display(), e))?;
        }
        imported.push(library);
    }
    Ok(imported)
}

/// `name` if it is free, otherwise the first free `name_N`.
fn unique_name(name: &str, taken: &[&str]) -> String {
    if !taken.contains(&name) {
        return name.to_string();
    }
    (1..)
        .map(|n| format!("{}_{}", name, n))
        .find(|candidate| !taken.contains(&candidate.as_str()))
        .expect("an unused name always exists")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_library(dir: &Path) -> SignalLibrary {
        let dbc = dir.join("pt.dbc");
        // Non-UTF-8 byte (0xB0, degree sign in Windows-1252) must survive.
        std::fs::write(&dbc, b"VERSION \"1\"\nCM_ \"25\xB0C\";\n").unwrap();
        let mut lib = SignalLibrary::new("lib_x".into(), "Powertrain".into(), ChannelType::CAN);
        let mut v = LibraryVersion::new(
            "v1.0".into(),
            dbc.to_string_lossy().to_string(),
            "2026-10-01".into(),
        )
        .with_description("first".into());
        v.channel_databases = vec![ChannelDatabase::new(
            ChannelType::CAN,
            1,
            "PT".into(),
            dbc.to_string_lossy().to_string(),
        )];
        lib.versions.push(v);
        lib
    }

    #[test]
    fn export_then_import_round_trips_library_and_file_bytes() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let lib = sample_library(src_dir.path());
        let bundle_path = src_dir.path().join("pt.cvlib");

        export_library(&lib, &bundle_path).unwrap();
        let imported = import_libraries(&bundle_path, &[], dest_dir.path()).unwrap();

        assert_eq!(imported.len(), 1);
        let got = &imported[0];
        assert_eq!(got.name, "Powertrain");
        assert_eq!(got.versions.len(), 1);
        let v = &got.versions[0];
        assert_eq!(v.name, "v1.0");
        assert_eq!(v.description, "first");
        assert_eq!(v.channel_databases.len(), 1);
        let ch = &v.channel_databases[0];
        assert_eq!(ch.channel_id, 1);
        assert_eq!(ch.channel_name, "PT");
        let new_path = PathBuf::from(&ch.database_path);
        assert!(new_path.starts_with(dest_dir.path()));
        assert!(new_path.ends_with(Path::new("Powertrain/v1.0/PT/pt.dbc")));
        assert_eq!(
            std::fs::read(&new_path).unwrap(),
            std::fs::read(src_dir.path().join("pt.dbc")).unwrap()
        );
        assert_eq!(v.path, ch.database_path);
    }

    #[test]
    fn import_renames_library_when_name_is_taken() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let lib = sample_library(src_dir.path());
        let bundle_path = src_dir.path().join("pt.cvlib");
        export_library(&lib, &bundle_path).unwrap();

        let existing = vec![lib];
        let imported = import_libraries(&bundle_path, &existing, dest_dir.path()).unwrap();
        assert_eq!(imported[0].name, "Powertrain_1");
        assert_ne!(imported[0].id, existing[0].id);
    }

    #[test]
    fn export_rejects_remote_database_paths() {
        let dir = TempDir::new().unwrap();
        let mut lib = sample_library(dir.path());
        lib.versions[0].channel_databases[0].database_path =
            "http://192.168.1.10:8080/files/pt.dbc".into();
        let err = export_library(&lib, &dir.path().join("x.cvlib")).unwrap_err();
        assert!(err.contains("remote database path"), "{}", err);
    }

    #[test]
    fn import_rejects_files_that_are_not_bundles() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("other.cvlib");
        std::fs::write(&path, r#"{"format":"something-else","format_version":1,"libraries":[]}"#)
            .unwrap();
        assert!(import_libraries(&path, &[], dir.path()).is_err());
        std::fs::write(&path, "not json").unwrap();
        assert!(import_libraries(&path, &[], dir.path()).is_err());
    }

    #[test]
    fn unique_name_appends_first_free_suffix() {
        assert_eq!(unique_name("A", &[]), "A");
        assert_eq!(unique_name("A", &["A"]), "A_1");
        assert_eq!(unique_name("A", &["A", "A_1"]), "A_2");
    }
}
