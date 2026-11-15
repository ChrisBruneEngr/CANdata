//! Checking a Math signal equation against the active library: syntax first,
//! then every name it uses. A name may be a decoded signal or another Math
//! signal; Math signals that use each other in a loop are reported.

use super::functions;
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

/// Every signal name of the loaded channels and where each is found, plus
/// every Math signal and its equation.
#[derive(Debug, Default)]
pub struct SignalIndex {
    by_name: HashMap<String, Vec<SignalLocation>>,
    math: HashMap<String, String>,
}

impl SignalIndex {
    /// Index the signals of the app's loaded channels and its Math signals.
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
        for signal in &app.math_store.signals {
            index.add_math(&signal.name, &signal.equation);
        }
        index
    }

    pub fn add(&mut self, name: &str, location: SignalLocation) {
        self.by_name.entry(name.to_string()).or_default().push(location);
    }

    pub fn add_math(&mut self, name: &str, equation: &str) {
        self.math.insert(name.to_string(), equation.to_string());
    }

    /// True when no decoded signals are loaded.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    fn get(&self, name: &str) -> &[SignalLocation] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    /// A signal or Math signal name that differs only in capitals, for a
    /// "did you mean" hint.
    fn same_but_case(&self, name: &str) -> Option<&str> {
        self.by_name
            .keys()
            .chain(self.math.keys())
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
    /// Every name is found exactly once. `signals` are the decoded signals it
    /// uses directly, `math` the Math signals it uses directly.
    Ok { signals: Vec<(String, SignalLocation)>, math: Vec<String> },
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

    /// The first problem as plain text, for messages about other equations.
    fn first_problem(&self) -> Option<String> {
        match self {
            Check::Ok { .. } | Check::NoLibrary => None,
            Check::Names(issues) => Some(issues[0].message.clone()),
            Check::Syntax(e) => Some(e.message.clone()),
        }
    }
}

/// Check an equation that doesn't belong to a Math signal.
pub fn check(source: &str, index: &SignalIndex) -> Check {
    check_named(None, source, index)
}

/// Check the equation of Math signal `name` (if it has one). Using itself,
/// directly or through other Math signals, is reported as a loop.
pub fn check_named(name: Option<&str>, source: &str, index: &SignalIndex) -> Check {
    let mut stack: Vec<String> = name.map(str::to_string).into_iter().collect();
    match parse(source) {
        Ok(expr) => check_names(&expr, index, &mut stack),
        Err(e) => Check::Syntax(e),
    }
}

/// `stack` is the chain of Math signals being checked, outermost first.
fn check_names(expr: &Expr, index: &SignalIndex, stack: &mut Vec<String>) -> Check {
    let mut issues = Vec::new();
    for (name, range, count) in expr.calls() {
        let problem = match functions::find(name) {
            Some(function) => function.check_arg_count(count).err(),
            None => Some(match functions::find_ignoring_case(name) {
                Some(similar) => format!("Unknown function '{}' (did you mean '{}'?)", name, similar.name),
                None => format!("Unknown function '{}'", name),
            }),
        };
        if let Some(message) = problem {
            issues.push(NameIssue { message, range });
        }
    }

    let no_library = index.is_empty();
    let mut signals: Vec<(String, SignalLocation)> = Vec::new();
    let mut math: Vec<String> = Vec::new();
    for (name, range) in expr.signal_names() {
        let decoded = index.get(name);
        if let Some(equation) = index.math.get(name) {
            if !decoded.is_empty() {
                issues.push(NameIssue {
                    message: format!("'{}' is both a signal and a Math signal; rename the Math signal", name),
                    range,
                });
            } else if let Some(problem) = check_math_input(name, equation, index, stack) {
                issues.push(NameIssue { message: problem, range });
            } else if !math.iter().any(|m| m == name) {
                math.push(name.to_string());
            }
            continue;
        }
        if no_library {
            continue;
        }
        match decoded {
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
    if !issues.is_empty() {
        Check::Names(issues)
    } else if no_library {
        Check::NoLibrary
    } else {
        Check::Ok { signals, math }
    }
}

/// Why Math signal `name`, used as an input, can't be computed, if it can't.
fn check_math_input(name: &str, equation: &str, index: &SignalIndex, stack: &mut Vec<String>) -> Option<String> {
    if let Some(start) = stack.iter().position(|s| s == name) {
        let mut chain: Vec<&str> = stack[start..].iter().map(String::as_str).collect();
        chain.push(name);
        return Some(format!("Loop: {}", chain.join(" → ")));
    }
    stack.push(name.to_string());
    let result = match parse(equation) {
        Ok(expr) => check_names(&expr, index, stack),
        Err(e) => Check::Syntax(e),
    };
    stack.pop();
    result.first_problem().map(|problem| {
        // A loop found further in reads the same from here.
        if problem.starts_with("Loop: ") {
            problem
        } else {
            format!("Math signal '{}' has a problem: {}", name, problem)
        }
    })
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
        let Check::Ok { signals, math } = check else { panic!("{check:?}") };
        let names: Vec<_> = signals.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Motor_Speed", "Motor_Torque"]);
        assert_eq!(signals[0].1, SignalLocation { channel_id: 1, msg_id: 0x100, is_can: true });
        assert!(math.is_empty());
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
    fn functions_are_checked() {
        assert!(matches!(check("abs(Motor_Speed) + max(1, 2, 3)", &index()), Check::Ok { .. }));
        let Check::Names(issues) = check("Abs(Motor_Speed) + nope(1) + sqrt(1, 2)", &index()) else { panic!() };
        assert_eq!(issues[0].message, "Unknown function 'Abs' (did you mean 'abs'?)");
        assert_eq!(issues[1].message, "Unknown function 'nope'");
        assert_eq!(issues[2].message, "sqrt takes 1 value, not 2 (sqrt(x))");
        assert_eq!(issues[2].range, 29..33);
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

    fn with_math(list: &[(&str, &str)]) -> SignalIndex {
        let mut index = index();
        for (name, equation) in list {
            index.add_math(name, equation);
        }
        index
    }

    #[test]
    fn math_signals_as_inputs() {
        let index = with_math(&[("Power", "Motor_Speed * Motor_Torque"), ("Power_kW", "Power / 1000")]);
        let check = check_named(Some("Double"), "Power_kW * 2 + Motor_Speed", &index);
        let Check::Ok { signals, math } = check else { panic!("{check:?}") };
        assert_eq!(math, ["Power_kW"]);
        assert_eq!(signals.len(), 1);
        // Case hint covers Math signals too.
        let Check::Names(issues) = check_named(None, "power", &index) else { panic!() };
        assert_eq!(issues[0].message, "Unknown signal 'power' (did you mean 'Power'?)");
    }

    #[test]
    fn loops_are_found() {
        let index = with_math(&[("A", "B + 1"), ("B", "C * 2"), ("C", "A - Motor_Speed")]);
        let Check::Names(issues) = check_named(Some("A"), "B + 1", &index) else { panic!() };
        assert_eq!(issues[0].message, "Loop: A → B → C → A");
        assert_eq!(issues[0].range, 0..1);
        let Check::Names(issues) = check_named(Some("A"), "A * 2", &index) else { panic!() };
        assert_eq!(issues[0].message, "Loop: A → A");
        // An equation outside the loop that uses it still hears about it.
        let Check::Names(issues) = check_named(Some("D"), "B", &index) else { panic!() };
        assert_eq!(issues[0].message, "Loop: B → C → A → B");
    }

    #[test]
    fn broken_math_inputs_and_name_clashes() {
        let index = with_math(&[("Bad", "Nope * 2"), ("Speed_x", "1 +"), ("Motor_Speed", "1")]);
        let Check::Names(issues) = check_named(None, "Bad + Speed_x", &index) else { panic!() };
        assert_eq!(issues[0].message, "Math signal 'Bad' has a problem: Unknown signal 'Nope'");
        assert_eq!(issues[1].message, "Math signal 'Speed_x' has a problem: A value is missing after '+'");
        let Check::Names(issues) = check_named(None, "Motor_Speed", &index) else { panic!() };
        assert!(issues[0].message.starts_with("'Motor_Speed' is both a signal and a Math signal"));
    }

    #[test]
    fn math_inputs_without_a_library() {
        let mut index = SignalIndex::default();
        index.add_math("A", "x * 2");
        assert_eq!(check_named(Some("B"), "A + 1", &index), Check::NoLibrary);
        index.add_math("L", "L");
        let Check::Names(issues) = check_named(Some("B"), "L", &index) else { panic!() };
        assert_eq!(issues[0].message, "Loop: L → L");
    }
}
