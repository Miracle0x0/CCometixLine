//! Share the most recent visible running count between the panel and the main status line.

use super::{panel::PanelInput, Result};
use crate::config::{paths, SegmentConfig};
use crate::core::segments::SegmentData;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_TTL_SECONDS: u64 = 15;

#[derive(Deserialize, Serialize)]
struct Summary {
    updated_at: u64,
    running: usize,
}

fn summary_path(claude_dir: &Path, session_id: &str) -> PathBuf {
    // Encode the session ID so even path separators remain part of a single filename.
    let key: String = session_id
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    paths::ccline_dir(claude_dir)
        .join("agents")
        .join(format!("{key}.json"))
}

pub fn save(claude_dir: &Path, input: &PanelInput, now: u64) -> Result<()> {
    let summary = Summary {
        updated_at: now,
        running: input
            .tasks
            .iter()
            .filter(|task| task.kind == "local_agent" && task.status == "running")
            .count(),
    };
    let path = summary_path(claude_dir, &input.session_id);
    fs::create_dir_all(path.parent().expect("summary has a parent directory"))?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, serde_json::to_vec(&summary)?)?;
    // Readers see either complete snapshot, never a partially written count.
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn collect(
    claude_dir: &Path,
    session_id: &str,
    config: &SegmentConfig,
    now: u64,
) -> Result<Option<SegmentData>> {
    let ttl = match config.options.get("summary_ttl_seconds") {
        None => DEFAULT_TTL_SECONDS,
        Some(value) => value
            .as_u64()
            .ok_or("Agents summary_ttl_seconds must be a non-negative integer")?,
    };
    let path = summary_path(claude_dir, session_id);
    let Some(content) = paths::read_optional(&path)? else {
        return Ok(None);
    };
    let summary: Summary = serde_json::from_str(&content)
        .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
    // The host stops invoking the panel command when its last row disappears.
    // Zero explicitly disables expiry; a positive TTL hides an outdated observation.
    if summary.running == 0 || (ttl > 0 && now.saturating_sub(summary.updated_at) >= ttl) {
        return Ok(None);
    }
    Ok(Some(segment_data(summary.running)))
}

pub fn segment_data(running: usize) -> SegmentData {
    let noun = if running == 1 { "Agent" } else { "Agents" };
    SegmentData {
        primary: format!("{running} {noun}"),
        secondary: String::new(),
        metadata: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{panel_task, TestDir};
    use crate::ui::themes::ThemePresets;
    use serde_json::json;

    fn input(session_id: &str, states: &[&str]) -> PanelInput {
        let tasks: Vec<_> = states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                let mut task = panel_task();
                task["id"] = json!(format!("t{index}"));
                task["status"] = json!(state);
                task
            })
            .collect();
        serde_json::from_value(json!({"session_id":session_id,"columns":80,"tasks":tasks})).unwrap()
    }

    #[test]
    fn counts_only_visible_running_agents_and_replaces_the_previous_snapshot() {
        let dir = TestDir::new();
        let config = ThemePresets::agents_segment("default");
        assert!(collect(&dir.0, "s", &config, 100).unwrap().is_none());
        let mut panel = input(
            "s",
            &["running", "completed", "failed", "killed", "running"],
        );
        panel.tasks[4].kind = "local_bash".into();
        save(&dir.0, &panel, 100).unwrap();
        assert_eq!(
            collect(&dir.0, "s", &config, 100).unwrap().unwrap().primary,
            "1 Agent"
        );
        for states in [&["running", "running"][..], &["failed"][..], &[][..]] {
            save(&dir.0, &input("s", states), 100).unwrap();
            let result = collect(&dir.0, "s", &config, 100).unwrap();
            if states.len() == 2 {
                assert_eq!(result.unwrap().primary, "2 Agents");
            } else {
                assert!(result.is_none());
            }
        }
        assert_eq!(
            fs::read_dir(paths::ccline_dir(&dir.0).join("agents"))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn expiry_is_configurable_and_zero_disables_it() {
        let dir = TestDir::new();
        let mut config = ThemePresets::agents_segment("default");
        config.options.clear();
        save(&dir.0, &input("s", &["running"]), 100).unwrap();
        assert!(collect(&dir.0, "s", &config, 114).unwrap().is_some());
        assert!(collect(&dir.0, "s", &config, 115).unwrap().is_none());
        config
            .options
            .insert("summary_ttl_seconds".into(), json!(30));
        assert!(collect(&dir.0, "s", &config, 129).unwrap().is_some());
        assert!(collect(&dir.0, "s", &config, 130).unwrap().is_none());
        config
            .options
            .insert("summary_ttl_seconds".into(), json!(0));
        assert!(collect(&dir.0, "s", &config, 100_000).unwrap().is_some());
    }

    #[test]
    fn sessions_have_independent_snapshots_and_cannot_escape_the_cache_directory() {
        let dir = TestDir::new();
        let config = ThemePresets::agents_segment("default");
        let first = "../session/一";
        let second = "..\\session\\一";
        save(&dir.0, &input(first, &["running"]), 100).unwrap();
        assert!(collect(&dir.0, second, &config, 100).unwrap().is_none());
        save(&dir.0, &input(second, &[]), 100).unwrap();
        assert!(collect(&dir.0, first, &config, 100).unwrap().is_some());
        let cache = paths::ccline_dir(&dir.0).join("agents");
        assert_eq!(summary_path(&dir.0, first).parent(), Some(cache.as_path()));
        assert_eq!(fs::read_dir(cache).unwrap().count(), 2);
    }

    #[test]
    fn invalid_options_snapshots_and_io_errors_are_reported() {
        let dir = TestDir::new();
        let mut config = ThemePresets::agents_segment("default");
        for invalid in [json!(-1), json!(1.5), json!("15"), json!(null)] {
            config.options.insert("summary_ttl_seconds".into(), invalid);
            assert!(collect(&dir.0, "s", &config, 100)
                .unwrap_err()
                .to_string()
                .contains("summary_ttl_seconds"));
        }
        config.options.clear();
        let panel = input("s", &["running"]);
        save(&dir.0, &panel, 100).unwrap();
        let path = summary_path(&dir.0, "s");
        for invalid in ["{", "{}", r#"{"updated_at":100,"running":-1}"#] {
            fs::write(&path, invalid).unwrap();
            let error = collect(&dir.0, "s", &config, 100).unwrap_err().to_string();
            assert!(error.contains(path.to_str().unwrap()), "{error}");
        }
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(collect(&dir.0, "s", &config, 100).is_err());
        assert!(save(&dir.0, &panel, 100).is_err());
    }
}
