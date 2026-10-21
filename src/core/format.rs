use chrono::{DateTime, Duration, Utc};
use std::borrow::Cow;

/// Ratatui text has no hyperlink metadata. Remove OSC 8 wrappers while preserving their labels
/// before ANSI conversion, whose OSC parser only understands BEL-terminated sequences.
pub(crate) fn without_hyperlinks(text: &str) -> Cow<'_, str> {
    static LINKS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    LINKS
        .get_or_init(|| {
            regex::Regex::new(r"\x1b\]8;[^\x07\x1b]*(?:\x07|\x1b\\)")
                .expect("valid OSC 8 expression")
        })
        .replace_all(text, "")
}

/// Drop control characters so host-provided text cannot break a rendered row.
pub(crate) fn printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

/// A future reset or expiry in days, hours, or minutes; unknown or elapsed times use `?`.
pub(crate) fn format_reset_time(reset_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> String {
    let Some(reset_at) = reset_at else {
        return "?".to_string();
    };
    let remaining = reset_at.signed_duration_since(now);
    if remaining <= Duration::zero() {
        "?".to_string()
    } else if remaining.num_days() > 0 {
        format!("{}d", remaining.num_days())
    } else if remaining.num_hours() > 0 {
        format!("{}h", remaining.num_hours())
    } else {
        format!("{}m", remaining.num_minutes())
    }
}
