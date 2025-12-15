//! TUI module for polyrev
//!
//! Provides a terminal user interface for:
//! - Viewing and managing review findings
//! - Monitoring planning perspectives
//! - Approving unified plans before execution

mod app;
mod views;
mod widgets;

pub use app::{run_tui, TuiConfig};

use std::sync::OnceLock;

/// Cached local timezone offset, computed before multi-threading starts.
static LOCAL_OFFSET: OnceLock<time::UtcOffset> = OnceLock::new();

/// Initialize the local timezone offset. Must be called from the main thread
/// before any other threads are spawned (the time crate cannot safely determine
/// local offset in multi-threaded programs).
pub fn init_local_offset() {
    LOCAL_OFFSET.get_or_init(|| {
        // time crate removed current_local_offset in recent versions
        // Fall back to UTC as a safe default
        time::UtcOffset::UTC
    });
}

#[allow(dead_code)]
fn get_local_offset() -> time::UtcOffset {
    *LOCAL_OFFSET.get().unwrap_or(&time::UtcOffset::UTC)
}

/// Format millisecond timestamp as human-readable date/time
#[allow(dead_code)]
pub fn format_timestamp(ms: i64) -> String {
    use time::OffsetDateTime;
    let secs = ms / 1000;
    OffsetDateTime::from_unix_timestamp(secs)
        .map(|dt| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                dt.year(),
                dt.month() as u8,
                dt.day(),
                dt.hour(),
                dt.minute()
            )
        })
        .unwrap_or_else(|_| ms.to_string())
}

/// Format millisecond timestamp as short human-friendly format (e.g., "Nov 25 3:32pm")
#[allow(dead_code)]
pub fn format_timestamp_short(ms: i64) -> String {
    use time::OffsetDateTime;
    let secs = ms / 1000;
    OffsetDateTime::from_unix_timestamp(secs)
        .map(|dt| {
            let local_dt = dt.to_offset(get_local_offset());
            let month = match local_dt.month() {
                time::Month::January => "Jan",
                time::Month::February => "Feb",
                time::Month::March => "Mar",
                time::Month::April => "Apr",
                time::Month::May => "May",
                time::Month::June => "Jun",
                time::Month::July => "Jul",
                time::Month::August => "Aug",
                time::Month::September => "Sep",
                time::Month::October => "Oct",
                time::Month::November => "Nov",
                time::Month::December => "Dec",
            };
            let hour = local_dt.hour();
            let (hour12, ampm) = if hour == 0 {
                (12, "am")
            } else if hour < 12 {
                (hour, "am")
            } else if hour == 12 {
                (12, "pm")
            } else {
                (hour - 12, "pm")
            };
            format!(
                "{} {} {}:{:02}{}",
                month,
                local_dt.day(),
                hour12,
                local_dt.minute(),
                ampm
            )
        })
        .unwrap_or_else(|_| ms.to_string())
}

/// Format duration as human-readable string
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Ellipsize text to fit within max_chars
pub fn ellipsize(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else if max_chars == 0 {
        String::new()
    } else {
        let take = max_chars.saturating_sub(1);
        let mut result = value.chars().take(take).collect::<String>();
        result.push('…');
        result
    }
}

/// Sanitize text by removing newlines for single-line display
pub fn sanitize_text(value: &str) -> String {
    value.replace('\n', " ").replace('\r', " ")
}

/// Simple word-wrap for text
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let clean = sanitize_text(text);

    for paragraph in clean.split('\n') {
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
