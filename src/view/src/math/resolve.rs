//! Checking a Math signal equation against the active library: syntax first,
//! then every signal name it uses.

use super::parser::{Expr, ParseError, parse, position_text};
use crate::app::CanDataApp;
use std::collections::HashMap;
use std::ops::Range;

/// Where a decoded signal comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalLocation {
    pub channel_id: u16,
    pub msg_id: u32,
    pub is_can: bool,
}

/// Every signal name of the loaded channels, and where each is found.
#[derive(Debug, Default)]
pub struct SignalIndex {
    by_name: HashMap<String, Vec<SignalLocation>>,
}

impl SignalIndex {
    /// Index the signals of the app's loaded channels.
    pub fn from_app(app: &CanDataApp) -> Self {
        let mut index = Self::default();
        for (&channel_id, dbc) in &app.dbc_channels {
            for msg in dbc.messages.values() {
                for name in msg.signals.keys() {
                    index.add(name, SignalLocation { channel_id, msg_id: msg.id, is_can: true });
                }
            }
        }
        for (&channel_id, ldf) in &app.ldf_channels {
            for frame in ldf.frames.values() {
                for mapping in &frame.signals {
                    index.add(
                        &mapping.signal_name,
                        SignalLocation { channel_id, msg_id: frame.id, is_can: false },
                    );
                }
            }
        }
        for locations in index.by_name.values_mut() {
            locations.sort_by_key(|l| (l.channel_id, l.msg_id));
            locations.dedup();
        }
        index
    }

    pub fn add(&mut self, name: &str, location: SignalLocation) {
        self.by_name.entry(name.to_string()).or_default().push(location);
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    fn get(&self, name: &str) -> &[SignalLocation] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    /// A name that differs only in capitals, for a "did you mean" hint.
    fn same_but_case(&self, name: &str) -> Option<&str> {
        self.by_name
            .keys()
            .filter(|k| k.eq_ignore_ascii_case(name))
            .min()
            .map(String::as_str)
    }
}

/// A problem with one name in an equation.
#[derive(Debug, Clone, PartialEq)]
pub struct NameIssue {
    pub message: String,
    pub range: Range<usize>,
}

/// The result of checking an equation.
#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    /// Every name is found exactly once.
    Ok { signals: Vec<(String, SignalLocation)> },
    /// The syntax is fine but no channels are loaded to check names against.
    NoLibrary,
    /// The syntax is fine but some names can't be used with the active library.
    Names(Vec<NameIssue>),
    /// The equation can't be read (this includes an empty equation).
    Syntax(ParseError),
}

impl Check {
    /// True when the equation can't be saved.
    pub fn is_error(&self) -> bool {
        matches!(self, Check::Syntax(_))
    }

    /// One line for the editor's status line.
    pub fn summary(&self, source: &str) -> String {
        match self {
            Check::Ok { .. } => "✓ OK".into(),
            Check::NoLibrary => "⚠ Syntax OK. Activate a library to check the signal names.".into(),
            Check::Names(issues) => {
                let first = &issues[0];
                let more = match issues.len() - 1 {
                    0 => String::new(),
                    1 => " (and 1 more problem)".into(),
                    n => format!(" (and {} more problems)", n),
                };
                format!("⚠ {} at {}{}", first.message, position_text(source, first.range.start), more)
            }
            Check::Syntax(e) if e.message == "The equation is empty" => "✗ The equation is empty.".into(),
            Check::Syntax(e) => format!("✗ {} at {}", e.message, position_text(source, e.range.start)),
        }
    }
}

/// Check an equation: syntax, then names against `index`.
pub fn check(source: &str, index: &SignalIndex) -> Check {
    match parse(source) {
        Ok(expr) => check_names(&expr, index),
        Err(e) => Check::Syntax(e),
    }
}

fn check_names(expr: &Expr, index: &SignalIndex) -> Check {
    let mut issues = Vec::new();
    // No functions exist yet; they are added in a later phase.
    for (name, range) in expr.function_names() {
        issues.push(NameIssue { message: format!("Unknown function '{}'", name), range });
    }
    if index.is_empty() {
        return if issues.is_empty() { Check::NoLibrary } else { Check::Names(issues) };
    }
    let mut signals: Vec<(String, SignalLocation)> = Vec::new();
    for (name, range) in expr.signal_names() {
        match index.get(name) {
            [location] => {
                if !signals.iter().any(|(n, _)| n == name) {
                    signals.push((name.to_string(), *location));
                }
            }
            [] => {
                let message = match index.same_but_case(name) {
                    Some(similar) => format!("Unknown signal '{}' (did you mean '{}'?)", name, similar),
                    None => format!("Unknown signal '{}'", name),
                };
                issues.push(NameIssue { message, range });
            }
            many => {
                let places: Vec<String> = many
                    .iter()
                    .map(|l| {
                        format!("Ch{} {}", l.channel_id, crate::rendering::format_message_id(l.msg_id, false))
                    })
                    .collect();
                issues.push(NameIssue {
                    message: format!("'{}' is in more than one message ({})", name, places.join(", ")),
                    range,
                });
            }
        }
    }
    // Report problems in the order they appear.
    issues.sort_by_key(|i| i.range.start);
    if issues.is_empty() { Check::Ok { signals } } else { Check::Names(issues) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SignalIndex {
        let mut index = SignalIndex::default();
        let at = |channel_id, msg_id| SignalLocation { channel_id, msg_id, is_can: true };
        index.add("Motor_Speed", at(1, 0x100));
        index.add("Motor_Torque", at(1, 0x100));
        index.add("Speed", at(1, 0x200));
        index.add("Speed", at(2, 0x300));
        index
    }

    #[test]
    fn ok_lists_each_signal_once() {
        let check = check("Motor_Speed * Motor_Torque + Motor_Speed", &index());
        let Check::Ok { signals } = check else { panic!("{check:?}") };
        let names: Vec<_> = signals.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Motor_Speed", "Motor_Torque"]);
        assert_eq!(signals[0].1, SignalLocation { channel_id: 1, msg_id: 0x100, is_can: true });
    }

    #[test]
    fn unknown_signal_with_case_hint() {
        let check = check("motor_speed + Nope", &index());
        let Check::Names(issues) = &check else { panic!("{check:?}") };
        assert_eq!(issues[0].message, "Unknown signal 'motor_speed' (did you mean 'Motor_Speed'?)");
        assert_eq!(issues[0].range, 0..11);
        assert_eq!(issues[1].message, "Unknown signal 'Nope'");
        assert_eq!(
            check.summary("motor_speed + Nope"),
            "⚠ Unknown signal 'motor_speed' (did you mean 'Motor_Speed'?) at column 1 (and 1 more problem)"
        );
        assert!(!check.is_error());
    }

    #[test]
    fn ambiguous_signal() {
        let Check::Names(issues) = check("2 * Speed", &index()) else { panic!() };
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.starts_with("'Speed' is in more than one message (Ch1 "));
        assert_eq!(issues[0].range, 4..9);
    }

    #[test]
    fn functions_are_unknown_for_now() {
        let Check::Names(issues) = check("abs(Motor_Speed)", &index()) else { panic!() };
        assert_eq!(issues[0].message, "Unknown function 'abs'");
    }

    #[test]
    fn syntax_errors_are_errors() {
        let check = check("Motor_Speed *", &index());
        assert!(check.is_error());
        assert_eq!(check.summary("Motor_Speed *"), "✗ A value is missing after '*' at column 14");
        assert_eq!(super::check("", &index()).summary(""), "✗ The equation is empty.");
    }

    #[test]
    fn no_library() {
        let empty = SignalIndex::default();
        assert_eq!(check("a * b", &empty), Check::NoLibrary);
        assert!(!check("a * b", &empty).is_error());
    }
}
