//! Claude Code hook input and subagent activity grouped by process and session.

pub mod experimental;
pub mod integration;
pub mod panel;
pub mod selection;
pub mod store;

#[cfg(test)]
mod tests;

use serde::Deserialize;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize)]
pub struct HookInput {
    pub session_id: String,
    #[serde(flatten)]
    pub event: HookEvent,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookEvent {
    SessionStart {
        source: String,
    },
    SessionEnd {
        reason: SessionEndReason,
    },
    SubagentStart {
        agent_id: String,
        agent_type: String,
    },
    SubagentStop {
        agent_id: String,
        agent_type: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Clear,
    Resume,
    Logout,
    PromptInputExit,
    Other,
}

impl SessionEndReason {
    pub fn exits_process(&self) -> bool {
        matches!(self, Self::Logout | Self::PromptInputExit | Self::Other)
    }
}

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

pub fn claude_process_id() -> Result<u32> {
    let value = std::env::var("CLAUDE_PID")
        .map_err(|error| format!("Cannot identify the Claude Code process: CLAUDE_PID {error}"))?;
    Ok(value.parse::<std::num::NonZeroU32>()?.get())
}

pub fn data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or("Cannot locate the home directory")?;
    Ok(home.join(".claude/ccline"))
}
