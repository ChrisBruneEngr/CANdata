//! Math signals — user-defined signals computed from an equation of decoded
//! signals, e.g. `Motor_Power = Motor_Speed * Motor_Torque`.
//!
//! Math signals are global: they are not tied to a library or version, and
//! live in `math_signals.json` next to `multi_channel_config.json`. Only the
//! equation text is stored; it is checked again every time it is used.
//! See `docs/design/2026-11-15-math-signals-design.md`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File name of the Math signal store.
pub const MATH_SIGNALS_FILE_NAME: &str = "math_signals.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathSignal {
    pub name: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub equation: String,
    #[serde(default)]
    pub time_mode: TimeMode,
}

impl MathSignal {
    /// A new, empty Math signal.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            unit: String::new(),
            equation: String::new(),
            time_mode: TimeMode::default(),
        }
    }
}

/// How the inputs of an equation are lined up in time.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum TimeMode {
    /// A point whenever any input updates; each input holds its last value.
    #[default]
    SampleAndHold,
    /// Points on a fixed grid of `rate_hz`.
    Resample {
        rate_hz: f64,
        #[serde(default)]
        interpolation: Interpolation,
    },
}

/// How an input is read between its samples in resample mode.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Interpolation {
    #[default]
    Hold,
    Linear,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MathSignalStore {
    #[serde(default)]
    pub signals: Vec<MathSignal>,
}

impl MathSignalStore {
    /// Position of the Math signal with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        self.signals
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// A name for a new Math signal that is not used yet: `Math_1`, `Math_2`, ...
    pub fn next_free_name(&self) -> String {
        (1..)
            .map(|n| format!("Math_{}", n))
            .find(|name| self.position(name).is_none())
            .expect("an unused name always exists")
    }

    /// Check `name` for a Math signal. `editing` is the position of the signal
    /// being renamed, so it doesn't clash with itself.
    pub fn validate_name(&self, name: &str, editing: Option<usize>) -> Result<(), String> {
        if name.is_empty() {
            return Err("Name is empty".into());
        }
        if !is_identifier(name) {
            return Err(
                "Name must start with a letter or _ and use only letters, digits and _".into(),
            );
        }
        match self.position(name) {
            Some(i) if Some(i) != editing => {
                Err(format!("A Math signal named '{}' already exists", self.signals[i].name))
            }
            _ => Ok(()),
        }
    }
}

/// True for names like `Motor_Speed`: a letter or `_`, then letters, digits or `_`.
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Path of `math_signals.json`, next to `multi_channel_config.json`.
pub fn math_signal_store_path(config_file_path: Option<&Path>) -> PathBuf {
    if let Some(dir) = config_file_path.and_then(|p| p.parent()) {
        return dir.join(MATH_SIGNALS_FILE_NAME);
    }
    crate::config::paths::app_dir().join(MATH_SIGNALS_FILE_NAME)
}

/// Load the store. A missing file gives an empty store. A file that can't be
/// read as Math signals is renamed to `math_signals.json.bad` first, so the
/// next save can't overwrite the user's equations.
pub fn load_math_signal_store(config_file_path: Option<&Path>) -> MathSignalStore {
    load_from(&math_signal_store_path(config_file_path))
}

fn load_from(path: &Path) -> MathSignalStore {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return MathSignalStore::default(),
        Err(e) => {
            eprintln!("⚠️  Failed to read {}: {}", path.display(), e);
            return MathSignalStore::default();
        }
    };
    match serde_json::from_str(&content) {
        Ok(store) => store,
        Err(e) => {
            let backup = path.with_extension("json.bad");
            eprintln!(
                "⚠️  {} is not valid ({}); moved it to {}",
                path.display(),
                e,
                backup.display()
            );
            let _ = std::fs::rename(path, &backup);
            MathSignalStore::default()
        }
    }
}

/// Save the store.
pub fn save_math_signal_store(
    store: &MathSignalStore,
    config_file_path: Option<&Path>,
) -> Result<(), String> {
    save_to(store, &math_signal_store_path(config_file_path))
}

fn save_to(store: &MathSignalStore, path: &Path) -> Result<(), String> {
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize Math signals: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(names: &[&str]) -> MathSignalStore {
        MathSignalStore {
            signals: names.iter().map(|n| MathSignal::new(*n)).collect(),
        }
    }

    #[test]
    fn identifiers() {
        for ok in ["a", "_", "Motor_Speed", "x2", "_tmp_3"] {
            assert!(is_identifier(ok), "{ok}");
        }
        for bad in ["", "2x", "a b", "a-b", "a.b", "Motor*", "é"] {
            assert!(!is_identifier(bad), "{bad}");
        }
    }

    #[test]
    fn validate_name_rules() {
        let store = store_with(&["Motor_Power", "Slip"]);
        assert!(store.validate_name("Wheel_Speed", None).is_ok());
        assert!(store.validate_name("", None).is_err());
        assert!(store.validate_name("1abc", None).is_err());
        // Clashes ignore case...
        assert!(store.validate_name("motor_power", None).is_err());
        // ...but a signal may keep (or re-case) its own name.
        assert!(store.validate_name("MOTOR_POWER", Some(0)).is_ok());
        assert!(store.validate_name("Slip", Some(0)).is_err());
    }

    #[test]
    fn next_free_name_skips_used() {
        assert_eq!(store_with(&[]).next_free_name(), "Math_1");
        assert_eq!(store_with(&["Math_1", "math_2"]).next_free_name(), "Math_3");
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(MATH_SIGNALS_FILE_NAME);
        let store = MathSignalStore {
            signals: vec![
                MathSignal {
                    name: "Motor_Power".into(),
                    unit: "kW".into(),
                    equation: "Motor_Speed * Motor_Torque / 9549".into(),
                    time_mode: TimeMode::SampleAndHold,
                },
                MathSignal {
                    name: "Slip".into(),
                    unit: "%".into(),
                    equation: "(A - B) / B * 100".into(),
                    time_mode: TimeMode::Resample {
                        rate_hz: 100.0,
                        interpolation: Interpolation::Linear,
                    },
                },
            ],
        };
        save_to(&store, &path).unwrap();
        assert_eq!(load_from(&path), store);
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_from(&dir.path().join("none.json")), MathSignalStore::default());
    }

    #[test]
    fn missing_fields_take_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(MATH_SIGNALS_FILE_NAME);
        std::fs::write(
            &path,
            r#"{"signals":[{"name":"A"},{"name":"B","time_mode":{"mode":"Resample","rate_hz":10.0}}]}"#,
        )
        .unwrap();
        let store = load_from(&path);
        assert_eq!(store.signals[0], MathSignal::new("A"));
        assert_eq!(
            store.signals[1].time_mode,
            TimeMode::Resample { rate_hz: 10.0, interpolation: Interpolation::Hold }
        );
    }

    #[test]
    fn bad_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(MATH_SIGNALS_FILE_NAME);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load_from(&path), MathSignalStore::default());
        assert!(!path.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("math_signals.json.bad")).unwrap(),
            "not json"
        );
    }
}
