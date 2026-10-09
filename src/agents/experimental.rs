//! The opt-in Mod's installation and its in-process snapshot transport.

use super::{store::Activity, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

pub const PLUGIN_NAME: &str = "ccline-agents-mod";
pub const PLUGIN_KEY: &str = "ccline-agents-mod@skills-dir";
pub const ENABLE_ENV: &str = "CLAUDE_CODE_ENABLE_FUNCTION_HOOKS";
pub const SNAPSHOT_ENV: &str = "CCLINE_AGENTS_SNAPSHOT";

/// Ship the complete plugin with the binary; users need no Node or package installer.
pub fn install(claude_dir: &Path) -> Result<()> {
    let root = claude_dir.join("skills").join(PLUGIN_NAME);
    for (path, content) in [
        (
            ".claude-plugin/plugin.json",
            include_str!("../../mods/agents/.claude-plugin/plugin.json"),
        ),
        (
            "hooks/hooks.json",
            include_str!("../../mods/agents/hooks/hooks.json"),
        ),
        (
            "hooks/register.ts",
            include_str!("../../mods/agents/hooks/register.ts"),
        ),
        (
            "hooks/activity.ts",
            include_str!("../../mods/agents/hooks/activity.ts"),
        ),
    ] {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().ok_or("Missing Mod asset parent")?)?;
        std::fs::write(path, content)?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct Snapshot {
    sessions: BTreeMap<String, Activity>,
}

/// Missing or invalid Mod data hides Agents without interrupting the status line.
pub fn parse_snapshot(snapshot: &str, session: &str) -> Option<Activity> {
    let mut snapshot: Snapshot = serde_json::from_str(snapshot).ok()?;
    snapshot.sessions.remove(session)
}

pub fn read_snapshot(session: &str) -> Option<Activity> {
    let snapshot = std::env::var(SNAPSHOT_ENV).ok()?;
    parse_snapshot(&snapshot, session)
}
