use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Default)]
pub struct UsageSegment;

impl UsageSegment {
    pub fn new() -> Self {
        Self
    }

    fn get_circle_icon(percentage: f64) -> &'static str {
        match percentage {
            p if p < 13.0 => "\u{f0a9e}", // circle_slice_1
            p if p < 26.0 => "\u{f0a9f}", // circle_slice_2
            p if p < 38.0 => "\u{f0aa0}", // circle_slice_3
            p if p < 51.0 => "\u{f0aa1}", // circle_slice_4
            p if p < 63.0 => "\u{f0aa2}", // circle_slice_5
            p if p < 76.0 => "\u{f0aa3}", // circle_slice_6
            p if p < 88.0 => "\u{f0aa4}", // circle_slice_7
            _ => "\u{f0aa5}",             // circle_slice_8
        }
    }

    fn format_reset_time(reset_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> String {
        let Some(reset_at) = reset_at else {
            return "?".to_string();
        };
        let remaining = reset_at.signed_duration_since(now);
        if remaining <= Duration::zero() {
            "?".to_string()
        } else if remaining.num_days() > 0 {
            format!("{}d", remaining.num_days())
        } else if remaining.num_hours() > 0 {
            format!("{}h", remaining.num_hours())
        } else {
            format!("{}m", remaining.num_minutes())
        }
    }
}

impl Segment for UsageSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let rate_limits = input.rate_limits.as_ref()?;
        if rate_limits.five_hour.is_none() && rate_limits.seven_day.is_none() {
            return None;
        }

        let mut metadata = HashMap::new();
        let (primary, secondary) = match &rate_limits.five_hour {
            Some(window) => {
                metadata.insert(
                    "five_hour_utilization".to_string(),
                    window.used_percentage.to_string(),
                );
                (
                    format!("{}%", window.used_percentage.round()),
                    format!(
                        "· {}",
                        Self::format_reset_time(window.resets_at, Utc::now())
                    ),
                )
            }
            None => ("?%".to_string(), "· ?".to_string()),
        };
        let dynamic_icon = match &rate_limits.seven_day {
            Some(window) => {
                metadata.insert(
                    "seven_day_utilization".to_string(),
                    window.used_percentage.to_string(),
                );
                Self::get_circle_icon(window.used_percentage)
            }
            None => "?",
        };
        metadata.insert("dynamic_icon".to_string(), dynamic_icon.to_string());

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
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
            "session_id": "usage-test",
            "model": { "id": "claude-opus-4-6", "display_name": "Opus 4.6" },
            "workspace": { "current_dir": "/work/project" },
            "transcript_path": "/nonexistent/session.jsonl"
        })
    }

    fn input(rate_limits: Value) -> InputData {
        let mut value = payload();
        value["rate_limits"] = rate_limits;
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn collects_stdin_usage_and_uses_the_five_hour_reset() {
        let now = Utc::now();
        let input = input(json!({
            "five_hour": { "used_percentage": 23.5, "resets_at": (now + Duration::minutes(150)).timestamp() },
            "seven_day": { "used_percentage": 63.0, "resets_at": (now + Duration::days(3)).timestamp() }
        }));
        let data = UsageSegment::new().collect(&input).unwrap();
        assert_eq!(data.primary, "24%");
        assert_eq!(data.secondary, "· 2h");
        assert_eq!(data.metadata["five_hour_utilization"], "23.5");
        assert_eq!(data.metadata["seven_day_utilization"], "63");
        assert_eq!(data.metadata["dynamic_icon"], "\u{f0aa3}");
    }

    #[test]
    fn absent_windows_do_not_fabricate_zero_usage() {
        let segment = UsageSegment::new();
        assert!(segment
            .collect(&serde_json::from_value(payload()).unwrap())
            .is_none());
        for limits in [
            Value::Null,
            json!({}),
            json!({"five_hour": null, "seven_day": null}),
        ] {
            assert!(segment.collect(&input(limits)).is_none());
        }

        let five_hour_only = segment
            .collect(&input(json!({"five_hour": {"used_percentage": 0.0}})))
            .unwrap();
        assert_eq!(five_hour_only.primary, "0%");
        assert_eq!(five_hour_only.secondary, "· ?");
        assert_eq!(five_hour_only.metadata["dynamic_icon"], "?");
        assert!(!five_hour_only
            .metadata
            .contains_key("seven_day_utilization"));

        let seven_day_only = segment
            .collect(&input(json!({"seven_day": {"used_percentage": 80.0}})))
            .unwrap();
        assert_eq!(seven_day_only.primary, "?%");
        assert_eq!(seven_day_only.secondary, "· ?");
        assert_eq!(seven_day_only.metadata["dynamic_icon"], "\u{f0aa4}");
        assert!(!seven_day_only
            .metadata
            .contains_key("five_hour_utilization"));
    }

    #[test]
    fn percentage_is_rounded_without_integer_saturation() {
        for (percentage, expected) in [
            (0.0, "0%"),
            (23.5, "24%"),
            (100.0, "100%"),
            (300.0, "300%"),
            (-5.0, "-5%"),
        ] {
            let data = UsageSegment::new()
                .collect(&input(
                    json!({"five_hour": {"used_percentage": percentage}}),
                ))
                .unwrap();
            assert_eq!(data.primary, expected);
        }
    }

    #[test]
    fn reset_countdown_handles_signed_epochs_and_unit_boundaries() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        for (seconds, expected) in [
            (-1, "?"),
            (0, "?"),
            (1, "0m"),
            (59, "0m"),
            (60, "1m"),
            (3599, "59m"),
            (3600, "1h"),
            (86399, "23h"),
            (86400, "1d"),
            (259200, "3d"),
        ] {
            assert_eq!(
                UsageSegment::format_reset_time(Some(now + Duration::seconds(seconds)), now),
                expected
            );
        }
        assert_eq!(UsageSegment::format_reset_time(None, now), "?");
        for epoch in [-1, 0] {
            let input = input(json!({"five_hour": {"used_percentage": 24.0, "resets_at": epoch}}));
            assert_eq!(
                UsageSegment::new().collect(&input).unwrap().secondary,
                "· ?"
            );
        }
    }

    #[test]
    fn malformed_windows_are_deserialization_errors() {
        for window in [
            json!({}),
            json!({"used_percentage": "24"}),
            json!({"used_percentage": 24, "resets_at": "tomorrow"}),
            json!({"used_percentage": 24, "resets_at": i64::MAX}),
        ] {
            let mut value = payload();
            value["rate_limits"] = json!({"five_hour": window});
            assert!(serde_json::from_value::<InputData>(value).is_err());
        }
    }

    #[test]
    fn rendering_respects_the_active_config_and_hides_absent_usage() {
        let present = input(json!({
            "five_hour": {"used_percentage": 24.0},
            "seven_day": {"used_percentage": 63.0}
        }));
        let absent = input(Value::Null);
        for mode in [StyleMode::Plain, StyleMode::NerdFont, StyleMode::Powerline] {
            let mut config = ThemePresets::get_default();
            config.style.mode = mode;
            config.segments.retain(|s| s.id == SegmentId::Usage);
            let disabled = collect_all_segments(&config, &present).unwrap();
            assert!(disabled.is_empty());

            config.segments[0].enabled = true;
            let config: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            let renderer = StatusLineGenerator::new(config.clone());
            let rendered = renderer.generate(collect_all_segments(&config, &present).unwrap());
            assert!(rendered.contains("24%"), "{rendered}");
            assert!(rendered.contains("· ?"), "{rendered}");
            if mode != StyleMode::Plain {
                assert!(rendered.contains('\u{f0aa3}'), "{rendered}");
            }
            assert_eq!(
                renderer.generate(collect_all_segments(&config, &absent).unwrap()),
                ""
            );
        }
    }
}
