use super::{Segment, SegmentData};
use crate::config::{InputData, ModelConfig, SegmentId};
use std::collections::HashMap;

pub struct ModelSegment<'a> {
    models: &'a ModelConfig,
}

impl<'a> ModelSegment<'a> {
    pub fn new(models: &'a ModelConfig) -> Self {
        Self { models }
    }
}

impl Segment for ModelSegment<'_> {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let mut metadata = HashMap::new();
        metadata.insert("model_id".to_string(), input.model.id.clone());
        metadata.insert("display_name".to_string(), input.model.display_name.clone());

        Some(SegmentData {
            primary: self.format_model_name(&input.model.id, &input.model.display_name),
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Model
    }
}

impl ModelSegment<'_> {
    fn format_model_name(&self, id: &str, display_name: &str) -> String {
        if let Some(config_name) = self.models.get_display_name(id) {
            // Model recognized by config, display_name already includes modifier suffix
            config_name
        } else {
            // Fallback: prefer upstream display_name, fall back to model_id if empty
            let base = if display_name.is_empty() {
                id.to_string()
            } else {
                display_name.to_string()
            };
            // Still apply context modifier suffix (e.g., " 1M") if present
            match self.models.get_display_suffix(id) {
                Some(suffix) => format!("{}{}", base, suffix),
                None => base,
            }
        }
    }
}
