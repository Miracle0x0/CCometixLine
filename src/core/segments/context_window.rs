use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

/// Context usage exactly as Claude Code reports it in `context_window`; null values are shown
/// as unknown rather than recovered from the transcript or replaced with zero.
#[derive(Default)]
pub struct ContextWindowSegment;

impl ContextWindowSegment {
    pub fn new() -> Self {
        Self
    }
}

/// Thousands use a `k` suffix with at most one decimal, such as `12.4k`.
pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1000 {
        let k_value = tokens as f64 / 1000.0;
        if k_value.fract() == 0.0 {
            format!("{}k", tokens / 1000)
        } else {
            format!("{:.1}k", k_value)
        }
    } else {
        tokens.to_string()
    }
}

fn format_percentage(percentage: f64) -> String {
    if percentage.fract() == 0.0 {
        format!("{:.0}%", percentage)
    } else {
        format!("{:.1}%", percentage)
    }
}

impl Segment for ContextWindowSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let window = &input.context_window;
        let tokens = window
            .current_usage
            .as_ref()
            .map(|usage| usage.context_tokens());
        let unknown = || "-".to_string();

        let mut metadata = HashMap::new();
        metadata.insert(
            "percentage".to_string(),
            window
                .used_percentage
                .map_or_else(unknown, |percentage| percentage.to_string()),
        );
        metadata.insert(
            "tokens".to_string(),
            tokens.map_or_else(unknown, |tokens| tokens.to_string()),
        );
        metadata.insert("limit".to_string(), window.context_window_size.to_string());

        Some(SegmentData {
            primary: format!(
                "{} · {} tokens",
                window
                    .used_percentage
                    .map_or_else(unknown, format_percentage),
                tokens.map_or_else(unknown, format_tokens)
            ),
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::ContextWindow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn input(context_window: Value) -> InputData {
        let mut payload = crate::core::segments::test_payload();
        payload["context_window"] = context_window;
        serde_json::from_value(payload).unwrap()
    }

    #[test]
    fn uses_the_hosts_percentage_and_input_only_tokens_for_a_1m_window() {
        // 200K input across fresh and cached tokens plus 10K output in a 1M window: the host
        // reports 20%, and output tokens are not counted toward the context.
        let data = ContextWindowSegment::new()
            .collect(&input(json!({
                "total_input_tokens": 200_000,
                "total_output_tokens": 10_000,
                "context_window_size": 1_000_000,
                "used_percentage": 20,
                "remaining_percentage": 80,
                "current_usage": {
                    "input_tokens": 150_000,
                    "output_tokens": 10_000,
                    "cache_creation_input_tokens": 30_000,
                    "cache_read_input_tokens": 20_000
                }
            })))
            .unwrap();
        assert_eq!(data.primary, "20% · 200k tokens");
        assert_eq!(data.metadata["limit"], "1000000");
        assert_eq!(data.metadata["tokens"], "200000");

        let fractional = ContextWindowSegment::new()
            .collect(&input(json!({
                "context_window_size": 200_000,
                "used_percentage": 7.75,
                "current_usage": {
                    "input_tokens": 8_500,
                    "output_tokens": 1_200,
                    "cache_creation_input_tokens": 5_000,
                    "cache_read_input_tokens": 2_000
                }
            })))
            .unwrap();
        assert_eq!(fractional.primary, "7.8% · 15.5k tokens");
    }

    #[test]
    fn null_values_before_the_first_response_and_after_compact_are_unknown() {
        // Session start: nothing reported yet.
        let start = ContextWindowSegment::new()
            .collect(&input(json!({
                "total_input_tokens": 0,
                "total_output_tokens": 0,
                "context_window_size": 1_000_000,
                "used_percentage": null,
                "remaining_percentage": null,
                "current_usage": null
            })))
            .unwrap();
        assert_eq!(start.primary, "- · - tokens");
        assert_eq!(start.metadata["percentage"], "-");
        assert_eq!(start.metadata["tokens"], "-");

        // Right after /compact the breakdown is null until the next API call; whatever the host
        // still reports is shown as is, and nothing is recovered from older data.
        let compacted = ContextWindowSegment::new()
            .collect(&input(json!({
                "context_window_size": 200_000,
                "used_percentage": 3,
                "current_usage": null
            })))
            .unwrap();
        assert_eq!(compacted.primary, "3% · - tokens");
    }

    #[test]
    fn missing_or_malformed_context_window_is_a_deserialization_error() {
        let mut base = crate::core::segments::test_payload();
        base.as_object_mut().unwrap().remove("context_window");
        assert!(serde_json::from_value::<InputData>(base.clone()).is_err());
        for window in [
            json!({}),
            json!({"context_window_size": "1M"}),
            json!({"context_window_size": 200_000, "used_percentage": "8"}),
            json!({"context_window_size": 200_000, "current_usage": {"input_tokens": 1}}),
        ] {
            let mut value = base.clone();
            value["context_window"] = window;
            assert!(serde_json::from_value::<InputData>(value).is_err());
        }
    }
}
