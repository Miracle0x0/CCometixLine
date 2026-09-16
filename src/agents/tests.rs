use super::{
    integration,
    store::{Activity, ActivityStore},
    HookEvent, HookInput,
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
    activity.apply(HookEvent::SessionEnd, 160);
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
    assert!(render(store.read("one").unwrap().as_ref(), 130).is_none());
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
    let installation_id = manifest["installation_id"].as_str().unwrap();
    installation_id
        .parse::<integration::InstallationId>()
        .unwrap();
    assert!(manifest["command"].as_str().unwrap().ends_with(&format!(
        "--agents-hook --installation-id {installation_id}"
    )));
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
    assert_ne!(first["installation_id"], second["installation_id"]);
    // A delayed event from the old hook writes only to its original installation.
    let old_store = ActivityStore::new(
        data.join("agents")
            .join(first["installation_id"].as_str().unwrap()),
    );
    let new_store = ActivityStore::new(
        data.join("agents")
            .join(second["installation_id"].as_str().unwrap()),
    );
    old_store
        .record(input("session", start("old", "Explore")), 100)
        .unwrap();
    new_store
        .record(input("session", start("new", "Plan")), 110)
        .unwrap();
    old_store
        .record(input("session", stop("old")), 120)
        .unwrap();
    let current = new_store.read("session").unwrap().unwrap();
    assert_eq!(current.agents.len(), 1);
    assert!(current.agents["new"].responded_at.is_none());
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
