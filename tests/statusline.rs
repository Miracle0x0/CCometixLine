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
        panel["tasks"][1]["status"] = json!("completed");
        panel["tasks"][0]["model"] = json!("claude-fable-5-1");
        assert!(render(&dir.0, true, &panel).contains("Fable 5.1"));
        let single = render(&dir.0, false, &input);
        assert!(single.contains("1 Agent running"), "{single}");
        assert!(!single.contains("1 Agents running"));
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

#[test]
fn native_segments_render_session_worktree_review_and_cache_without_external_state() {
    let dir = TestDir::new();
    let mut config = ThemePresets::get_default();
    config.segments = ThemePresets::native_segments("default").to_vec();
    for segment in &mut config.segments {
        segment.enabled = true;
    }
    save_config(&dir.0, &config);
    let mut input = status_input("native");
    assert!(render(&dir.0, false, &input).is_empty());
    input["session_name"] = json!("Repair\n登录");
    input["workspace"]["git_worktree"] = json!("feature-login");
    input["prompt_cache"] = json!({"warm":false,"caching_observed":true,"hit_ratio":0.91,
        "expires_at":null,"misses":1,"expected_rebuilds":12,
        "last_miss_cause":{"causes":["tools_changed"]}});
    input["pr"] = json!({"number":123,"url":"https://github.com/org/repo/pull/123",
        "review_state":"changes_requested"});
    let output = render(&dir.0, false, &input);
    assert_eq!(output.lines().count(), 1);
    for text in [
        "Repair登录",
        "feature-login",
        "91%",
        "cold",
        "1 miss",
        "last: tools_changed",
        "changes requested",
    ] {
        assert!(output.contains(text), "missing {text}: {output}");
    }
    assert!(
        output.contains("\x1b]8;;https://github.com/org/repo/pull/123\x1b\\PR #123\x1b]8;;\x1b\\")
    );
    assert!(!output.contains("12 misses"));
    for state in ["approved", "pending", "draft"] {
        input["pr"] = json!({"number":42,"url":"https://gitlab.com/team/sub/repo/-/merge_requests/42", "kind":"mr", "review_state":state});
        let output = render(&dir.0, false, &input);
        assert!(output.contains("MR !42"));
        assert!(output.contains(state));
    }
    input["pr"].as_object_mut().unwrap().remove("review_state");
    assert!(render(&dir.0, false, &input).contains("MR !42"));
    input["session_name"] = json!(null);
    input["workspace"]["git_worktree"] = json!(null);
    input["pr"] = json!(null);
    input["prompt_cache"] = json!(null);
    assert!(render(&dir.0, false, &input).is_empty());
}

#[test]
fn malformed_native_identity_fields_fail_instead_of_rendering_defaults() {
    let dir = TestDir::new();
    for (key, value) in [
        ("session_name", json!(3)),
        (
            "workspace",
            json!({"current_dir":"/work","git_worktree":true}),
        ),
        ("pr", json!({"number":1})),
        (
            "pr",
            json!({"number":1,"url":"https://example.com","review_state":"not-a-state"}),
        ),
    ] {
        let mut input = status_input("invalid");
        input[key] = value;
        let output = invoke(&dir.0, false, &input);
        assert!(!output.status.success(), "{key}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
