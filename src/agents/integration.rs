//! Install/uninstall only the hooks owned by the Agents segment.

use super::{
    experimental::{self, Backend},
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

#[derive(Debug, Deserialize, Serialize)]
struct Installation {
    command: String,
    added_refresh_interval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mod_settings: Option<ModSettings>,
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

fn edit_settings(
    settings: &mut Value,
    mut previous: Option<Installation>,
    enabled: bool,
    executable: &str,
    backend: Backend,
) -> Result<Option<Installation>> {
    if !settings.is_object() {
        return Err("Claude settings must be a JSON object".into());
    }
    if let Some(ref installed) = previous {
        remove_hooks(settings, &installed.command)?;
    }
    let mut mod_settings = previous
        .as_mut()
        .and_then(|installed| installed.mod_settings.take());
    if !enabled || backend == Backend::Hooks {
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
    if backend == Backend::Hooks {
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
        mod_settings,
    }))
}

pub fn configure(enabled: bool, experimental_mod: bool) -> Result<String> {
    let dir = super::data_dir()?;
    let settings_path = dir
        .parent()
        .ok_or("Missing Claude configuration directory")?
        .join("settings.json");
    let selection = if enabled && experimental_mod {
        experimental::detect()?
    } else {
        experimental::Selection {
            backend: Backend::Hooks,
            message: if enabled {
                "Hooks only"
            } else {
                "Agents disabled"
            }
            .into(),
        }
    };
    configure_at(
        &dir,
        &settings_path,
        &std::env::current_exe()?,
        enabled,
        selection.backend,
    )?;
    Ok(selection.message)
}

pub fn read_activity(session: &str) -> Result<Option<Activity>> {
    let dir = super::data_dir()?;
    let installation: Option<Installation> = read_json(&dir.join("agents-installation.json"))?
        .map(serde_json::from_value)
        .transpose()?;
    match installation {
        None => Ok(None),
        Some(installed) if installed.mod_settings.is_some() => {
            experimental::read_snapshot(session).map(Some)
        }
        Some(_) => ActivityStore::new(dir.join("agents")).read(session),
    }
}

pub fn configure_at(
    dir: &Path,
    settings_path: &Path,
    executable: &Path,
    enabled: bool,
    backend: Backend,
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
    Ok(())
}
