//! Claude Code hook input and session-scoped subagent activity.

pub mod experimental;
pub mod integration;
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
    SessionEnd,
    SubagentStart {
        agent_id: String,
        agent_type: String,
    },
    SubagentStop {
        agent_id: String,
        agent_type: String,
    },
}

pub fn data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or("Cannot locate the home directory")?;
    Ok(home.join(".claude/ccline"))
}
