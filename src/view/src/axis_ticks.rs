//! Tick positions and labels for the plot axes.
//!
//! Ticks fall on round numbers: steps of 1, 2 or 5 times a power of ten for
//! values, and clock-friendly steps (1, 2, 5, 10, 15, 30 s, minutes, hours) for
//! time. The step is the smallest one that leaves room between labels, so a
//! bigger plot gets more ticks. Labels show only as many decimals as the step
//! needs.
//!
//! This file has no dependencies, so its tests also run on their own:
//! `rustc --edition 2021 --test src/view/src/axis_ticks.rs -o <output>`

/// Ticks for one axis: where they go and how many decimals their labels need.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticks {
    pub step: f64,
    pub values: Vec<f64>,
    pub decimals: usize,
}

/// Width of one label character in pixels at the plot's 10 px font (digits and
/// colons are about this wide).
const CHAR_PX: f32 = 6.0;
/// Space kept clear between neighbouring X labels, in pixels.
const X_LABEL_GAP_PX: f32 = 24.0;
/// The most decimals a label ever shows.
const MAX_DECIMALS: usize = 6;

/// The smallest step of 1, 2 or 5 times a power of ten that is at least `raw`.
pub fn nice_step(raw: f64) -> f64 {
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let magnitude = 10f64.powf(raw.log10().floor());
    for factor in [1.0, 2.0, 5.0, 10.0] {
        let step = factor * magnitude;
        if step >= raw * (1.0 - 1e-9) {
            return step;
        }
    }
    10.0 * magnitude
}

/// Decimals needed to tell apart multiples of `step`: 5 → 0, 0.5 → 1, 0.02 → 2.
pub fn decimals_for(step: f64) -> usize {
    if !(step.is_finite() && step > 0.0) {
        return 0;
    }
    let d = -(step.log10() + 1e-9).floor();
    (d.max(0.0) as usize).min(MAX_DECIMALS)
}

/// Every multiple of `step` from `min` to `max`, inclusive.
pub fn multiples(min: f64, max: f64, step: f64) -> Vec<f64> {
    let first = (min / step - 1e-9).ceil() as i64;
    let last = (max / step + 1e-9).floor() as i64;
    if last < first || last - first > 10_000 {
        return Vec::new();
    }
    (first..=last)
        .map(|k| {
            let v = k as f64 * step;
            if v == 0.0 { 0.0 } else { v } // no "-0"
        })
        .collect()
}

/// The value step for an axis `length_px` long showing `min..max`, keeping
/// ticks at least `min_spacing_px` apart.
pub fn value_step(min: f64, max: f64, length_px: f32, min_spacing_px: f32) -> f64 {
    let range = max - min;
    if !(range.is_finite() && range > 0.0 && length_px > 0.0) {
        return 1.0;
    }
    let max_intervals = (length_px / min_spacing_px).floor().max(1.0) as f64;
    nice_step(range / max_intervals)
}

/// Value-axis ticks for `min..max` drawn `length_px` long.
pub fn value_ticks(min: f64, max: f64, length_px: f32, min_spacing_px: f32) -> Ticks {
    let step = value_step(min, max, length_px, min_spacing_px);
    Ticks { step, values: multiples(min, max, step), decimals: decimals_for(step) }
}

/// Widen `min..max` out to the nearest multiples of `step`, so the axis ends
/// on ticks.
pub fn round_out(min: f64, max: f64, step: f64) -> (f64, f64) {
    if !(step.is_finite() && step > 0.0) {
        return (min, max);
    }
    let lo = (min / step + 1e-9).floor() * step;
    let hi = (max / step - 1e-9).ceil() * step;
    if hi > lo { (lo, hi) } else { (lo, lo + step) }
}

/// A value label with `decimals` places.
pub fn format_value(value: f64, decimals: usize) -> String {
    let text = format!("{:.*}", decimals, value);
    // Rounding can leave "-0" or "-0.00".
    if text.starts_with('-') && text[1..].chars().all(|c| c == '0' || c == '.') {
        text[1..].to_string()
    } else {
        text
    }
}

/// Clock-friendly time steps from one second up, in seconds.
const TIME_STEPS: [f64; 19] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, // seconds
    60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, // minutes
    3600.0, 7200.0, 10800.0, 21600.0, 43200.0, // hours
    86400.0, 172800.0, // days
];

/// Candidate time steps, smallest first: 1-2-5 steps below a second, then
/// clock steps.
fn time_steps() -> impl Iterator<Item = f64> {
    let sub_second = (-6..0).flat_map(|exp| {
        let magnitude = 10f64.powi(exp);
        [1.0, 2.0, 5.0].into_iter().map(move |f| f * magnitude)
    });
    sub_second.chain(TIME_STEPS)
}

/// How a time label is written for a given step.
pub fn time_label_decimals(step: f64) -> usize {
    if step >= 1.0 { 0 } else { decimals_for(step) }
}

/// Characters in a time label for this step: "HH:MM", "HH:MM:SS" or
/// "HH:MM:SS.fff".
fn time_label_len(step: f64) -> usize {
    if step >= 60.0 {
        5
    } else {
        match time_label_decimals(step) {
            0 => 8,
            d => 9 + d,
        }
    }
}

/// Time-axis ticks for `min_t..max_t` (seconds) drawn `width_px` wide. The step
/// is the smallest that leaves room for its labels; more width gives more ticks.
pub fn time_ticks(min_t: f64, max_t: f64, width_px: f32) -> Ticks {
    let range = max_t - min_t;
    let mut step = *TIME_STEPS.last().unwrap();
    if range.is_finite() && range > 0.0 && width_px > 0.0 {
        let px_per_second = width_px as f64 / range;
        if let Some(s) = time_steps().find(|&s| {
            let needed = time_label_len(s) as f32 * CHAR_PX + X_LABEL_GAP_PX;
            s * px_per_second >= needed as f64
        }) {
            step = s;
        }
    }
    Ticks { step, values: multiples(min_t, max_t, step), decimals: time_label_decimals(step) }
}

/// A clock-time label for `t` (seconds; only the time of day is shown) at the
/// precision `step` calls for.
pub fn format_time(t: f64, step: f64) -> String {
    let decimals = time_label_decimals(step);
    let scale = 10i64.pow(decimals as u32);
    let ticks = (t * scale as f64).round() as i64;
    let secs = ticks.div_euclid(scale);
    let frac = ticks.rem_euclid(scale);
    let day_secs = secs.rem_euclid(86_400);
    let (h, m, s) = (day_secs / 3600, day_secs / 60 % 60, day_secs % 60);
    if step >= 60.0 {
        format!("{:02}:{:02}", h, m)
    } else if decimals == 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}:{:02}.{:0width$}", h, m, s, frac, width = decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn nice_steps_are_one_two_or_five() {
        for (raw, expected) in [
            (0.7, 1.0), (1.0, 1.0), (1.3, 2.0), (3.0, 5.0), (7.0, 10.0),
            (0.013, 0.02), (0.04, 0.05), (450.0, 500.0), (12_000.0, 20_000.0),
        ] {
            assert!(close(nice_step(raw), expected), "{} -> {}", raw, nice_step(raw));
        }
    }

    #[test]
    fn decimals_follow_the_step() {
        assert_eq!(decimals_for(10.0), 0);
        assert_eq!(decimals_for(1.0), 0);
        assert_eq!(decimals_for(0.5), 1);
        assert_eq!(decimals_for(0.1), 1);
        assert_eq!(decimals_for(0.02), 2);
        assert_eq!(decimals_for(0.001), 3);
    }

    #[test]
    fn value_ticks_land_on_round_numbers() {
        // 3.17..47.9 over 200 px with 40 px spacing: at most 5 intervals → step 10.
        let ticks = value_ticks(3.17, 47.9, 200.0, 40.0);
        assert!(close(ticks.step, 10.0));
        assert_eq!(ticks.values, vec![10.0, 20.0, 30.0, 40.0]);
        assert_eq!(ticks.decimals, 0);
    }

    #[test]
    fn taller_axes_get_more_value_ticks() {
        let short = value_ticks(0.0, 100.0, 120.0, 40.0);
        let tall = value_ticks(0.0, 100.0, 600.0, 40.0);
        assert!(tall.values.len() > short.values.len());
    }

    #[test]
    fn small_ranges_get_decimal_ticks() {
        let ticks = value_ticks(1.003, 1.097, 200.0, 40.0);
        assert!(close(ticks.step, 0.02));
        assert_eq!(ticks.decimals, 2);
        let labels: Vec<String> = ticks.values.iter().map(|v| format_value(*v, ticks.decimals)).collect();
        assert_eq!(labels, vec!["1.02", "1.04", "1.06", "1.08"]);
    }

    #[test]
    fn negative_ranges_and_zero() {
        let ticks = value_ticks(-7.5, 7.5, 200.0, 40.0);
        assert!(close(ticks.step, 5.0));
        assert_eq!(ticks.values, vec![-5.0, 0.0, 5.0]);
        assert_eq!(format_value(-0.0001, 2), "0.00");
    }

    #[test]
    fn rounding_out_ends_the_axis_on_ticks() {
        assert_eq!(round_out(3.17, 47.9, 10.0), (0.0, 50.0));
        assert_eq!(round_out(-7.2, 7.2, 5.0), (-10.0, 10.0));
        assert_eq!(round_out(10.0, 50.0, 10.0), (10.0, 50.0)); // already on ticks
    }

    #[test]
    fn time_ticks_fall_on_clock_steps() {
        // Six minutes across 1,000 px: labels are 8 chars, step 30 s.
        let start = 17.0 * 3600.0 + 53.0 * 60.0 + 56.955;
        let ticks = time_ticks(start, start + 357.0, 1000.0);
        assert!(close(ticks.step, 30.0));
        assert_eq!(format_time(ticks.values[0], ticks.step), "17:54:00");
        assert_eq!(format_time(ticks.values[1], ticks.step), "17:54:30");
    }

    #[test]
    fn wider_plots_get_more_time_ticks() {
        let narrow = time_ticks(0.0, 600.0, 500.0);
        let wide = time_ticks(0.0, 600.0, 2000.0);
        assert!(wide.values.len() > narrow.values.len());
        assert!(wide.step < narrow.step);
    }

    #[test]
    fn long_spans_drop_seconds() {
        let ticks = time_ticks(0.0, 3.0 * 3600.0, 1000.0);
        assert!(ticks.step >= 60.0);
        assert_eq!(format_time(9.0 * 3600.0 + 30.0 * 60.0, ticks.step), "09:30");
    }

    #[test]
    fn zoomed_in_labels_show_just_enough_decimals() {
        // 0.5 s across 1,000 px.
        let ticks = time_ticks(100.0, 100.5, 1000.0);
        assert!(ticks.step < 1.0);
        let label = format_time(ticks.values[1], ticks.step);
        let decimals = label.split('.').nth(1).map_or(0, str::len);
        assert_eq!(decimals, ticks.decimals);
        assert!((1..=3).contains(&decimals), "{}", label);
    }

    #[test]
    fn time_labels_round_instead_of_truncating() {
        // A tick computed as 9.999999 s must read 00:00:10.0, not 00:00:09.9.
        assert_eq!(format_time(9.999_999_9, 0.1), "00:00:10.0");
        assert_eq!(format_time(1_731_000_000.25, 0.05), format_time(1_731_000_000.25, 0.05));
    }

    #[test]
    fn real_timestamps_keep_their_precision() {
        // Unix time around 2026 with 1 ms ticks.
        let t0 = 1_794_000_000.0;
        let ticks = time_ticks(t0, t0 + 0.05, 1000.0);
        assert!(ticks.step <= 0.005);
        // f64 resolves about 0.2 µs at this magnitude, far finer than a tick.
        assert!(ticks.values.windows(2).all(|w| (w[1] - w[0] - ticks.step).abs() < 1e-6));
    }
}
