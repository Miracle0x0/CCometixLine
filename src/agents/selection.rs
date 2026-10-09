//! Choose how subagent activity reaches the user from the installed Claude Code version.

use super::Result;
use semver::Version;
use std::{io::ErrorKind, process::Command};

/// `subagentStatusLine` tasks carry `agentType` from this release.
pub const PANEL_MIN_VERSION: &str = "2.1.293";
/// Oldest release whose function hook declarations the Mod was written against.
pub const MOD_MIN_VERSION: &str = "2.1.273";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Lifecycle command hooks feed the inline Agents segment.
    Hooks,
    /// The experimental Mod feeds the inline Agents segment.
    Mod,
    /// Claude Code's subagent panel shows the agents; ccline renders its rows.
    Panel,
}

pub struct Selection {
    pub backend: Backend,
    pub message: String,
}

pub fn select(version: Option<&str>, experimental_mod: bool) -> Result<Selection> {
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
    if version >= Version::parse(PANEL_MIN_VERSION)? {
        let note = if experimental_mod {
            format!(" Experimental Mod applies below {PANEL_MIN_VERSION}.")
        } else {
            String::new()
        };
        return Ok(Selection {
            backend: Backend::Panel,
            message: format!(
                "Subagent panel: Claude Code {version}.{note} Restart Claude Code to apply."
            ),
        });
    }
    if !experimental_mod {
        return Ok(Selection {
            backend: Backend::Hooks,
            message: format!(
                "Hooks only: Claude Code {version}; the subagent panel requires {PANEL_MIN_VERSION}+"
            ),
        });
    }
    if version < Version::parse(MOD_MIN_VERSION)? {
        return Ok(Selection {
            backend: Backend::Hooks,
            message: format!("Hooks only: Claude Code {version}; Mod requires {MOD_MIN_VERSION}+"),
        });
    }
    Ok(Selection {
        backend: Backend::Mod,
        message: format!("Experimental Mod: Claude Code {version}. Restart Claude Code to apply."),
    })
}

pub fn detect(experimental_mod: bool) -> Result<Selection> {
    let output = match Command::new("claude").arg("--version").output() {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return select(None, experimental_mod),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        return Err(format!(
            "claude --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    select(Some(std::str::from_utf8(&output.stdout)?), experimental_mod)
}
