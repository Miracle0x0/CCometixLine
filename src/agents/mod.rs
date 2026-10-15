//! Claude Code's subagent panel: ccline renders its rows and owns the `subagentStatusLine`
//! setting that runs it.

pub mod integration;
pub mod panel;

#[cfg(test)]
mod tests;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Drop control characters so host-provided text cannot break a rendered row.
pub fn printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

/// Elapsed seconds as `42s` or `1m05s`.
pub fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    }
}

/// Seconds since the Unix epoch.
pub fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}
