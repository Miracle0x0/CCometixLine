//! Install/uninstall only the hooks owned by the Agents segment.

use super::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

const EVENTS: [&str; 4] = [
    "SessionStart",
    "SubagentStart",
    "SubagentStop",
    "SessionEnd",
];
const REFRESH_SECONDS: u64 = 2;

#[derive(Debug, Deserialize, Serialize)]
struct Installation {
    command: String,
    added_refresh_interval: bool,
}

fn read_json(path: &Path) -> Result<Option<Value>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn remove_hooks(settings: &mut Value, command: &str) -> Result<()> {
    let Some(hooks) = settings.get_mut("hooks") else {
        return Ok(());
    };
    let hooks = hooks
        .as_object_mut()
        .ok_or("Claude settings.hooks must be an object")?;
    for event in EVENTS {
        let Some(groups) = hooks.get_mut(event) else {
            continue;
        };
        let groups = groups
            .as_array_mut()
            .ok_or("Claude hook event must be an array")?;
        let mut removed_groups = Vec::new();
        for (index, group) in groups.iter_mut().enumerate() {
            let entries = group
                .get_mut("hooks")
                .and_then(Value::as_array_mut)
                .ok_or("Claude hook group must contain a hooks array")?;
            let original_len = entries.len();
            entries.retain(|entry| entry.get("command").and_then(Value::as_str) != Some(command));
            if entries.is_empty() && original_len > 0 {
                removed_groups.push(index);
            }
        }
        let removed_any = !removed_groups.is_empty();
        for index in removed_groups.into_iter().rev() {
            groups.remove(index);
        }
        if removed_any && groups.is_empty() {
            hooks.remove(event);
        }
    }
    if hooks.is_empty() {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    Ok(())
}

fn edit_settings(
    settings: &mut Value,
    previous: Option<Installation>,
    enabled: bool,
    executable: &str,
) -> Result<Option<Installation>> {
    if !settings.is_object() {
        return Err("Claude settings must be a JSON object".into());
    }
    if let Some(ref installed) = previous {
        remove_hooks(settings, &installed.command)?;
    }
    if !enabled {
        if previous.is_some_and(|installed| installed.added_refresh_interval) {
            if let Some(statusline) = settings.get_mut("statusLine") {
                if statusline.get("refreshInterval") == Some(&json!(REFRESH_SECONDS)) {
                    statusline
                        .as_object_mut()
                        .ok_or("statusLine must be an object")?
                        .remove("refreshInterval");
                }
            }
        }
        return Ok(None);
    }

    // Command hooks use a shell. Forward slashes also work with Git Bash on Windows.
    let path = if cfg!(windows) {
        executable.replace('\\', "/")
    } else {
        executable.to_owned()
    };
    let quoted = format!("'{}'", path.replace('\'', "'\\''"));
    let command = format!("{quoted} --agents-hook");
    remove_hooks(settings, &command)?;
    let object = settings.as_object_mut().unwrap();
    let hooks = object
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("Claude settings.hooks must be an object")?;
    for event in EVENTS {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or("Claude hook event must be an array")?;
        groups.push(json!({ "hooks": [{ "type": "command", "command": command }] }));
    }

    let statusline = object
        .entry("statusLine")
        .or_insert_with(|| json!({"type": "command", "command": quoted}))
        .as_object_mut()
        .ok_or("Claude settings.statusLine must be an object")?;
    let mut added_refresh_interval =
        previous.is_some_and(|installed| installed.added_refresh_interval);
    if !statusline.contains_key("refreshInterval") {
        statusline.insert("refreshInterval".into(), json!(REFRESH_SECONDS));
        added_refresh_interval = true;
    }
    Ok(Some(Installation {
        command,
        added_refresh_interval,
    }))
}

pub fn configure(enabled: bool) -> Result<()> {
    let dir = super::data_dir()?;
    let settings_path = dir
        .parent()
        .ok_or("Missing Claude configuration directory")?
        .join("settings.json");
    configure_at(&dir, &settings_path, &std::env::current_exe()?, enabled)
}

pub fn active_state_dir() -> Result<Option<std::path::PathBuf>> {
    let dir = super::data_dir()?;
    let installation: Option<Installation> = read_json(&dir.join("agents-installation.json"))?
        .map(serde_json::from_value)
        .transpose()?;
    Ok(installation.map(|_| dir.join("agents")))
}

pub fn configure_at(
    dir: &Path,
    settings_path: &Path,
    executable: &Path,
    enabled: bool,
) -> Result<()> {
    let manifest_path = dir.join("agents-installation.json");
    let previous = read_json(&manifest_path)?
        .map(serde_json::from_value)
        .transpose()?;
    // Saving an unrelated configuration with Agents disabled has no integration side effects.
    if previous.is_none() && !enabled {
        return Ok(());
    }
    let mut settings = read_json(settings_path)?.unwrap_or_else(|| json!({}));
    let installation = edit_settings(
        &mut settings,
        previous,
        enabled,
        executable.to_str().ok_or("Executable path is not UTF-8")?,
    )?;
    fs::create_dir_all(dir)?;
    let settings_parent = settings_path
        .parent()
        .ok_or("Missing settings parent directory")?;
    fs::create_dir_all(settings_parent)?;
    let mut content = serde_json::to_string_pretty(&settings)?;
    content.push('\n');
    fs::write(settings_path, content)?;
    if let Some(installation) = installation {
        fs::write(manifest_path, serde_json::to_vec_pretty(&installation)?)?;
    } else {
        fs::remove_file(manifest_path)?;
    }
    Ok(())
}
