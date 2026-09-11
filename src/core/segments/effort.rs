use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct EffortSegment;

impl EffortSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for EffortSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let effort = input.effort.as_ref()?;

        Some(SegmentData {
            primary: effort.level.clone(),
            secondary: String::new(),
            metadata: HashMap::new(),
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Effort
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, StyleMode};
    use crate::core::{collect_all_segments, StatusLineGenerator};
    use crate::ui::themes::ThemePresets;
    use serde_json::{json, Value};

    fn payload() -> Value {
        json!({
            "session_id": "effort-test",
            "model": { "id": "claude-opus-4-6", "display_name": "Opus 4.6" },
            "workspace": { "current_dir": "/work/project" },
            "transcript_path": "/nonexistent/session.jsonl",
            "output_style": { "name": "default" },
            "thinking": { "enabled": false }
        })
    }

    fn effort_config() -> Config {
        let mut config = ThemePresets::get_default();
        config.segments.retain(|s| s.id == SegmentId::Effort);
        config
    }

    #[test]
    fn renders_each_reported_level_independently_of_thinking() {
        let config = effort_config();
        let renderer = StatusLineGenerator::new(config.clone());
        for level in ["low", "medium", "high", "xhigh", "max", "low"] {
            let mut value = payload();
            value["effort"] = json!({ "level": level });
            let input: InputData = serde_json::from_value(value).unwrap();
            let data = collect_all_segments(&config, &input).unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].1.primary, level);
            assert!(renderer.generate(data).contains(level));
        }
        let input = serde_json::from_value(payload()).unwrap();
        assert_eq!(
            renderer.generate(collect_all_segments(&config, &input).unwrap()),
            ""
        );
    }

    #[test]
    fn rejects_malformed_effort_input() {
        for effort in [json!({}), json!({ "level": 3 }), json!("high"), json!([])] {
            let mut value = payload();
            value["effort"] = effort;
            assert!(serde_json::from_value::<InputData>(value).is_err());
        }
    }

    #[test]
    fn disabled_effort_produces_no_output_after_config_round_trip() {
        let mut config = effort_config();
        config.segments[0].enabled = false;
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("id = \"effort\""));
        let restored: Config = toml::from_str(&serialized).unwrap();
        let mut value = payload();
        value["effort"] = json!({ "level": "high" });
        let input = serde_json::from_value(value).unwrap();
        let data = collect_all_segments(&restored, &input).unwrap();
        assert!(data.is_empty());
        assert_eq!(StatusLineGenerator::new(restored).generate(data), "");
    }

    #[test]
    fn absent_effort_leaves_no_separator_or_background_at_any_position() {
        let input = serde_json::from_value(payload()).unwrap();
        for mut base in [
            ThemePresets::get_default(),
            ThemePresets::get_powerline_dark(),
        ] {
            for mode in [StyleMode::Plain, StyleMode::NerdFont, StyleMode::Powerline] {
                base.style.mode = mode;
                for position in 0..=2 {
                    let mut config = base.clone();
                    let effort = config
                        .segments
                        .iter()
                        .find(|s| s.id == SegmentId::Effort)
                        .unwrap()
                        .clone();
                    config
                        .segments
                        .retain(|s| matches!(s.id, SegmentId::Directory | SegmentId::OutputStyle));
                    for segment in &mut config.segments {
                        segment.enabled = true;
                    }
                    let expected = StatusLineGenerator::new(config.clone())
                        .generate(collect_all_segments(&config, &input).unwrap());
                    config.segments.insert(position, effort);
                    let actual = StatusLineGenerator::new(config.clone())
                        .generate(collect_all_segments(&config, &input).unwrap());
                    assert_eq!(actual, expected, "position {position}, mode {mode:?}");
                }
            }
        }
    }
}
