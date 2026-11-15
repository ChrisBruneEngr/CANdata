//! The functions a Math signal equation can call.

/// One function: its name, how many values it takes, and help text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Function {
    pub name: &'static str,
    /// The parameters as shown to the user, e.g. "x, low, high".
    pub params: &'static str,
    pub min_args: usize,
    /// `None` means any number from `min_args` up.
    pub max_args: Option<usize>,
    pub description: &'static str,
    /// Uses the signal's history (earlier points), not just the current point.
    pub over_time: bool,
}

const fn f(
    name: &'static str,
    params: &'static str,
    min_args: usize,
    max_args: Option<usize>,
    description: &'static str,
    over_time: bool,
) -> Function {
    Function { name, params, min_args, max_args, description, over_time }
}

/// Every function, in the order the Math screen lists them.
pub const FUNCTIONS: &[Function] = &[
    f("abs", "x", 1, Some(1), "Absolute value", false),
    f("sqrt", "x", 1, Some(1), "Square root", false),
    f("exp", "x", 1, Some(1), "e to the power x", false),
    f("ln", "x", 1, Some(1), "Natural logarithm", false),
    f("log10", "x", 1, Some(1), "Base-10 logarithm", false),
    f("sin", "x", 1, Some(1), "Sine (x in radians)", false),
    f("cos", "x", 1, Some(1), "Cosine (x in radians)", false),
    f("tan", "x", 1, Some(1), "Tangent (x in radians)", false),
    f("round", "x", 1, Some(1), "Nearest whole number", false),
    f("min", "a, b, ...", 2, None, "Smallest of the values", false),
    f("max", "a, b, ...", 2, None, "Largest of the values", false),
    f("clamp", "x, low, high", 3, Some(3), "x kept between low and high", false),
    f("if", "condition, then, else", 3, Some(3), "then where condition is not 0, else otherwise", false),
    f("derivative", "x", 1, Some(1), "Rate of change of x per second", true),
    f("integral", "x", 1, Some(1), "Running total of x over time (x · s)", true),
    f("lowpass", "x, seconds", 2, Some(2), "x smoothed with a time constant in seconds", true),
];

/// The function with this exact name.
pub fn find(name: &str) -> Option<&'static Function> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// A function whose name differs only in capitals, for a "did you mean" hint.
pub fn find_ignoring_case(name: &str) -> Option<&'static Function> {
    FUNCTIONS.iter().find(|f| f.name.eq_ignore_ascii_case(name))
}

impl Function {
    /// "abs(x)"
    pub fn signature(&self) -> String {
        format!("{}({})", self.name, self.params)
    }

    /// Why `count` values don't suit this function, if they don't.
    pub fn check_arg_count(&self, count: usize) -> Result<(), String> {
        let ok = count >= self.min_args && self.max_args.is_none_or(|max| count <= max);
        if ok {
            return Ok(());
        }
        let wanted = match self.max_args {
            Some(max) if max == self.min_args => format!("{}", max),
            Some(max) => format!("{} to {}", self.min_args, max),
            None => format!("at least {}", self.min_args),
        };
        let plural = |n: usize| if n == 1 { "value" } else { "values" };
        Err(format!(
            "{} takes {} {}, not {} ({})",
            self.name,
            wanted,
            plural(self.max_args.unwrap_or(self.min_args)),
            count,
            self.signature()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_identifiers() {
        for (i, f) in FUNCTIONS.iter().enumerate() {
            assert!(crate::library::math_signals::is_identifier(f.name), "{}", f.name);
            assert!(FUNCTIONS[..i].iter().all(|g| g.name != f.name), "{}", f.name);
        }
    }

    #[test]
    fn arg_counts() {
        let abs = find("abs").unwrap();
        assert!(abs.check_arg_count(1).is_ok());
        assert_eq!(abs.check_arg_count(2).unwrap_err(), "abs takes 1 value, not 2 (abs(x))");
        let max = find("max").unwrap();
        assert!(max.check_arg_count(5).is_ok());
        assert_eq!(max.check_arg_count(1).unwrap_err(), "max takes at least 2 values, not 1 (max(a, b, ...))");
        assert_eq!(find_ignoring_case("ABS").map(|f| f.name), Some("abs"));
        assert!(find("ABS").is_none());
    }
}
