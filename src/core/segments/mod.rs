pub mod agents;
pub mod context_window;
pub mod cost;
pub mod directory;
pub mod effort;
pub mod git;
pub mod model;
pub mod output_style;
pub mod session;
pub mod update;
pub mod usage;

use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

// New Segment trait for data collection only
pub trait Segment {
    fn collect(&self, input: &InputData) -> Option<SegmentData>;
    fn id(&self) -> SegmentId;
}

#[derive(Debug, Clone)]
pub struct SegmentData {
    pub primary: String,
    pub secondary: String,
    pub metadata: HashMap<String, String>,
}

/// Keep variable-length agent activity structured until the entire line is laid out.
#[derive(Debug, Clone)]
pub enum SegmentContent {
    Text(SegmentData),
    Agents(agents::AgentSummary),
}

impl From<SegmentData> for SegmentContent {
    fn from(data: SegmentData) -> Self {
        Self::Text(data)
    }
}

// Re-export all segment types
pub use agents::AgentsSegment;
pub use context_window::ContextWindowSegment;
pub use cost::CostSegment;
pub use directory::DirectorySegment;
pub use effort::EffortSegment;
pub use git::GitSegment;
pub use model::ModelSegment;
pub use output_style::OutputStyleSegment;
pub use session::SessionSegment;
pub use update::UpdateSegment;
pub use usage::UsageSegment;
