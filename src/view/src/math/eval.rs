//! Evaluating a Math signal equation over time.
//!
//! The inputs are the decoded `(time, value)` points of each signal the
//! equation names, sorted by time. They rarely share timestamps, so they are
//! lined up first, as the Math signal's time mode says:
//!
//! - Sample and hold: a point at every time any input has a sample; each
//!   input holds its latest value.
//! - Resample: a point every `1 / rate_hz` seconds; each input is read with
//!   hold or linear interpolation.
//!
//! Output starts once every input has a value. The equation is then worked
//! out over the whole series at once, so functions such as `derivative` can
//! use earlier points. Points whose result is NaN or infinite (division by
//! zero, ...) are dropped.

use super::parser::{Expr, Op};
use crate::library::math_signals::{Interpolation, TimeMode};
use std::collections::HashMap;

/// Most points one Math signal may produce in resample mode.
pub const MAX_RESAMPLED_POINTS: usize = 10_000_000;

/// The decoded samples of one input signal: (time in seconds, value), sorted by time.
pub type Samples = Vec<(f64, f64)>;

/// Evaluate `expr` over time. `inputs` holds the samples of every signal it
/// names. Returns (time, value) pairs, or an error for the status bar.
pub fn evaluate(
    expr: &Expr,
    inputs: &HashMap<String, Samples>,
    time_mode: TimeMode,
) -> Result<Vec<(f64, f64)>, String> {
    let names: Vec<&str> = {
        let mut names: Vec<&str> = expr.signal_names().into_iter().map(|(n, _)| n).collect();
        names.sort_unstable();
        names.dedup();
        names
    };
    let mut series: Vec<&[(f64, f64)]> = Vec::with_capacity(names.len());
    for name in &names {
        match inputs.get(*name) {
            Some(samples) if !samples.is_empty() => series.push(samples),
            _ => return Err(format!("'{}' has no data in this log", name)),
        }
    }

    // An equation of constants only: one value over the whole log is not
    // meaningful without a time base, so it needs at least one signal.
    if series.is_empty() {
        return Err("The equation uses no signals".into());
    }

    // Output starts when every input has its first sample.
    let start = series.iter().map(|s| s[0].0).fold(f64::NEG_INFINITY, f64::max);
    let end = series.iter().map(|s| s[s.len() - 1].0).fold(f64::NEG_INFINITY, f64::max);

    let times: Vec<f64> = match time_mode {
        TimeMode::SampleAndHold => union_times(&series, start),
        TimeMode::Resample { rate_hz, .. } => {
            if !(rate_hz.is_finite() && rate_hz > 0.0) {
                return Err("The resample rate must be more than 0 Hz".into());
            }
            let count = ((end - start) * rate_hz).floor() as usize + 1;
            if count > MAX_RESAMPLED_POINTS {
                return Err(format!(
                    "{} Hz over this log gives {} points; the most is {}",
                    rate_hz, count, MAX_RESAMPLED_POINTS
                ));
            }
            (0..count).map(|i| start + i as f64 / rate_hz).collect()
        }
    };
    let interpolation = match time_mode {
        TimeMode::Resample { interpolation, .. } => interpolation,
        TimeMode::SampleAndHold => Interpolation::Hold,
    };

    // Read every input at every output time. `cursor` is the last sample at
    // or before the current time, walked forward as time goes on.
    let mut values: HashMap<&str, Vec<f64>> = HashMap::with_capacity(names.len());
    for (k, samples) in series.iter().enumerate() {
        let mut cursor = 0usize;
        let column = times
            .iter()
            .map(|&t| {
                while cursor + 1 < samples.len() && samples[cursor + 1].0 <= t {
                    cursor += 1;
                }
                read(samples, cursor, t, interpolation)
            })
            .collect();
        values.insert(names[k], column);
    }

    let results = eval_series(expr, &times, &values);
    Ok(times
        .into_iter()
        .zip(results)
        .filter(|(_, v)| v.is_finite())
        .collect())
}

/// Work out `expr` at every one of `times`, given each signal's value at
/// those times. Comparisons give 1 or 0. Anything undefined gives NaN.
pub fn eval_series(expr: &Expr, times: &[f64], values: &HashMap<&str, Vec<f64>>) -> Vec<f64> {
    let n = times.len();
    match expr {
        Expr::Number(x) => vec![*x; n],
        Expr::Signal(name, _) => values.get(name.as_str()).cloned().unwrap_or_else(|| vec![f64::NAN; n]),
        Expr::Neg(e) => eval_series(e, times, values).into_iter().map(|v| -v).collect(),
        Expr::Binary(op, a, b) => {
            let (a, b) = (eval_series(a, times, values), eval_series(b, times, values));
            a.into_iter().zip(b).map(|(a, b)| binary(*op, a, b)).collect()
        }
        Expr::Call(name, _, args) => {
            let args: Vec<Vec<f64>> = args.iter().map(|a| eval_series(a, times, values)).collect();
            call(name, &args, times)
        }
    }
}

fn binary(op: Op, a: f64, b: f64) -> f64 {
    let truth = |t: bool| if a.is_nan() || b.is_nan() { f64::NAN } else if t { 1.0 } else { 0.0 };
    match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => a / b,
        Op::Pow => a.powf(b),
        Op::Lt => truth(a < b),
        Op::Le => truth(a <= b),
        Op::Gt => truth(a > b),
        Op::Ge => truth(a >= b),
        Op::Eq => truth(a == b),
        Op::Ne => truth(a != b),
    }
}

/// A function over whole series. Unknown names and wrong argument counts
/// (which checking catches first) give NaN.
fn call(name: &str, args: &[Vec<f64>], times: &[f64]) -> Vec<f64> {
    let n = times.len();
    let nan = || vec![f64::NAN; n];
    let unary = |f: fn(f64) -> f64| match args {
        [x] => x.iter().map(|v| f(*v)).collect(),
        _ => nan(),
    };
    match name {
        "abs" => unary(f64::abs),
        "sqrt" => unary(f64::sqrt),
        "exp" => unary(f64::exp),
        "ln" => unary(f64::ln),
        "log10" => unary(f64::log10),
        "sin" => unary(f64::sin),
        "cos" => unary(f64::cos),
        "tan" => unary(f64::tan),
        "round" => unary(f64::round),
        "min" | "max" if args.len() >= 2 => (0..n)
            .map(|i| {
                let column = args.iter().map(|a| a[i]);
                if args.iter().any(|a| a[i].is_nan()) {
                    f64::NAN
                } else if name == "min" {
                    column.fold(f64::INFINITY, f64::min)
                } else {
                    column.fold(f64::NEG_INFINITY, f64::max)
                }
            })
            .collect(),
        "clamp" => match args {
            [x, lo, hi] => (0..n)
                .map(|i| {
                    let (x, lo, hi) = (x[i], lo[i], hi[i]);
                    if x.is_nan() || lo.is_nan() || hi.is_nan() || lo > hi { f64::NAN } else { x.clamp(lo, hi) }
                })
                .collect(),
            _ => nan(),
        },
        "if" => match args {
            [c, a, b] => (0..n)
                .map(|i| if c[i].is_nan() { f64::NAN } else if c[i] != 0.0 { a[i] } else { b[i] })
                .collect(),
            _ => nan(),
        },
        "derivative" => match args {
            [x] => derivative(x, times),
            _ => nan(),
        },
        "integral" => match args {
            [x] => integral(x, times),
            _ => nan(),
        },
        "lowpass" => match args {
            [x, tau] => lowpass(x, tau, times),
            _ => nan(),
        },
        _ => nan(),
    }
}

/// Change per second from the previous point. The first point has none (NaN).
fn derivative(x: &[f64], times: &[f64]) -> Vec<f64> {
    (0..x.len())
        .map(|i| {
            if i == 0 {
                return f64::NAN;
            }
            let dt = times[i] - times[i - 1];
            if dt > 0.0 { (x[i] - x[i - 1]) / dt } else { f64::NAN }
        })
        .collect()
}

/// Running total by the trapezoid rule, starting at 0. A step with an
/// undefined end adds nothing.
fn integral(x: &[f64], times: &[f64]) -> Vec<f64> {
    let mut total = 0.0;
    (0..x.len())
        .map(|i| {
            if i > 0 {
                let step = (x[i] + x[i - 1]) / 2.0 * (times[i] - times[i - 1]);
                if step.is_finite() {
                    total += step;
                }
            }
            total
        })
        .collect()
}

/// First-order low-pass filter with time constant `tau` seconds, which
/// handles uneven spacing. It starts at the first defined value; a time
/// constant of 0 or less passes x through.
fn lowpass(x: &[f64], tau: &[f64], times: &[f64]) -> Vec<f64> {
    let mut state: Option<f64> = None;
    (0..x.len())
        .map(|i| {
            if x[i].is_nan() {
                return state.unwrap_or(f64::NAN);
            }
            let y = match state {
                None => x[i],
                Some(prev) if tau[i] > 0.0 => {
                    let dt = times[i] - times[i - 1];
                    prev + (1.0 - (-dt / tau[i]).exp()) * (x[i] - prev)
                }
                Some(_) => x[i],
            };
            state = Some(y);
            y
        })
        .collect()
}

/// The value of `samples` at time `t`, where `i` is its last sample at or before `t`.
fn read(samples: &[(f64, f64)], i: usize, t: f64, interpolation: Interpolation) -> f64 {
    let (t0, v0) = samples[i];
    match (interpolation, samples.get(i + 1)) {
        (Interpolation::Linear, Some(&(t1, v1))) if t1 > t0 && t >= t0 => {
            v0 + (v1 - v0) * (t - t0) / (t1 - t0)
        }
        _ => v0,
    }
}

/// Every sample time of every input from `start` on, sorted, without repeats.
fn union_times(series: &[&[(f64, f64)]], start: f64) -> Vec<f64> {
    let mut times: Vec<f64> = series
        .iter()
        .flat_map(|s| s.iter().map(|(t, _)| *t))
        .filter(|t| *t >= start)
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times.dedup();
    times
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::parser::parse;

    fn inputs(list: &[(&str, &[(f64, f64)])]) -> HashMap<String, Samples> {
        list.iter().map(|(n, s)| (n.to_string(), s.to_vec())).collect()
    }

    fn run(equation: &str, list: &[(&str, &[(f64, f64)])], mode: TimeMode) -> Vec<(f64, f64)> {
        evaluate(&parse(equation).unwrap(), &inputs(list), mode).unwrap()
    }

    const HOLD: TimeMode = TimeMode::SampleAndHold;

    #[test]
    fn same_message_lines_up_exactly() {
        let a: &[(f64, f64)] = &[(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)];
        let b: &[(f64, f64)] = &[(0.0, 10.0), (1.0, 20.0), (2.0, 30.0)];
        assert_eq!(run("a * b", &[("a", a), ("b", b)], HOLD), [(0.0, 10.0), (1.0, 40.0), (2.0, 90.0)]);
    }

    #[test]
    fn sample_and_hold_different_rates_and_late_start() {
        // a every second from 0; b starts at 1.5.
        let a: &[(f64, f64)] = &[(0.0, 1.0), (1.0, 2.0), (2.0, 3.0), (3.0, 4.0)];
        let b: &[(f64, f64)] = &[(1.5, 10.0), (2.5, 20.0)];
        assert_eq!(
            run("a + b", &[("a", a), ("b", b)], HOLD),
            [(1.5, 12.0), (2.0, 13.0), (2.5, 23.0), (3.0, 24.0)]
        );
    }

    #[test]
    fn repeated_names_count_once() {
        let a: &[(f64, f64)] = &[(0.0, 3.0)];
        assert_eq!(run("a * a + a", &[("a", a)], HOLD), [(0.0, 12.0)]);
    }

    #[test]
    fn resample_hold_and_linear() {
        let a: &[(f64, f64)] = &[(0.0, 0.0), (1.0, 10.0)];
        let hold = TimeMode::Resample { rate_hz: 4.0, interpolation: Interpolation::Hold };
        let linear = TimeMode::Resample { rate_hz: 4.0, interpolation: Interpolation::Linear };
        assert_eq!(
            run("a", &[("a", a)], hold),
            [(0.0, 0.0), (0.25, 0.0), (0.5, 0.0), (0.75, 0.0), (1.0, 10.0)]
        );
        assert_eq!(
            run("a", &[("a", a)], linear),
            [(0.0, 0.0), (0.25, 2.5), (0.5, 5.0), (0.75, 7.5), (1.0, 10.0)]
        );
    }

    #[test]
    fn resample_grid_starts_when_all_inputs_have_values_and_holds_past_last_sample() {
        let a: &[(f64, f64)] = &[(0.0, 1.0), (2.0, 3.0)];
        let b: &[(f64, f64)] = &[(1.0, 100.0)];
        let linear = TimeMode::Resample { rate_hz: 2.0, interpolation: Interpolation::Linear };
        // Grid 1.0, 1.5, 2.0; b holds 100 after its only sample.
        assert_eq!(
            run("a + b", &[("a", a), ("b", b)], linear),
            [(1.0, 102.0), (1.5, 102.5), (2.0, 103.0)]
        );
    }

    #[test]
    fn non_finite_results_are_dropped() {
        let a: &[(f64, f64)] = &[(0.0, 1.0), (1.0, 0.0), (2.0, 2.0)];
        assert_eq!(run("1 / a", &[("a", a)], HOLD), [(0.0, 1.0), (2.0, 0.5)]);
    }

    #[test]
    fn functions() {
        let a: &[(f64, f64)] = &[(0.0, -4.0), (1.0, 9.0)];
        let b: &[(f64, f64)] = &[(0.0, 2.0), (1.0, 2.0)];
        let both = [("a", a), ("b", b)];
        assert_eq!(run("abs(a)", &both, HOLD), [(0.0, 4.0), (1.0, 9.0)]);
        // sqrt(-4) is NaN, so that point is dropped.
        assert_eq!(run("sqrt(a)", &both, HOLD), [(1.0, 3.0)]);
        assert_eq!(run("max(a, b, 0)", &both, HOLD), [(0.0, 2.0), (1.0, 9.0)]);
        assert_eq!(run("min(a, b)", &both, HOLD), [(0.0, -4.0), (1.0, 2.0)]);
        assert_eq!(run("clamp(a, 0, 5)", &both, HOLD), [(0.0, 0.0), (1.0, 5.0)]);
        assert_eq!(run("if(a > b, 1, -1)", &both, HOLD), [(0.0, -1.0), (1.0, 1.0)]);
        assert_eq!(run("round(a / b)", &both, HOLD), [(0.0, -2.0), (1.0, 5.0)]);
    }

    #[test]
    fn derivative_and_integral() {
        let a: &[(f64, f64)] = &[(0.0, 0.0), (1.0, 2.0), (3.0, 2.0), (4.0, 0.0)];
        assert_eq!(run("derivative(a)", &[("a", a)], HOLD), [(1.0, 2.0), (3.0, 0.0), (4.0, -2.0)]);
        assert_eq!(run("integral(a)", &[("a", a)], HOLD), [(0.0, 0.0), (1.0, 1.0), (3.0, 5.0), (4.0, 6.0)]);
    }

    #[test]
    fn lowpass_approaches_a_step() {
        // A step from 0 to 1 at t = 0.1, resampled at 100 Hz, time constant 0.5 s.
        let a: &[(f64, f64)] = &[(0.0, 0.0), (0.1, 1.0), (5.0, 1.0)];
        let mode = TimeMode::Resample { rate_hz: 100.0, interpolation: Interpolation::Hold };
        let out = run("lowpass(a, 0.5)", &[("a", a)], mode);
        let at = |t: f64| out.iter().find(|(time, _)| (time - t).abs() < 1e-9).unwrap().1;
        assert_eq!(at(0.0), 0.0);
        // One time constant after the step: 1 - 1/e.
        assert!((at(0.6) - (1.0 - (-1.0f64).exp())).abs() < 0.01, "{}", at(0.6));
        assert!(at(5.0) > 0.999);
        // A time constant of 0 passes the input through.
        assert_eq!(run("lowpass(a, 0)", &[("a", a)], HOLD), [(0.0, 0.0), (0.1, 1.0), (5.0, 1.0)]);
    }

    #[test]
    fn errors() {
        let expr = parse("a + b").unwrap();
        let a: &[(f64, f64)] = &[(0.0, 1.0)];
        assert_eq!(
            evaluate(&expr, &inputs(&[("a", a)]), HOLD).unwrap_err(),
            "'b' has no data in this log"
        );
        assert_eq!(
            evaluate(&parse("1 + 2").unwrap(), &inputs(&[]), HOLD).unwrap_err(),
            "The equation uses no signals"
        );
        let bad_rate = TimeMode::Resample { rate_hz: 0.0, interpolation: Interpolation::Hold };
        assert!(evaluate(&parse("a").unwrap(), &inputs(&[("a", a)]), bad_rate).is_err());
        let huge = TimeMode::Resample { rate_hz: 1e9, interpolation: Interpolation::Hold };
        let long: &[(f64, f64)] = &[(0.0, 1.0), (100.0, 1.0)];
        assert!(evaluate(&parse("a").unwrap(), &inputs(&[("a", long)]), huge).is_err());
    }
}
