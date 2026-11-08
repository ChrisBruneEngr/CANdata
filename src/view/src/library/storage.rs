use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn libraries_base_path(config_file_path: Option<&Path>) -> PathBuf {
    if let Some(dir) = config_file_path.and_then(|p| p.parent()) {
        return dir.join("libraries");
    }
    // can-viewer is portable: libraries live next to the executable.
    crate::config::paths::app_dir().join("libraries")
}

pub fn build_library_file_subdir(
    library_name: &str,
    version_name: &str,
    channel_name: &str,
) -> PathBuf {
    PathBuf::from(sanitize_filename(library_name))
        .join(sanitize_filename(version_name))
        .join(sanitize_filename(channel_name))
}

pub fn copy_database_to_libraries(
    base_dir: &Path,
    library_name: &str,
    version_name: &str,
    channel_name: &str,
    source_path: &Path,
) -> Result<PathBuf> {
    let target_dir = base_dir.join(build_library_file_subdir(
        library_name,
        version_name,
        channel_name,
    ));
    fs::create_dir_all(&target_dir)
        .context(format!("Failed to create target directory: {:?}", target_dir))?;

    let extension = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("dbc");

    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_filename)
        .unwrap_or_else(|| format!("database.{}", extension));

    let dest_path = target_dir.join(file_name);

    // Saving an edited channel can point at the file already in storage.
    // Copying a file over itself could destroy it, so leave it alone.
    if let (Ok(source), Ok(dest)) = (fs::canonicalize(source_path), fs::canonicalize(&dest_path)) {
        if source == dest {
            return Ok(dest_path);
        }
    }

    fs::copy(source_path, &dest_path).context(format!(
        "Failed to copy database from {:?} to {:?}",
        source_path, dest_path
    ))?;

    Ok(dest_path)
}

pub fn delete_library_from_libraries(base_dir: &Path, library_name: &str) -> Result<()> {
    let library_dir = base_dir.join(sanitize_filename(library_name));
    if library_dir.exists() {
        fs::remove_dir_all(&library_dir)
            .context(format!("Failed to delete library directory: {:?}", library_dir))?;
    }
    Ok(())
}

pub fn delete_version_from_libraries(
    base_dir: &Path,
    library_name: &str,
    version_name: &str,
) -> Result<()> {
    let version_dir = base_dir
        .join(sanitize_filename(library_name))
        .join(sanitize_filename(version_name));
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)
            .context(format!("Failed to delete version directory: {:?}", version_dir))?;
    }
    Ok(())
}

pub fn delete_channel_from_libraries(
    base_dir: &Path,
    library_name: &str,
    version_name: &str,
    channel_name: &str,
) -> Result<()> {
    let channel_dir = base_dir.join(build_library_file_subdir(
        library_name,
        version_name,
        channel_name,
    ));
    if channel_dir.exists() {
        fs::remove_dir_all(&channel_dir)
            .context(format!("Failed to delete channel directory: {:?}", channel_dir))?;
    }

    Ok(())
}

/// 清理文件名，移除不安全的字符
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect();

    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal_name"), "normal_name");
        assert_eq!(sanitize_filename("name/with\\slashes"), "name_with_slashes");
        assert_eq!(
            sanitize_filename("name:with*special?chars"),
            "name_with_special_chars"
        );
    }

}
