use super::{Segment, SegmentData};
use crate::config::{InputData, PromptCache, SegmentId};
use crate::core::format::{format_reset_time, printable};
use chrono::{DateTime, Utc};

pub struct PromptCacheSegment;

impl Segment for PromptCacheSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        input
            .prompt_cache
            .as_ref()
            .map(|cache| format_cache(cache, Utc::now()))
    }

    fn id(&self) -> SegmentId {
        SegmentId::PromptCache
    }
}

fn format_cache(cache: &PromptCache, now: DateTime<Utc>) -> SegmentData {
    let primary = cache
        .hit_ratio
        .map_or_else(|| "?".into(), |ratio| format!("{:.0}%", ratio * 100.0));
    // Expiry can pass between responses while refreshInterval keeps the countdown moving.
    let warm = cache.warm && cache.expires_at.is_none_or(|expiry| expiry > now);
    let state = if !cache.caching_observed {
        "unobserved"
    } else if warm {
        "warm"
    } else {
        "cold"
    };
    let mut details = vec![state.to_string()];
    if warm && cache.expires_at.is_some() {
        details.push(format_reset_time(cache.expires_at, now));
    }
    if cache.misses > 0 {
        let noun = if cache.misses == 1 { "miss" } else { "misses" };
        details.push(format!("{} {noun}", cache.misses));
        if let Some(cause) = &cache.last_miss_cause {
            if !cause.causes.is_empty() {
                details.push(format!("last: {}", printable(&cause.causes.join(", "))));
            }
        }
    }
    SegmentData {
        primary,
        secondary: format!("· {}", details.join(" · ")),
        metadata: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn cache() -> serde_json::Value {
        json!({"warm":true,"caching_observed":true,"hit_ratio":0.91,"expires_at":3600,
            "misses":2,"last_miss_cause":{"causes":["tools_changed","system_prompt_changed"]}})
    }

    #[test]
    fn renders_token_ratio_expiry_and_the_hosts_miss_diagnosis() {
        let now = Utc.timestamp_opt(60, 0).unwrap();
        let data = format_cache(&serde_json::from_value(cache()).unwrap(), now);
        assert_eq!(data.primary, "91%");
        assert_eq!(
            data.secondary,
            "· warm · 59m · 2 misses · last: tools_changed, system_prompt_changed"
        );
        let expired = format_cache(
            &serde_json::from_value(cache()).unwrap(),
            now + chrono::Duration::seconds(3540),
        );
        assert!(expired.secondary.starts_with("· cold · 2 misses"));
        let mut single = cache();
        single["misses"] = json!(1);
        single["last_miss_cause"] = json!(null);
        let data = format_cache(&serde_json::from_value(single).unwrap(), now);
        assert_eq!(data.secondary, "· warm · 59m · 1 miss");
    }

    #[test]
    fn distinguishes_unobserved_caching_null_ratio_and_a_cold_cache() {
        let mut value = cache();
        value["warm"] = json!(false);
        value["hit_ratio"] = json!(null);
        value["expires_at"] = json!(null);
        value["misses"] = json!(0);
        for (observed, expected) in [(true, "· cold"), (false, "· unobserved")] {
            value["caching_observed"] = json!(observed);
            let data = format_cache(&serde_json::from_value(value.clone()).unwrap(), Utc::now());
            assert_eq!(data.primary, "?");
            assert_eq!(data.secondary, expected);
        }
    }

    #[test]
    fn absent_cache_is_hidden_and_malformed_fields_are_errors() {
        let mut value = super::super::test_payload();
        let absent = serde_json::from_value(value.clone()).unwrap();
        assert!(PromptCacheSegment.collect(&absent).is_none());
        for invalid in [
            json!({}),
            json!({"warm":"yes"}),
            json!({"warm":true,"caching_observed":true,"misses":-1}),
        ] {
            value["prompt_cache"] = invalid;
            assert!(serde_json::from_value::<InputData>(value.clone()).is_err());
        }
        for (key, invalid) in [
            ("hit_ratio", json!("0.9")),
            ("expires_at", json!("soon")),
            ("last_miss_cause", json!({"causes":"tools_changed"})),
        ] {
            value["prompt_cache"] = cache();
            value["prompt_cache"][key] = invalid;
            assert!(
                serde_json::from_value::<InputData>(value.clone()).is_err(),
                "{key}"
            );
        }
    }
}
