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
    /// Decimal places for values shown at the plot cursor. Files saved
    /// before this setting existed get the old fixed 3.
    #[serde(default = "default_decimals")]
    pub decimals: u8,
}

/// Decimal places a new Math signal shows.
pub const DEFAULT_DECIMALS: u8 = 3;
/// Most decimal places a Math signal can show.
pub const MAX_DECIMALS: u8 = 9;

fn default_decimals() -> u8 {
    DEFAULT_DECIMALS
}

/// Read the Decimals box: a whole number from 0 to `MAX_DECIMALS`.
pub fn parse_decimals(text: &str) -> Result<u8, String> {
    match text.trim().parse::<u8>() {
        Ok(n) if n <= MAX_DECIMALS => Ok(n),
        _ => Err(format!("Decimals must be a whole number from 0 to {}", MAX_DECIMALS)),
    }
}

impl MathSignal {
    /// A new, empty Math signal.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            unit: String::new(),
            equation: String::new(),
            time_mode: TimeMode::default(),
            decimals: DEFAULT_DECIMALS,
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
        if let Some(function) = crate::math::functions::find_ignoring_case(name) {
            return Err(format!("'{}' is the name of a function", function.name));
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

// ---------------------------------------------------------------------------
// Export / import (`.cvmath` files)
// ---------------------------------------------------------------------------

/// File extension for Math signal export files.
pub const MATH_EXTENSION: &str = "cvmath";
const MATH_FORMAT: &str = "candata-math";
const MATH_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct MathBundle {
    format: String,
    format_version: u32,
    #[serde(default)]
    exported_at: String,
    signals: Vec<MathSignal>,
}

/// Write Math signals to a `.cvmath` file.
pub fn export_math_signals(signals: &[MathSignal], path: &Path) -> Result<(), String> {
    let bundle = MathBundle {
        format: MATH_FORMAT.into(),
        format_version: MATH_FORMAT_VERSION,
        exported_at: chrono::Local::now().to_rfc3339(),
        signals: signals.to_vec(),
    };
    let content = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("Failed to serialize Math signals: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Read the Math signals of a `.cvmath` file.
pub fn read_math_signals(path: &Path) -> Result<Vec<MathSignal>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_math_bundle(&content)
}

fn parse_math_bundle(content: &str) -> Result<Vec<MathSignal>, String> {
    let bundle: MathBundle = serde_json::from_str(content)
        .map_err(|_| "This is not a CANdata Math signal file".to_string())?;
    if bundle.format != MATH_FORMAT {
        return Err("This is not a CANdata Math signal file".into());
    }
    if bundle.format_version > MATH_FORMAT_VERSION {
        return Err("This file was made by a newer version of CANdata".into());
    }
    let bad: Vec<&str> = bundle
        .signals
        .iter()
        .map(|s| s.name.as_str())
        .filter(|n| !is_identifier(n))
        .collect();
    if !bad.is_empty() {
        return Err(format!("The file has Math signals with invalid names: {}", bad.join(", ")));
    }
    let mut seen = MathSignalStore::default();
    for signal in &bundle.signals {
        if seen.position(&signal.name).is_some() {
            return Err(format!("The file has '{}' more than once", signal.name));
        }
        seen.signals.push(MathSignal::new(signal.name.clone()));
    }
    Ok(bundle.signals)
}

/// What to do with an imported Math signal whose name is already used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictChoice {
    /// Overwrite the existing Math signal.
    Replace,
    /// Add it under a free name: `Name_2`, `Name_3`, ...
    KeepBoth,
    /// Leave the existing one and drop the imported one.
    Skip,
}

/// What an import did.
#[derive(Debug, Default, PartialEq)]
pub struct ImportSummary {
    pub added: Vec<String>,
    pub replaced: Vec<String>,
    /// (imported name, name it was added as)
    pub renamed: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

impl ImportSummary {
    /// One line for the status bar.
    pub fn message(&self) -> String {
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!("added {}", self.added.join(", ")));
        }
        if !self.replaced.is_empty() {
            parts.push(format!("replaced {}", self.replaced.join(", ")));
        }
        if !self.renamed.is_empty() {
            let renamed: Vec<String> = self.renamed.iter().map(|(a, b)| format!("{} as {}", a, b)).collect();
            parts.push(format!("added {}", renamed.join(", ")));
        }
        if !self.skipped.is_empty() {
            parts.push(format!("skipped {}", self.skipped.join(", ")));
        }
        if parts.is_empty() {
            "The file has no Math signals".into()
        } else {
            format!("Math signals imported: {}", parts.join("; "))
        }
    }
}

impl MathSignalStore {
    /// Names of `incoming` that are already used here (ignoring case).
    pub fn conflicts(&self, incoming: &[MathSignal]) -> Vec<String> {
        incoming
            .iter()
            .filter(|s| self.position(&s.name).is_some())
            .map(|s| s.name.clone())
            .collect()
    }

    /// Add `incoming`, settling used names with `choice`.
    pub fn import(&mut self, incoming: Vec<MathSignal>, choice: ConflictChoice) -> ImportSummary {
        let mut summary = ImportSummary::default();
        for mut signal in incoming {
            match (self.position(&signal.name), choice) {
                (None, _) => {
                    summary.added.push(signal.name.clone());
                    self.signals.push(signal);
                }
                (Some(i), ConflictChoice::Replace) => {
                    // Keep the existing spelling so plots and sets still match.
                    signal.name = self.signals[i].name.clone();
                    summary.replaced.push(signal.name.clone());
                    self.signals[i] = signal;
                }
                (Some(_), ConflictChoice::KeepBoth) => {
                    let new_name = (2..)
                        .map(|n| format!("{}_{}", signal.name, n))
                        .find(|name| self.position(name).is_none())
                        .expect("an unused name always exists");
                    summary.renamed.push((signal.name.clone(), new_name.clone()));
                    signal.name = new_name;
                    self.signals.push(signal);
                }
                (Some(_), ConflictChoice::Skip) => summary.skipped.push(signal.name.clone()),
            }
        }
        summary
    }
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
        assert_eq!(store.validate_name("Max", None).unwrap_err(), "'max' is the name of a function");
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
                    decimals: 1,
                },
                MathSignal {
                    name: "Slip".into(),
                    unit: "%".into(),
                    equation: "(A - B) / B * 100".into(),
                    time_mode: TimeMode::Resample {
                        rate_hz: 100.0,
                        interpolation: Interpolation::Linear,
                    },
                    decimals: 0,
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
    fn decimals() {
        assert_eq!(MathSignal::new("A").decimals, 3);
        assert_eq!(parse_decimals(" 2 "), Ok(2));
        assert_eq!(parse_decimals("0"), Ok(0));
        assert_eq!(parse_decimals("9"), Ok(9));
        for bad in ["10", "-1", "1.5", "", "two"] {
            assert!(parse_decimals(bad).is_err(), "{bad}");
        }
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

    fn eq(name: &str, equation: &str) -> MathSignal {
        MathSignal { equation: equation.into(), ..MathSignal::new(name) }
    }

    #[test]
    fn export_import_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.cvmath");
        let signals = vec![eq("P", "a * b"), eq("Q", "a / 2")];
        export_math_signals(&signals, &path).unwrap();
        assert_eq!(read_math_signals(&path).unwrap(), signals);
    }

    #[test]
    fn bad_files_are_refused() {
        assert!(parse_math_bundle("not json").is_err());
        assert!(parse_math_bundle(r#"{"format":"candata-other","format_version":1,"signals":[]}"#).is_err());
        assert_eq!(
            parse_math_bundle(r#"{"format":"candata-math","format_version":99,"signals":[]}"#).unwrap_err(),
            "This file was made by a newer version of CANdata"
        );
        assert!(
            parse_math_bundle(r#"{"format":"candata-math","format_version":1,"signals":[{"name":"2x"}]}"#)
                .unwrap_err()
                .contains("invalid names: 2x")
        );
        assert!(
            parse_math_bundle(
                r#"{"format":"candata-math","format_version":1,"signals":[{"name":"A"},{"name":"a"}]}"#
            )
            .unwrap_err()
            .contains("more than once")
        );
    }

    #[test]
    fn import_conflicts() {
        let base = MathSignalStore { signals: vec![eq("P", "old"), eq("P_2", "taken")] };
        let incoming = || vec![eq("p", "new"), eq("R", "r")];
        assert_eq!(base.conflicts(&incoming()), ["p"]);

        let mut store = base.clone();
        let summary = store.import(incoming(), ConflictChoice::Replace);
        assert_eq!(summary.replaced, ["P"]);
        assert_eq!(summary.added, ["R"]);
        assert_eq!(store.signals[0], eq("P", "new"));
        assert_eq!(store.signals.len(), 3);

        let mut store = base.clone();
        let summary = store.import(incoming(), ConflictChoice::KeepBoth);
        assert_eq!(summary.renamed, [("p".to_string(), "p_3".to_string())]);
        assert_eq!(store.signals[0], eq("P", "old"));
        assert!(store.position("p_3").is_some());

        let mut store = base.clone();
        let summary = store.import(incoming(), ConflictChoice::Skip);
        assert_eq!(summary.skipped, ["p"]);
        assert_eq!(store.signals.len(), 3);
        assert_eq!(summary.message(), "Math signals imported: added R; skipped p");
    }
}
