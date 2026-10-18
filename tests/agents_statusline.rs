use ccometixline::config::{paths, Config, SegmentId, StyleMode};
use ccometixline::ui::themes::ThemePresets;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

#[path = "../src/test_support.rs"]
mod test_support;
use test_support::{panel_task, TestDir};

fn save_config(dir: &Path, config: &Config) {
    let ccline = paths::ccline_dir(dir);
    fs::create_dir_all(&ccline).unwrap();
    fs::write(ccline.join("config.toml"), toml::to_string(config).unwrap()).unwrap();
}

fn invoke(dir: &Path, panel: bool, input: &Value) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccometixline"));
    command.env("CLAUDE_CONFIG_DIR", dir);
    if panel {
        command.arg("--subagent-statusline");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(input).unwrap())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn render(dir: &Path, panel: bool, input: &Value) -> String {
    let output = invoke(dir, panel, input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

fn status_input(session: &str) -> Value {
    json!({"session_id":session,"model":{"id":"claude-opus-4-6","display_name":"Opus"},
        "effort":{"level":"high"},"workspace":{"current_dir":"/work"},
        "context_window":{"context_window_size":200000}})
}

#[test]
fn native_panel_feeds_an_inline_count_without_icons_or_blank_lines_in_details() {
    let escapes = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    for mut config in [
        ThemePresets::get_default(),
        ThemePresets::get_powerline_dark(),
    ] {
        let dir = TestDir::new();
        config.segments.retain(|segment| {
            matches!(
                segment.id,
                SegmentId::Model | SegmentId::Effort | SegmentId::Agents
            )
        });
        for segment in &mut config.segments {
            segment.enabled = true;
        }
        save_config(&dir.0, &config);
        let input = status_input("one");
        let without = render(&dir.0, false, &input);
        assert!(!without.contains("Agents running"));
        let mut panel = json!({"session_id":"one","columns":500,"tasks":[panel_task(), panel_task(), panel_task()]});
        panel["tasks"][1]["id"] = json!("t2");
        panel["tasks"][2]["id"] = json!("t3");
        panel["tasks"][2]["status"] = json!("failed");
        let details = render(&dir.0, true, &panel);
        assert_eq!(details.lines().count(), 3);
        for line in details.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let content = row["content"].as_str().unwrap();
            let plain = escapes.replace_all(content, "");
            assert!(plain.starts_with("Explore · "), "{plain}");
            assert!(!content.contains(['\r', '\n']));
        }
        let main = render(&dir.0, false, &input);
        assert_eq!(main.lines().count(), 1);
        assert!(
            main.contains("Opus 4.6") && main.contains("high") && main.contains("2 Agents running"),
            "{main:?}"
        );
        let agents = config
            .segments
            .iter()
            .find(|segment| segment.id == SegmentId::Agents)
            .unwrap();
        let icon = if config.style.mode == StyleMode::Plain {
            &agents.icon.plain
        } else {
            &agents.icon.nerd_font
        };
        assert!(main.contains(icon));
        assert_eq!(render(&dir.0, false, &status_input("two")), without);
        panel["tasks"] = json!([]);
        assert!(render(&dir.0, true, &panel).is_empty());
        assert_eq!(render(&dir.0, false, &input), without);
    }
}

#[test]
fn empty_expired_disabled_and_invalid_summaries_do_not_leave_blank_output() {
    let dir = TestDir::new();
    let mut config = ThemePresets::get_default();
    config.segments = vec![ThemePresets::agents_segment("default")];
    config.segments[0].enabled = true;
    save_config(&dir.0, &config);
    let input = status_input("s");
    assert!(render(&dir.0, false, &input).is_empty());
    let panel = json!({"session_id":"s","columns":500,"tasks":[panel_task()]});
    render(&dir.0, true, &panel);
    let cache = fs::read_dir(paths::ccline_dir(&dir.0).join("agents"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(&cache, r#"{"updated_at":0,"running":1}"#).unwrap();
    assert!(render(&dir.0, false, &input).is_empty());
    fs::write(&cache, "{").unwrap();
    let error = invoke(&dir.0, false, &input);
    assert!(!error.status.success());
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("Invalid"));
    config.segments[0].enabled = false;
    save_config(&dir.0, &config);
    assert!(render(&dir.0, true, &panel).is_empty());
    assert!(render(&dir.0, false, &input).is_empty());
    assert_eq!(fs::read_to_string(cache).unwrap(), "{");
}
