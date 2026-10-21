use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId, SpendLimit, SpendPeriod};
use crate::core::format::format_reset_time;
use chrono::{DateTime, Utc};
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
}

impl UsageSegment {
    fn format_usd(amount: f64) -> String {
        if amount.fract() == 0.0 {
            format!("${amount:.0}")
        } else {
            format!("${amount:.2}")
        }
    }

    /// Gateway budget text such as `spend 63% · $314.12/$500 monthly`, plus its reset countdown.
    /// The dollar amounts and period arrive separately from the percentage, so each is optional.
    fn spend_limit_text(spend: &SpendLimit, now: DateTime<Utc>) -> (String, String) {
        let mut text = format!("spend {}%", spend.used_percentage.round());
        let amounts = match (spend.used_usd, spend.limit_usd) {
            (Some(used), Some(limit)) => Some(format!(
                "{}/{}",
                Self::format_usd(used),
                Self::format_usd(limit)
            )),
            (Some(used), None) => Some(format!("{} used", Self::format_usd(used))),
            (None, Some(limit)) => Some(format!("{} limit", Self::format_usd(limit))),
            (None, None) => None,
        };
        let period = spend.period.map(|period| match period {
            SpendPeriod::Daily => "daily",
            SpendPeriod::Weekly => "weekly",
            SpendPeriod::Monthly => "monthly",
        });
        match (amounts, period) {
            (Some(amounts), Some(period)) => text.push_str(&format!(" · {amounts} {period}")),
            (Some(amounts), None) => text.push_str(&format!(" · {amounts}")),
            (None, Some(period)) => text.push_str(&format!(" · {period}")),
            (None, None) => {}
        }
        (text, format_reset_time(Some(spend.resets_at), now))
    }
}

impl Segment for UsageSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let rate_limits = input.rate_limits.as_ref()?;
        let now = Utc::now();
        let mut metadata = HashMap::new();
        let spend = rate_limits.spend_limit.as_ref().map(|spend| {
            metadata.insert(
                "spend_limit_utilization".to_string(),
                spend.used_percentage.to_string(),
            );
            (Self::spend_limit_text(spend, now), spend.used_percentage)
        });

        if rate_limits.five_hour.is_none() && rate_limits.seven_day.is_none() {
            // A gateway budget alone is not a 5-hour or 7-day window.
            let ((text, reset), percentage) = spend?;
            metadata.insert(
                "dynamic_icon".to_string(),
                Self::get_circle_icon(percentage).to_string(),
            );
            return Some(SegmentData {
                primary: text,
                secondary: format!("· {reset}"),
                metadata,
            });
        }

        let (five_hour_percentage, mut secondary) = match &rate_limits.five_hour {
            Some(window) => {
                metadata.insert(
                    "five_hour_utilization".to_string(),
                    window.used_percentage.to_string(),
                );
                (
                    format!("{}%", window.used_percentage.round()),
                    format!("· {}", format_reset_time(window.resets_at, now)),
                )
            }
            None => ("?%".to_string(), "· ?".to_string()),
        };
        let (dynamic_icon, seven_day_percentage) = match &rate_limits.seven_day {
            Some(window) => {
                metadata.insert(
                    "seven_day_utilization".to_string(),
                    window.used_percentage.to_string(),
                );
                (
                    Self::get_circle_icon(window.used_percentage),
                    format!("{}%", window.used_percentage.round()),
                )
            }
            None => ("?", "?%".to_string()),
        };
        metadata.insert("dynamic_icon".to_string(), dynamic_icon.to_string());
        if let Some(((text, reset), _)) = spend {
            secondary.push_str(&format!(" · {text} · {reset}"));
        }

        Some(SegmentData {
            primary: format!("{seven_day_percentage} · {five_hour_percentage}"),
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
    use crate::config::{Config, ModelConfig, StyleMode};
    use crate::core::{collect_all_segments, StatusLineGenerator};
    use crate::ui::themes::ThemePresets;
    use chrono::Duration;
    use serde_json::{json, Value};

    use crate::core::segments::test_payload as payload;

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
        assert_eq!(data.primary, "63% · 24%");
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
            json!({"five_hour": null, "seven_day": null, "spend_limit": null}),
        ] {
            assert!(segment.collect(&input(limits)).is_none());
        }

        let five_hour_only = segment
            .collect(&input(json!({"five_hour": {"used_percentage": 0.0}})))
            .unwrap();
        assert_eq!(five_hour_only.primary, "?% · 0%");
        assert_eq!(five_hour_only.secondary, "· ?");
        assert_eq!(five_hour_only.metadata["dynamic_icon"], "?");
        assert!(!five_hour_only
            .metadata
            .contains_key("seven_day_utilization"));

        let seven_day_only = segment
            .collect(&input(json!({"seven_day": {"used_percentage": 80.0}})))
            .unwrap();
        assert_eq!(seven_day_only.primary, "80% · ?%");
        assert_eq!(seven_day_only.secondary, "· ?");
        assert_eq!(seven_day_only.metadata["dynamic_icon"], "\u{f0aa4}");
        assert!(!seven_day_only
            .metadata
            .contains_key("five_hour_utilization"));
    }

    #[test]
    fn a_gateway_spend_limit_alone_is_shown_as_a_budget() {
        let now = Utc::now();
        let resets_at = (now + Duration::days(12) + Duration::minutes(5)).timestamp();
        let segment = UsageSegment::new();
        let full = segment
            .collect(&input(json!({"spend_limit": {
                "used_percentage": 62.8, "resets_at": resets_at,
                "used_usd": 314.12, "limit_usd": 500, "period": "monthly"
            }})))
            .unwrap();
        assert_eq!(full.primary, "spend 63% · $314.12/$500 monthly");
        assert_eq!(full.secondary, "· 12d");
        assert_eq!(full.metadata["spend_limit_utilization"], "62.8");
        assert_eq!(full.metadata["dynamic_icon"], "\u{f0aa2}");
        assert!(!full.metadata.contains_key("five_hour_utilization"));
        assert!(!full.metadata.contains_key("seven_day_utilization"));

        // The dollar amounts and period arrive later than the percentage, and independently.
        for (extra, expected) in [
            (json!({}), "spend 63%"),
            (json!({"used_usd": 314.12}), "spend 63% · $314.12 used"),
            (json!({"limit_usd": 500}), "spend 63% · $500 limit"),
            (json!({"period": "daily"}), "spend 63% · daily"),
            (
                json!({"used_usd": 9.5, "limit_usd": 20.25, "period": "weekly"}),
                "spend 63% · $9.50/$20.25 weekly",
            ),
        ] {
            let mut spend = json!({"used_percentage": 62.8, "resets_at": resets_at});
            spend
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let data = segment
                .collect(&input(json!({ "spend_limit": spend })))
                .unwrap();
            assert_eq!(data.primary, expected);
            assert_eq!(data.secondary, "· 12d");
        }
    }

    #[test]
    fn a_spend_limit_beside_subscription_windows_keeps_both_meanings() {
        let now = Utc::now();
        let data = UsageSegment::new()
            .collect(&input(json!({
                "five_hour": { "used_percentage": 24.0, "resets_at": (now + Duration::minutes(150)).timestamp() },
                "seven_day": { "used_percentage": 63.0 },
                "spend_limit": { "used_percentage": 110.0, "resets_at": (now + Duration::hours(5) + Duration::minutes(1)).timestamp(), "limit_usd": 50 }
            })))
            .unwrap();
        assert_eq!(data.primary, "63% · 24%");
        assert_eq!(data.secondary, "· 2h · spend 110% · $50 limit · 5h");
        assert_eq!(data.metadata["dynamic_icon"], "\u{f0aa3}");
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
                .collect(&input(json!({
                    "five_hour": {"used_percentage": percentage},
                    "seven_day": {"used_percentage": percentage}
                })))
                .unwrap();
            assert_eq!(data.primary, format!("{expected} · {expected}"));
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
                format_reset_time(Some(now + Duration::seconds(seconds)), now),
                expected
            );
        }
        assert_eq!(format_reset_time(None, now), "?");
        for epoch in [-1, 0] {
            let input = input(json!({"five_hour": {"used_percentage": 24.0, "resets_at": epoch}}));
            assert_eq!(
                UsageSegment::new().collect(&input).unwrap().secondary,
                "· ?"
            );
        }
    }

    #[test]
    fn malformed_windows_and_spend_limits_are_deserialization_errors() {
        for (key, window) in [
            ("five_hour", json!({})),
            ("five_hour", json!({"used_percentage": "24"})),
            (
                "five_hour",
                json!({"used_percentage": 24, "resets_at": "tomorrow"}),
            ),
            (
                "five_hour",
                json!({"used_percentage": 24, "resets_at": i64::MAX}),
            ),
            ("spend_limit", json!({})),
            ("spend_limit", json!({"used_percentage": 50})),
            ("spend_limit", json!({"resets_at": 1_800_000_000})),
            (
                "spend_limit",
                json!({"used_percentage": 50, "resets_at": 1_800_000_000, "period": "yearly"}),
            ),
            (
                "spend_limit",
                json!({"used_percentage": 50, "resets_at": 1_800_000_000, "used_usd": "3"}),
            ),
        ] {
            let mut value = payload();
            value["rate_limits"] = json!({ key: window });
            assert!(
                serde_json::from_value::<InputData>(value).is_err(),
                "{key}: {window}"
            );
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
            let disabled =
                collect_all_segments(&config, &ModelConfig::default(), &present).unwrap();
            assert!(disabled.is_empty());

            config.segments[0].enabled = true;
            let config: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            let renderer = StatusLineGenerator::new(config.clone());
            let rendered = renderer.generate(
                collect_all_segments(&config, &ModelConfig::default(), &present).unwrap(),
            );
            assert!(rendered.contains("63% · 24%"), "{rendered}");
            assert!(rendered.contains("· ?"), "{rendered}");
            if mode != StyleMode::Plain {
                assert!(rendered.contains('\u{f0aa3}'), "{rendered}");
            }
            assert_eq!(
                renderer.generate(
                    collect_all_segments(&config, &ModelConfig::default(), &absent).unwrap()
                ),
                ""
            );
        }
    }
}
