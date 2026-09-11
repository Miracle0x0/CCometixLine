use super::{
    integration,
    store::{Activity, ActivityStore},
    HookEvent, HookInput,
};
use crate::core::segments::AgentsSegment;
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
fn input(session: &str, event: HookEvent) -> HookInput {
    HookInput {
        session_id: session.into(),
        event,
    }
}

#[test]
fn duplicate_events_and_resuming_preserve_identity_and_elapsed_time() {
    let mut activity = Activity::default();
    activity.apply(start("a", "Explore"), 100);
    activity.apply(start("a", "Explore"), 110);
    activity.apply(start("b", "reviewer"), 115);
    let text = AgentsSegment::render(Some(&activity), 180).unwrap().primary;
    assert_eq!(text, "Agents: 2 active · Explore 1m20s · reviewer 1m05s");
    activity.apply(stop("a"), 190);
    activity.apply(stop("a"), 200);
    activity.apply(stop("unobserved"), 200);
    assert_eq!(activity.agents.len(), 2);
    assert_eq!(activity.agents["a"].responded_at, Some(190));
    assert!(AgentsSegment::render(Some(&activity), 201)
        .unwrap()
        .primary
        .contains("1 active · 1 responded"));
    activity.apply(start("a", "Explore"), 250);
    assert_eq!(activity.agents["a"].started_at, 250);
    assert_eq!(activity.agents["a"].responded_at, None);
}

#[test]
fn idle_and_ended_sessions_hide_the_entire_segment() {
    let mut activity = Activity::default();
    assert!(AgentsSegment::render(None, 100).is_none());
    assert!(AgentsSegment::render(Some(&activity), 100).is_none());
    activity.apply(start("a", "Explore"), 100);
    activity.apply(stop("a"), 130);
    assert!(AgentsSegment::render(Some(&activity), 140).is_none());
    activity.apply(start("a", "Explore"), 150);
    activity.apply(HookEvent::SessionEnd, 160);
    activity.apply(start("late", "Explore"), 170);
    assert!(AgentsSegment::render(Some(&activity), 180).is_none());
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
}

#[test]
fn store_serializes_concurrent_writers_and_isolates_sessions() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone());
    assert!(store.read("one").unwrap().is_none());
    assert!(fs::read_dir(&dir.0).unwrap().next().is_none());
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
        .record(input("../../two", start("other", "Plan")), 101)
        .unwrap();
    assert_eq!(store.read("../../two").unwrap().unwrap().agents.len(), 1);
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 4);
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
    assert!(AgentsSegment::render(store.read("one").unwrap().as_ref(), 130).is_none());
}

#[test]
fn corrupt_activity_is_reported_and_not_replaced_by_empty_state() {
    let dir = TestDir::new();
    let store = ActivityStore::new(dir.0.clone());
    store
        .record(input("a", start("one", "Explore")), 100)
        .unwrap();
    fs::write(dir.0.join("61.json"), "{").unwrap();
    assert!(store.read("a").is_err());
    assert!(store.record(input("a", stop("one")), 120).is_err());
    assert_eq!(fs::read_to_string(dir.0.join("61.json")).unwrap(), "{");
}

fn settings(path: &std::path::Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn installation_is_idempotent_and_uninstall_preserves_other_hooks_and_settings() {
    let dir = TestDir::new();
    let data = dir.0.join("ccline");
    let path = dir.0.join("settings.json");
    let exe = dir.0.join("a path with 'quotes'/$dollars/ccline");
    let original = json!({"model":"opus", "statusLine":{"type":"command", "command":"custom-status", "padding":2},
        "hooks":{"SubagentStart":[{"matcher":"Explore", "hooks":[{"type":"command","command":"audit-start"}]}],
            "SessionStart":[{"matcher":"startup", "hooks":[]}],
            "PostToolUse":[{"hooks":[{"type":"command", "command":"formatter"}]}]}});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, true).unwrap();
    let installed = settings(&path);
    let manifest = settings(&data.join("agents-installation.json"));
    assert_eq!(installed["statusLine"]["command"], "custom-status");
    assert_eq!(installed["statusLine"]["refreshInterval"], 2);
    assert!(manifest["command"].as_str().unwrap().contains("'\\''"));
    integration::configure_at(&data, &path, &exe, true).unwrap();
    assert_eq!(settings(&path), installed);
    assert_eq!(settings(&data.join("agents-installation.json")), manifest);
    integration::configure_at(&data, &path, &exe, false).unwrap();
    assert_eq!(settings(&path), original);
    assert!(!data.join("agents-installation.json").exists());
    integration::configure_at(&data, &path, &exe, false).unwrap();
    assert_eq!(settings(&path), original);
}

#[test]
fn uninstall_keeps_user_refresh_changes_and_a_new_install_has_fresh_state() {
    let dir = TestDir::new();
    let data = dir.0.join("ccline");
    let path = dir.0.join("settings.json");
    let exe = dir.0.join("ccline");
    integration::configure_at(&data, &path, &exe, true).unwrap();
    let first = settings(&data.join("agents-installation.json"));
    let mut updated = settings(&path);
    updated["statusLine"]["refreshInterval"] = json!(5);
    fs::write(&path, serde_json::to_vec(&updated).unwrap()).unwrap();
    integration::configure_at(&data, &path, &exe, false).unwrap();
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
    integration::configure_at(&data, &path, &exe, true).unwrap();
    let second = settings(&data.join("agents-installation.json"));
    assert_ne!(first["generation"], second["generation"]);
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
    integration::configure_at(&data, &path, &exe, false).unwrap();
    assert_eq!(settings(&path)["statusLine"]["refreshInterval"], 5);
}

#[test]
fn invalid_settings_fail_without_installation_or_file_changes_and_disabled_is_inert() {
    let dir = TestDir::new();
    let data = dir.0.join("ccline");
    let path = dir.0.join("settings.json");
    let exe = dir.0.join("ccline");
    integration::configure_at(&data, &path, &exe, false).unwrap();
    assert!(!data.exists());
    assert!(!path.exists());
    for invalid in [
        "{",
        "null",
        r#"{"hooks":[]}"#,
        r#"{"hooks":{"SubagentStart":{}}}"#,
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(integration::configure_at(&data, &path, &exe, true).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        assert!(!data.exists());
    }
}

#[test]
fn hiding_agents_leaves_no_separator_or_background_in_any_style() {
    use crate::config::{InputData, SegmentId, StyleMode};
    use crate::core::segments::SegmentData;
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
            let expected = renderer.generate(vec![(directory.clone(), data.clone())]);
            agents.enabled = true;
            for position in 0..=1 {
                let mut segments = vec![(directory.clone(), data.clone())];
                if let Some(data) = AgentsSegment::render(Some(&Activity::default()), 100) {
                    segments.insert(position, (agents.clone(), data));
                }
                assert_eq!(renderer.generate(segments), expected);
            }
        }
    }
    let mut config = ThemePresets::get_default();
    config.segments.retain(|s| s.id == SegmentId::Agents);
    let input: InputData = serde_json::from_value(json!({"session_id":"disabled-test",
        "model":{"id":"test","display_name":"test"},"workspace":{"current_dir":"/"},"transcript_path":"unused"})).unwrap();
    assert!(collect_all_segments(&config, &input).unwrap().is_empty());
}
