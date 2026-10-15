//! Install/uninstall the `subagentStatusLine` setting owned by the Agents segment.
//!
//! ccline records the command it wrote in a manifest and only ever removes that exact value, so a
//! user's own panel command, or a later edit to ccline's, is never overwritten or deleted.

use super::Result;
use crate::config::paths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const PANEL_SETTING: &str = "subagentStatusLine";
const MANIFEST: &str = "agents-installation.json";

#[derive(Debug, Deserialize, Serialize)]
struct Installation {
    /// The panel command ccline wrote to `subagentStatusLine`.
    subagent_status_line: String,
}

fn read_json(path: &Path) -> Result<Option<Value>> {
    paths::read_optional(path)?
        .map(|content| {
            serde_json::from_str(&content)
                .map_err(|error| format!("Invalid {}: {error}", path.display()).into())
        })
        .transpose()
}

/// The `subagentStatusLine` value ccline installs for `command`.
fn panel_setting(command: &str) -> Value {
    json!({ "type": "command", "command": command })
}

/// `executable --subagent-statusline`, quoted for the shell Claude Code runs commands in.
fn panel_command(executable: &Path) -> Result<String> {
    let path = executable.to_str().ok_or("Executable path is not UTF-8")?;
    // Forward slashes also work with Git Bash on Windows.
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    Ok(format!(
        "'{}' --subagent-statusline",
        path.replace('\'', "'\\''")
    ))
}

/// Install or remove the panel command in the configuration directory Claude Code uses.
pub fn configure(enabled: bool) -> Result<String> {
    configure_at(&paths::claude_dir()?, &std::env::current_exe()?, enabled)?;
    Ok(if enabled {
        format!("Subagent panel: installed {PANEL_SETTING}. Restart Claude Code to apply.")
    } else {
        "Agents disabled".into()
    })
}

pub fn configure_at(claude_dir: &Path, executable: &Path, enabled: bool) -> Result<()> {
    let manifest_path = paths::ccline_dir(claude_dir).join(MANIFEST);
    let settings_path = paths::settings_path(claude_dir);
    let previous: Option<Installation> = read_json(&manifest_path)?
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| format!("Invalid {}: {error}", manifest_path.display()))
        })
        .transpose()?;
    // Saving an unrelated configuration with Agents disabled has no integration side effects.
    if previous.is_none() && !enabled {
        return Ok(());
    }

    let original = read_json(&settings_path)?.unwrap_or_else(|| json!({}));
    let mut settings = original.clone();
    let object = settings
        .as_object_mut()
        .ok_or_else(|| format!("{} must be a JSON object", settings_path.display()))?;
    let owned = previous
        .as_ref()
        .map(|installed| panel_setting(&installed.subagent_status_line));
    let current = object.get(PANEL_SETTING);

    let installation = if enabled {
        let command = panel_command(executable)?;
        let setting = panel_setting(&command);
        // ccline's recorded value, or exactly what it would write, is ccline's to replace.
        if let Some(current) =
            current.filter(|current| Some(*current) != owned.as_ref() && **current != setting)
        {
            return Err(format!(
                "Cannot enable Agents: {PANEL_SETTING} in {} is already set to {current} by \
                 someone other than ccline. Remove it to let ccline render the subagent panel, \
                 or keep it and leave Agents disabled.",
                settings_path.display()
            )
            .into());
        }
        object.insert(PANEL_SETTING.into(), setting);
        Some(Installation {
            subagent_status_line: command,
        })
    } else {
        if current.is_some() && current == owned.as_ref() {
            object.remove(PANEL_SETTING);
        }
        None
    };

    if settings != original {
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut content = serde_json::to_string_pretty(&settings)?;
        content.push('\n');
        fs::write(&settings_path, content)?;
    }
    match installation {
        Some(installation) => {
            if let Some(parent) = manifest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&manifest_path, serde_json::to_vec_pretty(&installation)?)?;
        }
        None => fs::remove_file(&manifest_path)?,
    }
    Ok(())
}
