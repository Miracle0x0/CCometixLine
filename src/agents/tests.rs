use super::{
    integration,
    selection::{self, Backend},
    store::{Activity, ActivityStore},
    HookEvent, HookInput, SessionEndReason,
};
use crate::core::segments::{AgentsSegment, SegmentData};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ccline-agents-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn start(id: &str, name: &str) -> HookEvent {
    HookEvent::SubagentStart {
        agent_id: id.into(),
        agent_type: name.into(),
    }
}
fn stop(id: &str) -> HookEvent {
    HookEvent::SubagentStop {
        agent_id: id.into(),
        agent_type: "Explore".into(),
    }
}
fn session_start(source: &str) -> HookEvent {
    HookEvent::SessionStart {
        source: source.into(),
    }
}
fn session_end(reason: SessionEndReason) -> HookEvent {
    HookEvent::SessionEnd { reason }
}
fn input(session: &str, event: HookEvent) -> HookInput {
    HookInput {
        session_id: session.into(),
        event,
    }
}
fn statusline_input(session: &str) -> crate::config::InputData {
    serde_json::from_value(json!({"session_id": session,
        "model":{"id":"test","display_name":"test"},"workspace":{"current_dir":"/"},"transcript_path":"unused"})).unwrap()
}
fn render(
    activity: Option<&Activity>,
    now: u64,
) -> Option<crate::core::segments::agents::AgentSummary> {
    AgentsSegment::render(activity, now, AgentsSegment::DEFAULT_MAX_AGENTS)
}

#[test]
fn duplicate_events_and_resuming_preserve_identity_and_elapsed_time() {
    let mut activity = Activity::default();
    activity.apply(start("a", "Explore"), 100);
    activity.apply(start("a", "Explore"), 110);
    activity.apply(start("b", "reviewer"), 115);
    let text = render(Some(&activity), 180).unwrap().text(usize::MAX);
    assert_eq!(text, "Agents: 2 active · Explore 1m20s · reviewer 1m05s");
    activity.apply(stop("a"), 190);
    activity.apply(stop("a"), 200);
    activity.apply(stop("unobserved"), 200);
    assert_eq!(activity.agents.len(), 2);
    assert_eq!(activity.agents["a"].responded_at, Some(190));
    assert!(render(Some(&activity), 201)
        .unwrap()
        .text(usize::MAX)
        .contains("1 active · 1 responded"));
    activity.apply(start("a", "Explore"), 250);
    assert_eq!(activity.agents["a"].started_at, 250);
    assert_eq!(activity.agents["a"].responded_at, None);
}

#[test]
fn idle_and_ended_sessions_hide_the_entire_segment() {
    let mut activity = Activity::default();
    assert!(render(None, 100).is_none());
    assert!(render(Some(&activity), 100).is_none());
    activity.apply(start("a", "Explore"), 100);
    activity.apply(stop("a"), 130);
    assert!(render(Some(&activity), 140).is_none());
    activity.apply(start("a", "Explore"), 150);
    activity.apply(session_end(SessionEndReason::Other), 160);
    activity.apply(start("late", "Explore"), 170);
    assert!(render(Some(&activity), 180).is_none());
    assert_eq!(activity.agents.len(), 1);
}

#[test]
fn compaction_preserves_agents_and_resume_starts_a_new_observation() {
    let mut activity = Activity::default();
    activity.apply(start("a", "Explore"), 100);
    activity.apply(
        HookEvent::SessionStart {
            source: "compact".into(),
        },
        110,
    );
    assert_eq!(activity.agents.len(), 1);
    activity.apply(
        HookEvent::SessionStart {
            source: "resume".into(),
        },
        120,
    );
    assert!(activity.agents.is_empty());
    assert!(!activity.ended);
}

#[test]
fn hook_schema_rejects_missing_fields_and_unknown_events() {
    let valid = json!({"session_id":"one", "hook_event_name":"SubagentStart",
        "agent_id":"a", "agent_type":"Explore", "cwd":"/tmp", "transcript_path":"/tmp/one.jsonl"});
    assert!(serde_json::from_value::<HookInput>(valid.clone()).is_ok());
    for field in ["session_id", "agent_id", "agent_type"] {
        let mut invalid = valid.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<HookInput>(invalid).is_err());
    }
    assert!(serde_json::from_value::<HookInput>(
        json!({"session_id":"one", "hook_event_name":"Unknown"})
    )
    .is_err());
    for reason in ["clear", "resume", "logout", "prompt_input_exit", "other"] {
        assert!(serde_json::from_value::<HookInput>(json!({
            "session_id": "one", "hook_event_name": "SessionEnd", "reason": reason
        }))
        .is_ok());
    }
    for invalid in [
        json!({"session_id": "one", "hook_event_name": "SessionEnd"}),
        json!({"session_id": "one", "hook_event_name": "SessionEnd", "reason": "unknown"}),
    ] {
        assert!(serde_json::from_value::<HookInput>(invalid).is_err());
    }
}

#[test]
fn store_serializes_concurrent_writers_and_isolates_sessions() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    assert!(store.read("one").unwrap().is_none());
    assert!(fs::read_dir(&dir.0).unwrap().next().is_none());
    store
        .record(input("one", session_start("startup")), 90)
        .unwrap();
    std::thread::scope(|scope| {
        for id in 0..24 {
            let store = &store;
            scope.spawn(move || {
                store
                    .record(input("one", start(&id.to_string(), "Explore")), 100)
                    .unwrap()
            });
        }
    });
    assert_eq!(store.read("one").unwrap().unwrap().agents.len(), 24);
    assert!(store.read("two").unwrap().is_none());
    store
        .record(input("../../two", session_start("clear")), 100)
        .unwrap();
    store
        .record(input("../../two", start("other", "Plan")), 101)
        .unwrap();
    assert_eq!(store.read("../../two").unwrap().unwrap().agents.len(), 1);
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    std::thread::scope(|scope| {
        for id in 0..24 {
            let store = &store;
            scope.spawn(move || {
                store
                    .record(input("one", stop(&id.to_string())), 120)
                    .unwrap()
            });
        }
    });
    assert!(render(store.read("one").unwrap().as_ref(), 130).is_none());
}

#[test]
fn process_exit_removes_all_owned_sessions_and_preserves_other_processes() {
    for reason in [
        SessionEndReason::PromptInputExit,
        SessionEndReason::Logout,
        SessionEndReason::Other,
    ] {
        let dir = TestDir::new();
        let store = ActivityStore::new(dir.0.clone(), 1);
        let other = ActivityStore::new(dir.0.clone(), 2);
        for current in [&store, &other] {
            current
                .record(input("one", session_start("startup")), 100)
                .unwrap();
            current
                .record(input("one", start("a", "Explore")), 110)
                .unwrap();
        }
        store
            .record(input("one", session_end(SessionEndReason::Resume)), 120)
            .unwrap();
        assert!(store.read("one").unwrap().unwrap().ended);
        store
            .record(input("two", session_start("resume")), 130)
            .unwrap();
        store.record(input("two", start("b", "Plan")), 140).unwrap();
        store
            .record(input("two", session_end(reason)), 150)
            .unwrap();

        assert!(!dir.0.join("1.json").exists());
        assert!(!dir.0.join("1.lock").exists());
        assert!(store.read("one").unwrap().is_none());
        assert!(store.read("two").unwrap().is_none());
        assert_eq!(other.read("one").unwrap().unwrap().agents.len(), 1);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);

        store
            .record(input("two", start("late", "Explore")), 160)
            .unwrap();
        store.record(input("two", stop("b")), 160).unwrap();
        store
            .record(input("two", session_end(SessionEndReason::Other)), 160)
            .unwrap();
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }
}

#[test]
fn clear_removes_only_the_current_session_and_allows_new_activity() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    store
        .record(input("old", session_start("startup")), 100)
        .unwrap();
    store
        .record(input("old", start("old-agent", "Explore")), 110)
        .unwrap();
    store
        .record(input("kept", session_start("resume")), 120)
        .unwrap();
    store
        .record(input("kept", start("kept-agent", "Plan")), 130)
        .unwrap();
    store
        .record(input("old", session_end(SessionEndReason::Clear)), 140)
        .unwrap();
    assert!(store.read("old").unwrap().is_none());
    assert_eq!(store.read("kept").unwrap().unwrap().agents.len(), 1);
    assert!(!fs::read_to_string(dir.0.join("1.json"))
        .unwrap()
        .contains("old-agent"));

    store.record(input("old", stop("old-agent")), 150).unwrap();
    store
        .record(input("old", start("late", "Explore")), 150)
        .unwrap();
    assert!(store.read("old").unwrap().is_none());
    store
        .record(input("kept", session_end(SessionEndReason::Clear)), 150)
        .unwrap();
    assert!(!dir.0.join("1.json").exists());
    assert!(dir.0.join("1.lock").exists());
    store
        .record(input("kept", session_end(SessionEndReason::Clear)), 155)
        .unwrap();
    assert!(!dir.0.join("1.json").exists());
    store
        .record(input("new", session_start("clear")), 160)
        .unwrap();
    store
        .record(input("new", start("new-agent", "Explore")), 170)
        .unwrap();
    assert_eq!(store.read("new").unwrap().unwrap().agents.len(), 1);
    store
        .record(input("new", session_end(SessionEndReason::Other)), 180)
        .unwrap();
    assert!(fs::read_dir(&dir.0).unwrap().next().is_none());
}

#[test]
fn completed_subagents_and_compaction_keep_records_until_exit() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    store
        .record(input("one", session_start("startup")), 100)
        .unwrap();
    store
        .record(input("one", start("a", "Explore")), 110)
        .unwrap();
    store.record(input("one", stop("a")), 120).unwrap();
    store
        .record(input("one", session_start("compact")), 130)
        .unwrap();
    let activity = store.read("one").unwrap().unwrap();
    assert_eq!(activity.agents["a"].responded_at, Some(120));
    assert!(!activity.ended);
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    store
        .record(input("one", start("a", "Explore")), 140)
        .unwrap();
    assert_eq!(
        store.read("one").unwrap().unwrap().agents["a"].started_at,
        140
    );
}

#[test]
fn concurrent_exit_events_updates_and_reads_do_not_recreate_files() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    store
        .record(input("one", session_start("startup")), 100)
        .unwrap();
    let barrier = std::sync::Barrier::new(32);
    std::thread::scope(|scope| {
        for index in 0..32 {
            let store = &store;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                match index % 4 {
                    0 => store
                        .record(input("one", session_end(SessionEndReason::Other)), 120)
                        .unwrap(),
                    1 => store
                        .record(input("one", start(&index.to_string(), "Explore")), 120)
                        .unwrap(),
                    2 => store
                        .record(input("one", stop(&index.to_string())), 120)
                        .unwrap(),
                    _ => {
                        store.read("one").unwrap();
                    }
                }
            });
        }
    });
    assert!(store.read("one").unwrap().is_none());
    assert!(fs::read_dir(&dir.0).unwrap().next().is_none());
}

#[test]
fn exit_without_a_started_process_creates_no_files() {
    let dir = TestDir::new();
    let root = dir.0.join("agents");
    let store = ActivityStore::new(root.clone(), 1);
    store
        .record(input("one", session_end(SessionEndReason::Other)), 100)
        .unwrap();
    assert!(!root.exists());
}

#[test]
fn corrupt_activity_is_reported_and_not_replaced_by_empty_state() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    store
        .record(input("a", session_start("startup")), 90)
        .unwrap();
    store
        .record(input("a", start("one", "Explore")), 100)
        .unwrap();
    fs::write(dir.0.join("1.json"), "{").unwrap();
    assert!(store.read("a").is_err());
    assert!(store.record(input("a", stop("one")), 120).is_err());
    assert_eq!(fs::read_to_string(dir.0.join("1.json")).unwrap(), "{");
    store
        .record(input("a", session_end(SessionEndReason::Other)), 130)
        .unwrap();
    assert!(fs::read_dir(&dir.0).unwrap().next().is_none());
}

#[test]
fn exit_reports_removal_failures() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone(), 1);
    store
        .record(input("one", session_start("startup")), 100)
        .unwrap();
    fs::remove_file(dir.0.join("1.json")).unwrap();
    fs::create_dir(dir.0.join("1.json")).unwrap();
    assert!(store
        .record(input("one", session_end(SessionEndReason::Other)), 110)
        .is_err());
    assert_eq!(fs::metadata(dir.0.join("1.lock")).unwrap().len(), 0);
}

/// An isolated ccline data directory, Claude settings file and executable path; dropping the
/// returned directory removes them all.
fn installation(executable: &str) -> (TestDir, PathBuf, PathBuf, PathBuf) {
    let dir = TestDir::new();
    let data = dir.0.join("ccline");
    let path = dir.0.join("settings.json");
    let exe = dir.0.join(executable);
    (dir, data, path, exe)
}

fn settings(path: &std::path::Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn installation_is_idempotent_and_uninstall_preserves_other_hooks_and_settings() {
    let (_dir, data, path, exe) = installation("a path with 'quotes'/$dollars/ccline");
    let original = json!({"model":"opus", "statusLine":{"type":"command", "command":"custom-status", "padding":2},
        "hooks":{"SubagentStart":[{"matcher":"Explore", "hooks":[{"type":"command","command":"audit-start"}]}],
            "SessionStart":[{"matcher":"startup", "hooks":[]}],
            "PostToolUse":[{"hooks":[{"type":"command", "command":"formatter"}]}]}});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, true, Backend::Hooks).unwrap();
    let installed = settings(&path);
    let manifest = settings(&data.join("agents-installation.json"));
    assert!(manifest["command"]
        .as_str()
        .unwrap()
        .ends_with(" --agents-hook"));
    assert!(manifest.get("installation_id").is_none());
    assert_eq!(installed["statusLine"]["command"], "custom-status");
    assert_eq!(installed["statusLine"]["refreshInterval"], 2);
    assert!(manifest["command"].as_str().unwrap().contains("'\\''"));
    integration::configure_at(&data, &path, &exe, true, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), installed);
    assert_eq!(settings(&data.join("agents-installation.json")), manifest);
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), original);
    assert!(!data.join("agents-installation.json").exists());
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), original);
}

#[test]
fn uninstall_keeps_user_refresh_changes_and_reinstall_reuses_session_state() {
    let (_dir, data, path, exe) = installation("ccline");
    integration::configure_at(&data, &path, &exe, true, Backend::Hooks).unwrap();
    let first = settings(&data.join("agents-installation.json"));
    let store = ActivityStore::new(data.join("agents"), 1);
    store
        .record(input("session", session_start("startup")), 90)
        .unwrap();
    store
        .record(input("session", start("agent", "Explore")), 100)
        .unwrap();
    let mut updated = settings(&path);
    updated["statusLine"]["refreshInterval"] = json!(5);
    fs::write(&path, serde_json::to_vec(&updated).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
    integration::configure_at(&data, &path, &exe, true, Backend::Hooks).unwrap();
    let second = settings(&data.join("agents-installation.json"));
    assert_eq!(first["command"], second["command"]);
    let reopened = ActivityStore::new(data.join("agents"), 1);
    let current = reopened.read("session").unwrap().unwrap();
    assert_eq!(current.agents.len(), 1);
    assert_eq!(current.agents["agent"].started_at, 100);
    assert!(current.agents["agent"].responded_at.is_none());
    reopened
        .record(input("session", stop("agent")), 120)
        .unwrap();
    assert_eq!(
        reopened.read("session").unwrap().unwrap().agents["agent"].responded_at,
        Some(120)
    );
    reopened
        .record(
            input(
                "session",
                HookEvent::SessionStart {
                    source: "startup".into(),
                },
            ),
            130,
        )
        .unwrap();
    assert!(reopened.read("session").unwrap().unwrap().agents.is_empty());
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
}

#[test]
fn invalid_settings_fail_without_installation_or_file_changes_and_disabled_is_inert() {
    let (_dir, data, path, exe) = installation("ccline");
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert!(!data.exists());
    assert!(!path.exists());
    for invalid in [
        "{",
        "null",
        r#"{"hooks":[]}"#,
        r#"{"hooks":{"SubagentStart":{}}}"#,
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(integration::configure_at(&data, &path, &exe, true, Backend::Hooks).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        assert!(!data.exists());
    }
}

#[test]
fn hiding_agents_leaves_no_separator_or_background_in_any_style() {
    use crate::config::{SegmentId, StyleMode};
    use crate::core::{collect_all_segments, StatusLineGenerator};
    use crate::ui::themes::ThemePresets;
    for mut config in [
        ThemePresets::get_default(),
        ThemePresets::get_powerline_dark(),
    ] {
        for mode in [StyleMode::Plain, StyleMode::NerdFont, StyleMode::Powerline] {
            config.style.mode = mode;
            let mut agents = config
                .segments
                .iter()
                .find(|s| s.id == SegmentId::Agents)
                .unwrap()
                .clone();
            let directory = config
                .segments
                .iter()
                .find(|s| s.id == SegmentId::Directory)
                .unwrap()
                .clone();
            let data = SegmentData {
                primary: "project".into(),
                secondary: String::new(),
                metadata: Default::default(),
            };
            let renderer = StatusLineGenerator::new(config.clone());
            let expected = renderer.generate(vec![(directory.clone(), data.clone().into())]);
            agents.enabled = true;
            for position in 0..=1 {
                let mut segments = vec![(directory.clone(), data.clone().into())];
                if let Some(data) = render(Some(&Activity::default()), 100) {
                    segments.insert(
                        position,
                        (
                            agents.clone(),
                            crate::core::segments::SegmentContent::Agents(data),
                        ),
                    );
                }
                assert_eq!(renderer.generate(segments), expected);
            }
        }
    }
    let mut config = ThemePresets::get_default();
    config.segments.retain(|s| s.id == SegmentId::Agents);
    assert!(
        collect_all_segments(&config, &statusline_input("disabled-test"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn many_active_agents_are_listed_longest_running_first_up_to_the_limit() {
    let mut activity = Activity::default();
    // Start order differs from ID order, and two agents start within the same second.
    activity.apply(start("d", "Plan"), 130);
    activity.apply(start("c", "reviewer"), 100);
    activity.apply(start("f", "code-simplifier"), 100);
    activity.apply(start("b", "Explore"), 160);
    activity.apply(start("a", "general-purpose"), 175);
    activity.apply(start("e", "Explore"), 120);
    activity.apply(stop("e"), 150);
    let render = |max_agents| {
        AgentsSegment::render(Some(&activity), 200, max_agents)
            .unwrap()
            .text(usize::MAX)
    };
    assert_eq!(
        render(AgentsSegment::DEFAULT_MAX_AGENTS),
        "Agents: 5 active · 1 responded · reviewer 1m40s · code-simplifier 1m40s · Plan 1m10s · +2 more"
    );
    assert_eq!(
        render(1),
        "Agents: 5 active · 1 responded · reviewer 1m40s · +4 more"
    );
    assert_eq!(render(0), "Agents: 5 active · 1 responded");
    let complete = "Agents: 5 active · 1 responded · reviewer 1m40s · code-simplifier 1m40s · Plan 1m10s · Explore 40s · general-purpose 25s";
    assert_eq!(render(5), complete);
    assert_eq!(render(usize::MAX), complete);
}

#[test]
fn max_agents_option_defaults_to_three_and_rejects_anything_but_a_count() {
    use crate::config::{Config, SegmentId};
    use crate::core::collect_all_segments;
    use crate::ui::themes::ThemePresets;
    let parse =
        |options: Value| AgentsSegment::max_agents(&serde_json::from_value(options).unwrap());
    assert_eq!(parse(json!({})).unwrap(), 3);
    assert_eq!(parse(json!({"max_agents": 0})).unwrap(), 0);
    assert_eq!(parse(json!({"max_agents": 5})).unwrap(), 5);
    for invalid in [json!(-1), json!(2.5), json!("3"), json!(true), json!([3])] {
        let error = parse(json!({"max_agents": invalid}))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("max_agents") && error.contains(&invalid.to_string()),
            "{error}"
        );
    }
    // Built-in themes carry the default, and edits in config.toml reach the segment.
    let agents_options = |config: &Config| {
        config
            .segments
            .iter()
            .find(|s| s.id == SegmentId::Agents)
            .unwrap()
            .options
            .clone()
    };
    let preset = ThemePresets::get_default();
    let saved = toml::to_string_pretty(&preset).unwrap();
    assert!(saved.contains("max_agents = 3"), "{saved}");
    let reopened: Config = toml::from_str(&saved).unwrap();
    assert_eq!(agents_options(&reopened), agents_options(&preset));
    let edited: Config =
        toml::from_str(&saved.replace("max_agents = 3", "max_agents = 5")).unwrap();
    assert_eq!(
        AgentsSegment::max_agents(&agents_options(&edited)).unwrap(),
        5
    );
    // An invalid option is an explicit error for the status line, not a silent default.
    let mut invalid = edited;
    invalid.segments.retain(|s| s.id == SegmentId::Agents);
    invalid.segments[0].enabled = true;
    invalid.segments[0]
        .options
        .insert("max_agents".into(), json!(-1));
    let error = collect_all_segments(&invalid, &statusline_input("invalid-option")).unwrap_err();
    assert!(error.to_string().contains("max_agents"), "{error}");
}

#[test]
fn preview_lists_sample_agents_with_the_configured_limit_and_shows_option_errors() {
    use crate::config::SegmentId;
    use crate::ui::components::preview::PreviewComponent;
    use crate::ui::themes::ThemePresets;
    let mut config = ThemePresets::get_default();
    let index = config
        .segments
        .iter()
        .position(|s| s.id == SegmentId::Agents)
        .unwrap();
    config.segments[index].enabled = true;
    let mut preview = PreviewComponent::new();
    preview.update_preview_with_width(&config, 500);
    assert!(
        preview
            .get_preview_cache()
            .contains("Agents: 4 active · reviewer 1m20s · Explore 35s · Plan 12s · +1 more"),
        "{}",
        preview.get_preview_cache()
    );
    config.segments[index]
        .options
        .insert("max_agents".into(), json!(5));
    preview.update_preview_with_width(&config, 500);
    assert!(preview.get_preview_cache().contains(
        "Agents: 4 active · reviewer 1m20s · Explore 35s · Plan 12s · general-purpose 5s"
    ));
    config.segments[index]
        .options
        .insert("max_agents".into(), json!("many"));
    preview.update_preview_with_width(&config, 500);
    assert!(preview
        .get_preview_cache()
        .contains("max_agents must be a non-negative integer, not \"many\""));
}

#[test]
fn selection_prefers_the_panel_then_the_opt_in_mod_then_hooks() {
    for experimental_mod in [false, true] {
        let selection = selection::select(None, experimental_mod).unwrap();
        assert_eq!(selection.backend, Backend::Hooks);
        assert!(selection.message.contains("claude was not found"));
        for version in ["2.1.293 (Claude Code)", "2.1.295 (Claude Code)", "2.2.0"] {
            let selection = selection::select(Some(version), experimental_mod).unwrap();
            assert_eq!(selection.backend, Backend::Panel, "{version}");
            assert!(selection.message.starts_with("Subagent panel"));
            assert_eq!(
                selection
                    .message
                    .contains("Experimental Mod applies below 2.1.293"),
                experimental_mod
            );
        }
        assert!(selection::select(Some("not a version"), experimental_mod).is_err());
    }
    for version in [
        "2.1.272 (Claude Code)",
        "2.1.273 (Claude Code)",
        "2.1.293-beta.1",
    ] {
        let selection = selection::select(Some(version), false).unwrap();
        assert_eq!(selection.backend, Backend::Hooks, "{version}");
        assert!(selection
            .message
            .contains("subagent panel requires 2.1.293+"));
    }
    for version in [Some("2.1.272 (Claude Code)"), Some("2.1.273-beta.1")] {
        let selection = selection::select(version, true).unwrap();
        assert_eq!(selection.backend, Backend::Hooks);
        assert!(selection.message.contains("Mod requires 2.1.273+"));
    }
    for version in [
        "2.1.273 (Claude Code)",
        "2.1.292 (Claude Code)",
        "2.1.293-beta.1",
    ] {
        assert_eq!(
            selection::select(Some(version), true).unwrap().backend,
            Backend::Mod,
            "{version}"
        );
    }
    let options = |value| serde_json::from_value(value).unwrap();
    assert!(!AgentsSegment::experimental_mod(&options(json!({}))).unwrap());
    assert!(AgentsSegment::experimental_mod(&options(json!({"experimental_mod":true}))).unwrap());
    assert!(AgentsSegment::experimental_mod(&options(json!({"experimental_mod":"true"}))).is_err());
}

#[test]
fn mod_installation_switches_backends_without_changing_other_integrations() {
    use super::experimental;
    let (dir, data, path, exe) = installation("ccline");
    let original = json!({
        "env":{"CUSTOM":"keep"}, "enabledPlugins":{"other@market":true},
        "statusLine":{"type":"command","command":"existing-status"},
        "hooks":{"SubagentStart":[{"hooks":[{"type":"command","command":"audit"}]}]}
    });
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, true, Backend::Hooks).unwrap();
    let hooks = settings(&path);
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    let native = settings(&path);
    let manifest = settings(&data.join("agents-installation.json"));
    assert_eq!(native["hooks"], original["hooks"]);
    assert_eq!(native["env"][experimental::ENABLE_ENV], "1");
    assert_eq!(native["env"]["CUSTOM"], "keep");
    assert_eq!(native["enabledPlugins"][experimental::PLUGIN_KEY], true);
    assert_eq!(native["enabledPlugins"]["other@market"], true);
    let plugin = dir.0.join("skills").join(experimental::PLUGIN_NAME);
    assert_eq!(
        settings(&plugin.join(".claude-plugin/plugin.json"))["defaultEnabled"],
        false
    );
    assert!(plugin.join("hooks/register.ts").is_file());
    assert!(plugin.join("hooks/activity.ts").is_file());
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    assert_eq!(settings(&path), native);
    assert_eq!(settings(&data.join("agents-installation.json")), manifest);
    integration::configure_at(
        &data,
        &path,
        &exe,
        true,
        selection::select(Some("2.1.272"), true).unwrap().backend,
    )
    .unwrap();
    assert_eq!(settings(&path), hooks);
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), original);
    assert!(!data.join("agents-installation.json").exists());
}

#[test]
fn disabling_mod_restores_owned_values_and_preserves_later_user_edits() {
    use super::experimental;
    let (_dir, data, path, exe) = installation("ccline");
    let original = json!({"env":{experimental::ENABLE_ENV:"0"}, "enabledPlugins":{experimental::PLUGIN_KEY:false}, "statusLine":{"command":"custom","type":"command"}});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), original);
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    let mut edited = settings(&path);
    edited["env"][experimental::ENABLE_ENV] = json!("false");
    edited["enabledPlugins"][experimental::PLUGIN_KEY] = json!(false);
    fs::write(&path, serde_json::to_vec(&edited).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path)["env"][experimental::ENABLE_ENV], "false");
    assert_eq!(
        settings(&path)["enabledPlugins"][experimental::PLUGIN_KEY],
        false
    );
}

#[test]
fn mod_snapshot_uses_the_requested_session_and_hides_unavailable_activity() {
    use super::experimental::parse_snapshot;
    let snapshot = json!({"sessions":{
        "one":{"agents":{"a":{"agent_type":"Explore","started_at":100,"responded_at":null}},"ended":false},
        "two":{"agents":{},"ended":true}
    }}).to_string();
    assert!(
        render(Some(&parse_snapshot(&snapshot, "one").unwrap()), 110)
            .unwrap()
            .text(3)
            .contains("Explore 10s")
    );
    assert!(render(Some(&parse_snapshot(&snapshot, "two").unwrap()), 110).is_none());
    assert!(parse_snapshot(&snapshot, "missing").is_none());
    for malformed in [
        "",
        "{",
        "{}",
        "null",
        r#"{"sessions":{"one":{"agents":{}}}}"#,
    ] {
        assert!(parse_snapshot(malformed, "one").is_none());
        assert!(render(parse_snapshot(malformed, "one").as_ref(), 110).is_none());
    }
}

#[test]
fn panel_replaces_lifecycle_hooks_and_restores_only_owned_settings() {
    let (_dir, data, path, exe) = installation("a path with 'quotes'/ccline");
    let original = json!({"model":"opus", "statusLine":{"type":"command","command":"custom-status"},
        "hooks":{"SubagentStart":[{"hooks":[{"type":"command","command":"audit"}]}]}});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let configure =
        |enabled, backend| integration::configure_at(&data, &path, &exe, enabled, backend).unwrap();
    assert_eq!(configure(true, Backend::Hooks), Backend::Hooks);
    let hooks = settings(&path);
    assert_eq!(configure(true, Backend::Panel), Backend::Panel);
    let panel = settings(&path);
    let manifest = settings(&data.join("agents-installation.json"));
    assert_eq!(panel["hooks"], original["hooks"]);
    assert!(panel["statusLine"].get("refreshInterval").is_none());
    assert_eq!(panel["subagentStatusLine"]["type"], "command");
    let command = panel["subagentStatusLine"]["command"].as_str().unwrap();
    assert!(command.ends_with(" --subagent-statusline"));
    assert!(command.contains("'\\''"));
    assert_eq!(manifest["subagent_status_line"], command);
    assert!(integration::read_activity_at(&data, "session")
        .unwrap()
        .is_none());
    assert_eq!(configure(true, Backend::Panel), Backend::Panel);
    assert_eq!(settings(&path), panel);
    assert_eq!(settings(&data.join("agents-installation.json")), manifest);
    assert_eq!(configure(true, Backend::Hooks), Backend::Hooks);
    assert_eq!(settings(&path), hooks);
    assert!(settings(&data.join("agents-installation.json"))
        .get("subagent_status_line")
        .is_none());
    configure(true, Backend::Panel);
    configure(false, Backend::Hooks);
    assert_eq!(settings(&path), original);
    assert!(!data.join("agents-installation.json").exists());
}

#[test]
fn a_user_subagent_status_line_is_kept_and_selects_hooks() {
    let (_dir, data, path, exe) = installation("ccline");
    let mine = json!({"type":"command","command":"my-rows"});
    fs::write(
        &path,
        serde_json::to_vec(&json!({ "subagentStatusLine": mine })).unwrap(),
    )
    .unwrap();
    let configure =
        |enabled, backend| integration::configure_at(&data, &path, &exe, enabled, backend).unwrap();
    assert_eq!(configure(true, Backend::Panel), Backend::Hooks);
    let installed = settings(&path);
    assert_eq!(installed["subagentStatusLine"], mine);
    assert_eq!(
        installed["hooks"]["SubagentStart"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(installed["statusLine"]["refreshInterval"], 2);
    configure(false, Backend::Hooks);
    let uninstalled = settings(&path);
    assert_eq!(uninstalled["subagentStatusLine"], mine);
    assert!(uninstalled.get("hooks").is_none());
    assert!(uninstalled["statusLine"].get("refreshInterval").is_none());

    // A later edit to ccline's own value also makes it the user's.
    fs::write(&path, "{}").unwrap();
    assert_eq!(configure(true, Backend::Panel), Backend::Panel);
    let mut edited = settings(&path);
    edited["subagentStatusLine"]["command"] = json!("edited");
    fs::write(&path, serde_json::to_vec(&edited).unwrap()).unwrap();
    assert_eq!(configure(true, Backend::Panel), Backend::Hooks);
    assert_eq!(settings(&path)["subagentStatusLine"]["command"], "edited");
    configure(false, Backend::Hooks);
    assert_eq!(settings(&path)["subagentStatusLine"]["command"], "edited");
}

#[test]
fn switching_from_the_mod_to_the_panel_restores_mod_settings() {
    use super::experimental;
    let (_dir, data, path, exe) = installation("ccline");
    let original =
        json!({"env":{"CUSTOM":"keep"}, "statusLine":{"type":"command","command":"custom"}});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, true, Backend::Mod).unwrap();
    assert_eq!(settings(&path)["env"][experimental::ENABLE_ENV], "1");
    integration::configure_at(&data, &path, &exe, true, Backend::Panel).unwrap();
    let panel = settings(&path);
    assert_eq!(panel["env"], original["env"]);
    assert!(panel.get("enabledPlugins").is_none());
    assert!(panel.get("hooks").is_none());
    assert!(settings(&data.join("agents-installation.json"))
        .get("mod_settings")
        .is_none());
    integration::configure_at(&data, &path, &exe, false, Backend::Hooks).unwrap();
    assert_eq!(settings(&path), original);
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
