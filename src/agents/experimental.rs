//! Opt-in Mod selection and its in-process snapshot transport.

use super::{store::Activity, Result};
use semver::Version;
use serde::Deserialize;
use std::{collections::BTreeMap, io::ErrorKind, path::Path, process::Command};

pub const MIN_VERSION: &str = "2.1.273";
pub const PLUGIN_NAME: &str = "ccline-agents-mod";
pub const PLUGIN_KEY: &str = "ccline-agents-mod@skills-dir";
pub const ENABLE_ENV: &str = "CLAUDE_CODE_ENABLE_FUNCTION_HOOKS";
pub const SNAPSHOT_ENV: &str = "CCLINE_AGENTS_SNAPSHOT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Hooks,
    Mod,
}

pub struct Selection {
    pub backend: Backend,
    pub message: String,
}

pub fn select(version: Option<&str>) -> Result<Selection> {
    let Some(output) = version else {
        return Ok(Selection {
            backend: Backend::Hooks,
            message: "Hooks only: claude was not found".into(),
        });
    };
    let version = Version::parse(
        output
            .split_whitespace()
            .next()
            .ok_or("Empty Claude Code version")?,
    )?;
    if version < Version::parse(MIN_VERSION)? {
        return Ok(Selection {
            backend: Backend::Hooks,
            message: format!("Hooks only: Claude Code {version}; Mod requires {MIN_VERSION}+"),
        });
    }
    Ok(Selection {
        backend: Backend::Mod,
        message: format!("Experimental Mod: Claude Code {version}. Restart Claude Code to apply."),
    })
}

pub fn detect() -> Result<Selection> {
    let output = match Command::new("claude").arg("--version").output() {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return select(None),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        return Err(format!(
            "claude --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    select(Some(std::str::from_utf8(&output.stdout)?))
}

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

pub fn parse_snapshot(snapshot: &str, session: &str) -> Result<Activity> {
    let mut snapshot: Snapshot = serde_json::from_str(snapshot)?;
    snapshot.sessions.remove(session).ok_or_else(|| format!("Agents Mod has no snapshot for session {session}; restart Claude Code or disable Experimental Mod and save").into())
}

pub fn read_snapshot(session: &str) -> Result<Activity> {
    let snapshot = std::env::var(SNAPSHOT_ENV).map_err(|error| format!("Agents Mod snapshot is unavailable ({error}); restart Claude Code or disable Experimental Mod and save"))?;
    parse_snapshot(&snapshot, session)
}
