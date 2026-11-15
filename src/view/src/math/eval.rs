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
//! Output starts once every input has a value. Points whose result is NaN or
//! infinite (division by zero, ...) are dropped.

use super::parser::Expr;
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

    // Walk every input forward together; `cursor[k]` is the last sample of
    // input k at or before the current time.
    let mut cursor = vec![0usize; series.len()];
    let mut values: HashMap<&str, f64> = HashMap::with_capacity(names.len());
    let mut out = Vec::with_capacity(times.len());
    for t in times {
        for (k, samples) in series.iter().enumerate() {
            while cursor[k] + 1 < samples.len() && samples[cursor[k] + 1].0 <= t {
                cursor[k] += 1;
            }
            values.insert(names[k], read(samples, cursor[k], t, interpolation));
        }
        let value = expr.eval(&|name| values.get(name).copied().unwrap_or(f64::NAN));
        if value.is_finite() {
            out.push((t, value));
        }
    }
    Ok(out)
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
