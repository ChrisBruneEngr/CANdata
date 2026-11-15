//! Parser for Math signal equations.
//!
//! Grammar, lowest precedence first:
//!
//! ```text
//! compare := expr   (CMP expr)?            // at most one comparison
//! expr    := term   (('+' | '-') term)*
//! term    := unary  (('*' | '/') unary)*
//! unary   := '-' unary | power
//! power   := primary ('^' unary)?          // right-associative: 2^3^2 = 2^9
//! primary := NUMBER | NAME | NAME '(' args? ')' | '(' compare ')'
//! args    := compare (',' compare)*
//! CMP     := '<' | '<=' | '>' | '>=' | '==' | '!='
//! ```
//!
//! A comparison gives 1 when true and 0 when false.
//!
//! Every error carries the character range it refers to, so the editor can
//! say where it is.

use std::ops::Range;

/// A parsed equation.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    /// A signal name, with its character range in the equation.
    Signal(String, Range<usize>),
    Neg(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    /// A function call, with the character range of the function name.
    Call(String, Range<usize>, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl Expr {
    /// Every signal name used, with its range, in the order written.
    pub fn signal_names(&self) -> Vec<(&str, Range<usize>)> {
        let mut names = Vec::new();
        self.visit_names(&mut |name, range, is_call| {
            if !is_call {
                names.push((name, range));
            }
        });
        names
    }

    /// Every function call, as (name, name range, number of values), in the
    /// order written.
    pub fn calls(&self) -> Vec<(&str, Range<usize>, usize)> {
        let mut calls = Vec::new();
        fn walk<'a>(e: &'a Expr, calls: &mut Vec<(&'a str, Range<usize>, usize)>) {
            match e {
                Expr::Number(_) | Expr::Signal(..) => {}
                Expr::Neg(e) => walk(e, calls),
                Expr::Binary(_, a, b) => {
                    walk(a, calls);
                    walk(b, calls);
                }
                Expr::Call(name, range, args) => {
                    calls.push((name.as_str(), range.clone(), args.len()));
                    for arg in args {
                        walk(arg, calls);
                    }
                }
            }
        }
        walk(self, &mut calls);
        calls
    }

    fn visit_names<'a>(&'a self, f: &mut impl FnMut(&'a str, Range<usize>, bool)) {
        match self {
            Expr::Number(_) => {}
            Expr::Signal(name, range) => f(name, range.clone(), false),
            Expr::Neg(e) => e.visit_names(f),
            Expr::Binary(_, a, b) => {
                a.visit_names(f);
                b.visit_names(f);
            }
            Expr::Call(name, range, args) => {
                f(name, range.clone(), true);
                for arg in args {
                    arg.visit_names(f);
                }
            }
        }
    }

    /// Evaluate at one moment with the given signal values (tests only; the
    /// app evaluates whole series with `math::eval`).
    #[cfg(test)]
    pub fn eval(&self, value_of: &dyn Fn(&str) -> f64) -> f64 {
        let values: std::collections::HashMap<&str, Vec<f64>> = self
            .signal_names()
            .into_iter()
            .map(|(name, _)| (name, vec![value_of(name)]))
            .collect();
        crate::math::eval::eval_series(self, &[0.0], &values)[0]
    }
}

/// A syntax error, with the character range it refers to.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub range: Range<usize>,
}

impl ParseError {
    fn new(message: impl Into<String>, range: Range<usize>) -> Self {
        Self { message: message.into(), range }
    }
}

/// Where a character offset is, for messages: "column 5", or
/// "line 2, column 3" when the equation has more than one line.
pub fn position_text(source: &str, offset: usize) -> String {
    let before: Vec<char> = source.chars().take(offset).collect();
    let line = before.iter().filter(|c| **c == '\n').count() + 1;
    let column = before.iter().rev().take_while(|c| **c != '\n').count() + 1;
    if source.contains('\n') {
        format!("line {}, column {}", line, column)
    } else {
        format!("column {}", column)
    }
}

/// True if `source` uses the name `name` as a signal (not as a function).
pub fn uses_name(source: &str, name: &str) -> bool {
    parse(source).is_ok_and(|expr| expr.signal_names().iter().any(|(n, _)| *n == name))
}

/// `source` with every use of signal `old` changed to `new`, or `None` if it
/// doesn't use `old` (or can't be read). Everything else is kept as written.
pub fn rename_signal(source: &str, old: &str, new: &str) -> Option<String> {
    let expr = parse(source).ok()?;
    let ranges: Vec<Range<usize>> =
        expr.signal_names().into_iter().filter(|(n, _)| *n == old).map(|(_, r)| r).collect();
    if ranges.is_empty() {
        return None;
    }
    let mut chars: Vec<char> = source.chars().collect();
    for range in ranges.into_iter().rev() {
        chars.splice(range, new.chars());
    }
    Some(chars.into_iter().collect())
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    Cmp(Op),
}

impl Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Num(_) => "a number".into(),
            Tok::Name(name) => format!("'{}'", name),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Caret => "'^'".into(),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::Comma => "','".into(),
            Tok::Cmp(op) => format!("'{}'", match op {
                Op::Lt => "<",
                Op::Le => "<=",
                Op::Gt => ">",
                Op::Ge => ">=",
                Op::Eq => "==",
                _ => "!=",
            }),
        }
    }
}

struct Token {
    tok: Tok,
    range: Range<usize>,
}

fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let single = match c {
            '+' => Some(Tok::Plus),
            '-' => Some(Tok::Minus),
            '*' => Some(Tok::Star),
            '/' => Some(Tok::Slash),
            '^' => Some(Tok::Caret),
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            ',' => Some(Tok::Comma),
            _ => None,
        };
        let next_is_eq = chars.get(i + 1) == Some(&'=');
        let compare = match c {
            '<' => Some(if next_is_eq { Op::Le } else { Op::Lt }),
            '>' => Some(if next_is_eq { Op::Ge } else { Op::Gt }),
            '=' if next_is_eq => Some(Op::Eq),
            '!' if next_is_eq => Some(Op::Ne),
            '=' => return Err(ParseError::new("Use == to compare; a single = is not allowed", start..start + 1)),
            _ => None,
        };
        if let Some(tok) = single {
            i += 1;
            tokens.push(Token { tok, range: start..i });
        } else if let Some(op) = compare {
            i += if matches!(op, Op::Lt | Op::Gt) { 1 } else { 2 };
            tokens.push(Token { tok: Tok::Cmp(op), range: start..i });
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                } else {
                    return Err(ParseError::new("Bad number: the exponent has no digits", start..j));
                }
            }
            let text: String = chars[start..i].iter().collect();
            let value = text
                .parse::<f64>()
                .map_err(|_| ParseError::new(format!("Bad number '{}'", text), start..i))?;
            tokens.push(Token { tok: Tok::Num(value), range: start..i });
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            tokens.push(Token { tok: Tok::Name(name), range: start..i });
        } else {
            return Err(ParseError::new(format!("Unexpected character '{}'", c), start..start + 1));
        }
    }
    Ok(tokens)
}

/// Parse an equation.
pub fn parse(source: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(source)?;
    let end = source.chars().count();
    if tokens.is_empty() {
        return Err(ParseError::new("The equation is empty", 0..0));
    }
    let mut parser = Parser { tokens, pos: 0, end };
    let expr = parser.compare()?;
    if let Some(token) = parser.peek() {
        let message = match token.tok {
            Tok::RParen => "')' has no matching '('".to_string(),
            _ => format!("Expected an operator before {}", token.tok.describe()),
        };
        return Err(ParseError::new(message, token.range.clone()));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Character count of the source, for errors at the end.
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_tok(&self) -> Option<&Tok> {
        self.peek().map(|t| &t.tok)
    }

    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    fn compare(&mut self) -> Result<Expr, ParseError> {
        let left = self.expr()?;
        let Some(Tok::Cmp(op)) = self.peek_tok().cloned() else {
            return Ok(left);
        };
        self.pos += 1;
        let right = self.expr()?;
        if let Some(token) = self.peek().filter(|t| matches!(t.tok, Tok::Cmp(_))) {
            return Err(ParseError::new(
                "Only one comparison at a time; use brackets or if()",
                token.range.clone(),
            ));
        }
        Ok(Expr::Binary(op, Box::new(left), Box::new(right)))
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek_tok() {
                Some(Tok::Plus) => Op::Add,
                Some(Tok::Minus) => Op::Sub,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.term()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek_tok() {
                Some(Tok::Star) => Op::Mul,
                Some(Tok::Slash) => Op::Div,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.peek_tok() == Some(&Tok::Minus) {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.power()
    }

    fn power(&mut self) -> Result<Expr, ParseError> {
        let base = self.primary()?;
        if self.peek_tok() == Some(&Tok::Caret) {
            self.pos += 1;
            let exponent = self.unary()?;
            return Ok(Expr::Binary(Op::Pow, Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    /// The error for a missing value: after the previous token, or at the end.
    fn missing_value(&self) -> ParseError {
        let previous = self.pos.checked_sub(1).and_then(|i| self.tokens.get(i));
        match (self.peek(), previous) {
            (None, Some(prev)) => ParseError::new(
                format!("A value is missing after {}", prev.tok.describe()),
                prev.range.end..self.end,
            ),
            (Some(token), Some(prev)) => ParseError::new(
                format!("A value is missing between {} and {}", prev.tok.describe(), token.tok.describe()),
                token.range.clone(),
            ),
            (Some(token), None) => ParseError::new(
                format!("A value is missing before {}", token.tok.describe()),
                token.range.clone(),
            ),
            (None, None) => ParseError::new("The equation is empty", 0..0),
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let Some(token) = self.peek() else {
            return Err(self.missing_value());
        };
        let range = token.range.clone();
        match token.tok.clone() {
            Tok::Num(n) => {
                self.pos += 1;
                Ok(Expr::Number(n))
            }
            Tok::Name(name) => {
                self.pos += 1;
                if self.peek_tok() == Some(&Tok::LParen) {
                    let open = self.next().map(|t| t.range.clone()).unwrap_or(range.clone());
                    let args = self.args(&open)?;
                    Ok(Expr::Call(name, range, args))
                } else {
                    Ok(Expr::Signal(name, range))
                }
            }
            Tok::LParen => {
                self.pos += 1;
                let inner = self.compare()?;
                self.close(&range)?;
                Ok(inner)
            }
            _ => Err(self.missing_value()),
        }
    }

    /// Arguments of a call, after its '('.
    fn args(&mut self, open: &Range<usize>) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.peek_tok() == Some(&Tok::RParen) {
            self.pos += 1;
            return Ok(args);
        }
        loop {
            args.push(self.compare()?);
            if self.peek_tok() == Some(&Tok::Comma) {
                self.pos += 1;
                continue;
            }
            self.close(open)?;
            return Ok(args);
        }
    }

    /// Expect the ')' that closes the '(' at `open`.
    fn close(&mut self, open: &Range<usize>) -> Result<(), ParseError> {
        match self.peek() {
            Some(t) if t.tok == Tok::RParen => {
                self.pos += 1;
                Ok(())
            }
            Some(t) => Err(ParseError::new(
                format!("Expected ')' or an operator before {}", t.tok.describe()),
                t.range.clone(),
            )),
            None => Err(ParseError::new("Missing ')' for this '('", open.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(source: &str) -> f64 {
        parse(source)
            .unwrap_or_else(|e| panic!("{source}: {e:?}"))
            .eval(&|name| match name {
                "a" => 2.0,
                "b" => 3.0,
                "Motor_Speed" => 1000.0,
                "Motor_Torque" => 50.0,
                _ => f64::NAN,
            })
    }

    fn err(source: &str) -> ParseError {
        parse(source).expect_err(source)
    }

    #[test]
    fn precedence() {
        assert_eq!(eval("1 + 2 * 3"), 7.0);
        assert_eq!(eval("(1 + 2) * 3"), 9.0);
        assert_eq!(eval("10 - 4 - 3"), 3.0);
        assert_eq!(eval("8 / 4 / 2"), 1.0);
        assert_eq!(eval("2 * 3 ^ 2"), 18.0);
        assert_eq!(eval("a * b + a"), 8.0);
        assert_eq!(eval("Motor_Speed * Motor_Torque / 9549"), 1000.0 * 50.0 / 9549.0);
    }

    #[test]
    fn power_is_right_associative_and_binds_tighter_than_minus() {
        assert_eq!(eval("2 ^ 3 ^ 2"), 512.0);
        assert_eq!(eval("-2 ^ 2"), -4.0);
        assert_eq!(eval("2 ^ -1"), 0.5);
        assert_eq!(eval("--a"), 2.0);
        assert_eq!(eval("a - -b"), 5.0);
    }

    #[test]
    fn comparisons() {
        assert_eq!(eval("a < b"), 1.0);
        assert_eq!(eval("a >= b"), 0.0);
        assert_eq!(eval("a + 1 == b"), 1.0);
        assert_eq!(eval("a != 2"), 0.0);
        assert_eq!(eval("(a > 1) * 10"), 10.0);
        assert_eq!(eval("a <= 2"), 1.0);
        let e = err("a < b < 3");
        assert_eq!((e.message.as_str(), e.range), ("Only one comparison at a time; use brackets or if()", 6..7));
        assert_eq!(err("a = b").message, "Use == to compare; a single = is not allowed");
        assert_eq!(err("a ! b").message, "Unexpected character '!'");
        assert_eq!(err("a <").message, "A value is missing after '<'");
    }

    #[test]
    fn numbers() {
        for (text, value) in [("12", 12.0), ("1.5", 1.5), (".5", 0.5), ("1.", 1.0), ("1e-3", 0.001), ("2.5E+4", 25000.0), ("3e2", 300.0)] {
            assert_eq!(eval(text), value, "{text}");
        }
        assert_eq!(err("1e").range, 0..2);
        assert_eq!(err("1e+").range, 0..3);
    }

    #[test]
    fn whitespace_and_lines_are_ignored() {
        assert_eq!(eval("  a\n *\tb  "), 6.0);
    }

    #[test]
    fn calls_parse() {
        let expr = parse("max(a, b * 2) + f()").unwrap();
        let calls: Vec<_> = expr.calls().into_iter().map(|(n, _, count)| (n, count)).collect();
        assert_eq!(calls, [("max", 2), ("f", 0)]);
        let signals: Vec<_> = expr.signal_names().into_iter().map(|(n, r)| (n, r)).collect();
        assert_eq!(signals, [("a", 4..5), ("b", 7..8)]);
    }

    #[test]
    fn errors_point_at_the_problem() {
        let e = err("Speed $ 2");
        assert_eq!((e.message.as_str(), e.range), ("Unexpected character '$'", 6..7));

        let e = err("Speed *");
        assert_eq!(e.message, "A value is missing after '*'");
        assert_eq!(e.range.start, 7);

        let e = err("a * * b");
        assert_eq!(e.message, "A value is missing between '*' and '*'");
        assert_eq!(e.range, 4..5);

        let e = err("* b");
        assert_eq!(e.message, "A value is missing before '*'");

        let e = err("(Speed + 1");
        assert_eq!((e.message.as_str(), e.range), ("Missing ')' for this '('", 0..1));

        let e = err("Speed + 1)");
        assert_eq!((e.message.as_str(), e.range), ("')' has no matching '('", 9..10));

        let e = err("2 Speed");
        assert_eq!((e.message.as_str(), e.range), ("Expected an operator before 'Speed'", 2..7));

        let e = err("(a b)");
        assert_eq!(e.message, "Expected ')' or an operator before 'b'");

        let e = err("max(a,)");
        assert_eq!(e.message, "A value is missing between ',' and ')'");

        assert_eq!(err("   ").message, "The equation is empty");
        assert_eq!(err("").message, "The equation is empty");
        assert_eq!(err("()").message, "A value is missing between '(' and ')'");
    }

    #[test]
    fn ranges_count_characters_not_bytes() {
        // 'µ' is two bytes in UTF-8 but one character.
        let e = err("µ");
        assert_eq!(e.range, 0..1);
        let e = err("a + µ");
        assert_eq!(e.range, 4..5);
    }

    #[test]
    fn rename_and_uses() {
        assert_eq!(
            rename_signal("Power * 2 + max(Power,
 Power_2) + µ_Power", "Power", "P"),
            None, // 'µ' can't be read, so nothing is renamed
        );
        assert_eq!(
            rename_signal("Power*2 + max(Power,
 Power_2)", "Power", "Motor_Power").as_deref(),
            Some("Motor_Power*2 + max(Motor_Power,
 Power_2)")
        );
        assert_eq!(rename_signal("power + 1", "Power", "P"), None);
        // Function names are not signals.
        assert_eq!(rename_signal("abs(x)", "abs", "y"), None);
        assert!(uses_name("a + b * 2", "b"));
        assert!(!uses_name("abs(a)", "abs"));
        assert!(!uses_name("a +", "a"));
    }

    #[test]
    fn position_text_single_and_multi_line() {
        assert_eq!(position_text("a + b", 4), "column 5");
        assert_eq!(position_text("a +\n  b", 6), "line 2, column 3");
        assert_eq!(position_text("a +\nb", 0), "line 1, column 1");
    }
}
