use super::integration;
use crate::test_support::TestDir;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
/// An isolated Claude configuration directory (with a space in its path, as a custom
/// `CLAUDE_CONFIG_DIR` may have) and executable path; dropping the returned directory removes
/// them all.
fn installation(executable: &str) -> (TestDir, PathBuf, PathBuf) {
    let dir = TestDir::new();
    let claude = dir.0.join("claude work");
    let exe = dir.0.join(executable);
    (dir, claude, exe)
}

fn settings_path(claude: &Path) -> PathBuf {
    claude.join("settings.json")
}

fn manifest_path(claude: &Path) -> PathBuf {
    claude.join("ccline").join("agents-installation.json")
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

#[test]
fn panel_install_is_idempotent_and_uninstall_restores_other_settings() {
    let (_dir, claude, exe) = installation("a path with 'quotes'/$dollars/ccline");
    let original = json!({"model":"opus",
        "statusLine":{"type":"command","command":"custom-status","padding":2,"refreshInterval":60},
        "hooks":{"SubagentStart":[{"hooks":[{"type":"command","command":"audit"}]}]}});
    write(&settings_path(&claude), &original);
    integration::configure_at(&claude, &exe, true).unwrap();
    let installed = read(&settings_path(&claude));
    let manifest = read(&manifest_path(&claude));
    let command = installed["subagentStatusLine"]["command"].as_str().unwrap();
    assert_eq!(installed["subagentStatusLine"]["type"], "command");
    assert!(command.ends_with(" --subagent-statusline"));
    assert!(command.contains("'\\''"));
    assert_eq!(manifest["subagent_status_line"], command);
    // Nothing but the panel setting is touched, including the user's refresh interval.
    let mut expected = original.clone();
    expected["subagentStatusLine"] = installed["subagentStatusLine"].clone();
    assert_eq!(installed, expected);

    integration::configure_at(&claude, &exe, true).unwrap();
    assert_eq!(read(&settings_path(&claude)), installed);
    assert_eq!(read(&manifest_path(&claude)), manifest);
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), original);
    assert!(!manifest_path(&claude).exists());
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), original);
}

#[test]
fn installing_creates_files_only_in_the_given_directory() {
    let (dir, claude, exe) = installation("ccline");
    integration::configure_at(&claude, &exe, false).unwrap();
    assert!(!claude.exists());
    integration::configure_at(&claude, &exe, true).unwrap();
    assert_eq!(read(&settings_path(&claude)).as_object().unwrap().len(), 1);
    assert!(manifest_path(&claude).is_file());
    let mut created: Vec<_> = fs::read_dir(&dir.0)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    created.sort();
    assert_eq!(created, ["claude work"]);
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), json!({}));
}

#[test]
fn a_foreign_panel_command_is_a_conflict_that_changes_nothing() {
    let (_dir, claude, exe) = installation("ccline");
    let mine = json!({"subagentStatusLine":{"type":"command","command":"my-rows"}});
    write(&settings_path(&claude), &mine);
    let before = fs::read(settings_path(&claude)).unwrap();
    let error = integration::configure_at(&claude, &exe, true)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("subagentStatusLine") && error.contains("my-rows"),
        "{error}"
    );
    assert_eq!(fs::read(settings_path(&claude)).unwrap(), before);
    assert!(!manifest_path(&claude).exists());
    // Disabling without an installation leaves the user's value alone.
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(fs::read(settings_path(&claude)).unwrap(), before);
}

#[test]
fn a_later_edit_makes_the_value_the_users_and_uninstall_keeps_it() {
    let (_dir, claude, exe) = installation("ccline");
    integration::configure_at(&claude, &exe, true).unwrap();
    let mut edited = read(&settings_path(&claude));
    edited["subagentStatusLine"]["command"] = json!("edited");
    write(&settings_path(&claude), &edited);
    assert!(integration::configure_at(&claude, &exe, true).is_err());
    assert_eq!(read(&settings_path(&claude)), edited);
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), edited);
    assert!(!manifest_path(&claude).exists());
}

#[test]
fn a_moved_executable_replaces_its_own_previous_command() {
    let (_dir, claude, old) = installation("old/ccline");
    integration::configure_at(&claude, &old, true).unwrap();
    let new = old.parent().unwrap().join("new-ccline");
    integration::configure_at(&claude, &new, true).unwrap();
    let command = read(&settings_path(&claude))["subagentStatusLine"]["command"].clone();
    assert!(command.as_str().unwrap().contains("new-ccline"));
    assert_eq!(
        read(&manifest_path(&claude))["subagent_status_line"],
        command
    );
    integration::configure_at(&claude, &new, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), json!({}));
}

#[test]
fn an_identical_unrecorded_value_is_adopted_instead_of_reported_as_a_conflict() {
    let (_dir, claude, exe) = installation("ccline");
    integration::configure_at(&claude, &exe, true).unwrap();
    fs::remove_file(manifest_path(&claude)).unwrap();
    integration::configure_at(&claude, &exe, true).unwrap();
    assert!(manifest_path(&claude).is_file());
    integration::configure_at(&claude, &exe, false).unwrap();
    assert_eq!(read(&settings_path(&claude)), json!({}));
}

#[test]
fn invalid_settings_or_manifest_fail_without_changes() {
    let (_dir, claude, exe) = installation("ccline");
    for invalid in ["{", "null", "[]"] {
        fs::create_dir_all(&claude).unwrap();
        fs::write(settings_path(&claude), invalid).unwrap();
        assert!(integration::configure_at(&claude, &exe, true).is_err());
        assert_eq!(fs::read_to_string(settings_path(&claude)).unwrap(), invalid);
        assert!(!manifest_path(&claude).exists());
    }
    fs::write(settings_path(&claude), "{}").unwrap();
    // A manifest from the removed hooks backend has no panel command to own.
    write(
        &manifest_path(&claude),
        &json!({"command":"'ccline' --agents-hook","added_refresh_interval":true}),
    );
    let error = integration::configure_at(&claude, &exe, true)
        .unwrap_err()
        .to_string();
    assert!(error.contains("agents-installation.json"), "{error}");
    assert_eq!(fs::read_to_string(settings_path(&claude)).unwrap(), "{}");
}

mod panel_rows {
    use super::super::panel::{rows, PanelInput, PanelRow};
    use crate::config::{Config, ModelConfig, SegmentId};
    use crate::core::statusline::visible_width;
    use crate::ui::themes::ThemePresets;
    use serde_json::json;

    fn config(theme: &str, enabled: bool) -> Config {
        // Built-in presets only: the user's theme files must not affect tests.
        let mut config = match theme {
            "default" => ThemePresets::get_default(),
            _ => ThemePresets::get_powerline_dark(),
        };
        for segment in &mut config.segments {
            if segment.id == SegmentId::Agents {
                segment.enabled = enabled;
            }
        }
        config
    }

    fn task() -> serde_json::Value {
        json!({"id":"t1","name":"scout","type":"local_agent","agentType":"Explore","status":"running",
            "description":"Find config loading","label":"查找配置加载","startTime":100_000_u64,
            "model":"custom-model","effort":"high","contextWindowSize":200_000,"tokenCount":12_400,
            "tokenSamples":[12_400],"cwd":"/tmp"})
    }

    fn render(config: &Config, columns: usize, tasks: Vec<serde_json::Value>) -> Vec<PanelRow> {
        let input: PanelInput =
            serde_json::from_value(json!({"session_id":"s","transcript_path":"/t",
            "cwd":"/tmp","columns":columns,"tasks":tasks}))
            .unwrap();
        rows(&input, config, 180, &ModelConfig::default())
    }

    #[test]
    fn rows_show_every_field_in_the_agents_colors_without_backgrounds() {
        for theme in ["default", "powerline-dark"] {
            let rows = render(&config(theme, true), 500, vec![task()]);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].id, "t1");
            assert!(
                rows[0]
                    .content
                    .contains("Explore · 查找配置加载 · 1m20s · 12.4k (6%) · custom-model · high"),
                "{theme}: {:?}",
                rows[0].content
            );
            assert!(!rows[0].content.contains("\x1b[48"), "{theme}");
            assert!(!rows[0].content.contains('\n'));
        }
    }

    #[test]
    fn narrow_rows_drop_trailing_details_then_shorten_the_label() {
        let config = config("default", true);
        let full = render(&config, 500, vec![task()]).remove(0).content;
        let width = visible_width(&full);
        let expectations = [
            (width - 1, "custom-model", "high"),
            (width - 8, "12.4k (6%)", "custom-model"),
        ];
        for (columns, kept, dropped) in expectations {
            let row = render(&config, columns, vec![task()]).remove(0).content;
            assert!(visible_width(&row) <= columns);
            assert!(row.contains(kept), "{columns}: {row:?}");
            assert!(!row.contains(dropped), "{columns}: {row:?}");
        }
        let minimal = render(&config, 0, vec![task()]).remove(0).content;
        let base = visible_width(&minimal) - 1; // without the ellipsis
        let short = render(&config, base + 3, vec![task()]).remove(0).content;
        assert!(short.contains("查…"), "{short:?}");
        assert!(visible_width(&short) <= base + 3);
        assert!(minimal.contains("Explore · … · 1m20s"), "{minimal:?}");
    }

    #[test]
    fn optional_fields_status_and_control_characters() {
        let config = config("default", true);
        let mut failed = task();
        failed["status"] = json!("failed");
        failed["label"] = json!("a\nb\u{1b}c");
        failed["effort"] = json!(8000);
        for key in ["agentType", "model", "contextWindowSize"] {
            failed.as_object_mut().unwrap().remove(key);
        }
        let row = render(&config, 500, vec![failed.clone()]).remove(0).content;
        assert!(
            row.contains("scout · abc · failed · 12.4k · 8k budget"),
            "{row:?}"
        );
        failed.as_object_mut().unwrap().remove("name");
        let row = render(&config, 500, vec![failed.clone()]).remove(0).content;
        assert!(row.contains("local_agent · abc"), "{row:?}");
        failed["effort"] = json!(true);
        let input = json!({"columns":80,"tasks":[failed]});
        assert!(serde_json::from_value::<PanelInput>(input).is_err());
    }

    #[test]
    fn disabled_agents_keep_the_default_rows() {
        assert!(render(&config("default", false), 500, vec![task()]).is_empty());
        let mut config = config("default", true);
        config.segments.retain(|s| s.id != SegmentId::Agents);
        assert!(render(&config, 500, vec![task()]).is_empty());
    }
}
