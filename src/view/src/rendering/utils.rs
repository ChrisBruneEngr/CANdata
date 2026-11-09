//! Rendering utility functions
//!
//! This module contains pure utility functions for formatting and data transformation
//! used in rendering UI elements.

/// Format a timestamp into a human-readable string
///
/// # Arguments
/// * `timestamp` - The timestamp in nanoseconds
/// * `start_time` - Optional start time for relative timestamps
///
/// # Returns
/// A formatted timestamp string in the format "YYYY-MM-DD HH:MM:SS.mmmmmm"
/// If no start time is provided, returns the timestamp as seconds with microsecond precision
///
/// # Examples
/// ```
/// let timestamp = 1_500_000_000; // 1.5 seconds
/// let formatted = format_timestamp(timestamp, None);
/// assert_eq!(formatted, "1.500000");
/// ```
#[allow(dead_code)]
pub fn format_timestamp(timestamp: u64, _start_time: Option<chrono::NaiveDateTime>) -> String {
    // `timestamp` is the absolute Unix nanoseconds written by MergedView::from_segments
    // (abs_ns = file_start_ns + msg.relative_ns). It already includes the file's
    // measurement_start_time, so there's no need to add start_time.
    use chrono::{TimeZone, Utc};
    let dt = Utc.timestamp_nanos(timestamp as i64);
    dt.naive_utc().format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_without_start() {
        // Timestamps are absolute Unix nanoseconds, so 1.5 s after the epoch
        // prints as a full date and time.
        let result = format_timestamp(1_500_000_000, None);
        assert_eq!(result, "1970-01-01 00:00:01.500000");
    }

}
