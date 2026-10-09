//! Install/uninstall only the Claude Code settings owned by the Agents segment.

use super::{
    experimental,
    selection::{self, Backend, Selection},
    store::{Activity, ActivityStore},
    Result,
};
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
const PANEL_SETTING: &str = "subagentStatusLine";

#[derive(Debug, Deserialize, Serialize)]
struct Installation {
    command: String,
    added_refresh_interval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mod_settings: Option<ModSettings>,
    /// The panel command ccline wrote to `subagentStatusLine`.
    #[serde(skip_serializing_if = "Option::is_none")]
    subagent_status_line: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModSettings {
    previous_function_hooks: Option<Value>,
    previous_plugin_enabled: Option<Value>,
}

fn replace_setting(
    settings: &mut Value,
    section: &str,
    key: &str,
    value: Value,
) -> Result<Option<Value>> {
    let values = settings
        .as_object_mut()
        .ok_or("Claude settings must be an object")?
        .entry(section)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| format!("Claude settings.{section} must be an object"))?;
    Ok(values.insert(key.into(), value))
}

fn restore_setting(
    settings: &mut Value,
    section: &str,
    key: &str,
    installed: Value,
    previous: Option<Value>,
) -> Result<()> {
    let Some(values) = settings.get_mut(section) else {
        return Ok(());
    };
    let values = values
        .as_object_mut()
        .ok_or_else(|| format!("Claude settings.{section} must be an object"))?;
    if values.get(key) == Some(&installed) {
        if let Some(previous) = previous {
            values.insert(key.into(), previous);
        } else {
            values.remove(key);
        }
    }
    if values.is_empty() {
        settings.as_object_mut().unwrap().remove(section);
    }
    Ok(())
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

/// The `subagentStatusLine` value ccline installs for `command`.
fn panel_setting(command: &str) -> Value {
    json!({ "type": "command", "command": command })
}

/// Returns the installation to record and the backend actually installed: the panel falls back
/// to hooks when the user already configured their own `subagentStatusLine`.
fn edit_settings(
    settings: &mut Value,
    mut previous: Option<Installation>,
    enabled: bool,
    executable: &str,
    backend: Backend,
) -> Result<(Option<Installation>, Backend)> {
    let Some(object) = settings.as_object_mut() else {
        return Err("Claude settings must be a JSON object".into());
    };
    // Only an unchanged value is ccline's to remove; it is installed again below when selected.
    if let Some(installed) = previous
        .as_mut()
        .and_then(|installed| installed.subagent_status_line.take())
    {
        if object.get(PANEL_SETTING) == Some(&panel_setting(&installed)) {
            object.remove(PANEL_SETTING);
        }
    }
    let backend = if backend == Backend::Panel && object.contains_key(PANEL_SETTING) {
        Backend::Hooks
    } else {
        backend
    };
    if let Some(ref installed) = previous {
        remove_hooks(settings, &installed.command)?;
    }
    let mut mod_settings = previous
        .as_mut()
        .and_then(|installed| installed.mod_settings.take());
    if !enabled || backend != Backend::Mod {
        if let Some(owned) = mod_settings.take() {
            restore_setting(
                settings,
                "env",
                experimental::ENABLE_ENV,
                json!("1"),
                owned.previous_function_hooks,
            )?;
            restore_setting(
                settings,
                "enabledPlugins",
                experimental::PLUGIN_KEY,
                json!(true),
                owned.previous_plugin_enabled,
            )?;
        }
    } else {
        let original = ModSettings {
            previous_function_hooks: replace_setting(
                settings,
                "env",
                experimental::ENABLE_ENV,
                json!("1"),
            )?,
            previous_plugin_enabled: replace_setting(
                settings,
                "enabledPlugins",
                experimental::PLUGIN_KEY,
                json!(true),
            )?,
        };
        if mod_settings.is_none() {
            mod_settings = Some(original);
        }
    }
    let mut added_refresh_interval =
        previous.is_some_and(|installed| installed.added_refresh_interval);
    // The panel refreshes itself; only the inline segment needs the status line timer.
    if !enabled || backend == Backend::Panel {
        if added_refresh_interval {
            if let Some(statusline) = settings.get_mut("statusLine") {
                if statusline.get("refreshInterval") == Some(&json!(REFRESH_SECONDS)) {
                    statusline
                        .as_object_mut()
                        .ok_or("statusLine must be an object")?
                        .remove("refreshInterval");
                }
            }
        }
        added_refresh_interval = false;
    }
    if !enabled {
        return Ok((None, backend));
    }

    // Commands use a shell. Forward slashes also work with Git Bash on Windows.
    let path = if cfg!(windows) {
        executable.replace('\\', "/")
    } else {
        executable.to_owned()
    };
    let quoted = format!("'{}'", path.replace('\'', "'\\''"));
    let command = format!("{quoted} --agents-hook");
    remove_hooks(settings, &command)?;
    let object = settings.as_object_mut().unwrap();
    let mut subagent_status_line = None;
    match backend {
        Backend::Hooks => {
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
        }
        Backend::Mod => {}
        Backend::Panel => {
            let panel_command = format!("{quoted} --subagent-statusline");
            object.insert(PANEL_SETTING.into(), panel_setting(&panel_command));
            subagent_status_line = Some(panel_command);
        }
    }

    let statusline = object
        .entry("statusLine")
        .or_insert_with(|| json!({"type": "command", "command": quoted}))
        .as_object_mut()
        .ok_or("Claude settings.statusLine must be an object")?;
    if backend != Backend::Panel && !statusline.contains_key("refreshInterval") {
        statusline.insert("refreshInterval".into(), json!(REFRESH_SECONDS));
        added_refresh_interval = true;
    }
    Ok((
        Some(Installation {
            command,
            added_refresh_interval,
            mod_settings,
            subagent_status_line,
        }),
        backend,
    ))
}

pub fn configure(enabled: bool, experimental_mod: bool) -> Result<String> {
    let dir = super::data_dir()?;
    let settings_path = dir
        .parent()
        .ok_or("Missing Claude configuration directory")?
        .join("settings.json");
    let selection = if enabled {
        selection::detect(experimental_mod)?
    } else {
        Selection {
            backend: Backend::Hooks,
            message: "Agents disabled".into(),
        }
    };
    let installed = configure_at(
        &dir,
        &settings_path,
        &std::env::current_exe()?,
        enabled,
        selection.backend,
    )?;
    Ok(if installed == selection.backend {
        selection.message
    } else {
        format!("Hooks only: kept your existing {PANEL_SETTING}. Restart Claude Code to apply.")
    })
}

pub fn read_activity(session: &str) -> Result<Option<Activity>> {
    read_activity_at(&super::data_dir()?, session)
}

pub fn read_activity_at(dir: &Path, session: &str) -> Result<Option<Activity>> {
    let installation: Option<Installation> = read_json(&dir.join("agents-installation.json"))?
        .map(serde_json::from_value)
        .transpose()?;
    match installation {
        None => Ok(None),
        // Claude Code's own panel lists the agents; the status line stays free of them.
        Some(installed) if installed.subagent_status_line.is_some() => Ok(None),
        Some(installed) if installed.mod_settings.is_some() => {
            Ok(experimental::read_snapshot(session))
        }
        Some(_) => {
            ActivityStore::new(dir.join("agents"), super::claude_process_id()?).read(session)
        }
    }
}

/// Apply `backend` and return the backend actually installed (see `edit_settings`).
pub fn configure_at(
    dir: &Path,
    settings_path: &Path,
    executable: &Path,
    enabled: bool,
    backend: Backend,
) -> Result<Backend> {
    let manifest_path = dir.join("agents-installation.json");
    let previous = read_json(&manifest_path)?
        .map(serde_json::from_value)
        .transpose()?;
    // Saving an unrelated configuration with Agents disabled has no integration side effects.
    if previous.is_none() && !enabled {
        return Ok(backend);
    }
    let mut settings = read_json(settings_path)?.unwrap_or_else(|| json!({}));
    let (installation, backend) = edit_settings(
        &mut settings,
        previous,
        enabled,
        executable.to_str().ok_or("Executable path is not UTF-8")?,
        backend,
    )?;
    fs::create_dir_all(dir)?;
    let settings_parent = settings_path
        .parent()
        .ok_or("Missing settings parent directory")?;
    fs::create_dir_all(settings_parent)?;
    if installation
        .as_ref()
        .is_some_and(|installed| installed.mod_settings.is_some())
    {
        experimental::install(settings_parent)?;
    }
    let mut content = serde_json::to_string_pretty(&settings)?;
    content.push('\n');
    fs::write(settings_path, content)?;
    if let Some(installation) = installation {
        fs::write(manifest_path, serde_json::to_vec_pretty(&installation)?)?;
    } else {
        fs::remove_file(manifest_path)?;
    }
    Ok(backend)
}
