//! Where CANdata keeps its files.
//!
//! CANdata is portable: everything it saves lives in the folder that holds
//! the executable, never in the current working directory or a per-user
//! profile folder. Copying that folder copies the whole setup.
//!
//! ```text
//! <exe folder>/
//! ├── candata.exe
//! ├── multi_channel_config.json   channel mappings + library definitions
//! ├── signal_sets.json            saved plot signal sets
//! └── libraries/                  copies of each library's DBC/LDF files
//!     └── <library>/<version>/<channel>/<file>
//! ```

use std::path::PathBuf;

/// File name of the main configuration file.
pub const CONFIG_FILE_NAME: &str = "multi_channel_config.json";

/// The folder that holds the running executable.
///
/// Falls back to `.` only if the OS cannot report the executable's path.
pub fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Path of the main configuration file, next to the executable.
pub fn config_file() -> PathBuf {
    app_dir().join(CONFIG_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_is_next_to_executable() {
        let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        assert_eq!(app_dir(), exe_dir);
        assert_eq!(config_file(), exe_dir.join(CONFIG_FILE_NAME));
    }
}
