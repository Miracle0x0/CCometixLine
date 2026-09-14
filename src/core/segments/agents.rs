use super::SegmentData;
use crate::agents::{
    self,
    store::{Activity, ActivityStore},
    Result,
};
use crate::config::InputData;
use serde_json::Value;
use std::collections::HashMap;

pub struct AgentsSegment;

impl AgentsSegment {
    /// Active agents listed individually before the rest are summarized as `+N more`.
    pub const DEFAULT_MAX_AGENTS: usize = 3;

    pub fn collect(
        input: &InputData,
        options: &HashMap<String, Value>,
    ) -> Result<Option<SegmentData>> {
        // Validate the configuration before reading state, even when hooks are not installed.
        let max_agents = Self::max_agents(options)?;
        let Some(root) = agents::integration::active_state_dir()? else {
            return Ok(None);
        };
        let store = ActivityStore::new(root);
        let activity = store.read(&input.session_id)?;
        Ok(Self::render(
            activity.as_ref(),
            agents::store::now()?,
            max_agents,
        ))
    }

    /// The `max_agents` option: absent means the default; anything but a count is an error.
    pub fn max_agents(options: &HashMap<String, Value>) -> Result<usize> {
        let Some(value) = options.get("max_agents") else {
            return Ok(Self::DEFAULT_MAX_AGENTS);
        };
        value
            .as_u64()
            .and_then(|max_agents| usize::try_from(max_agents).ok())
            .ok_or_else(|| {
                format!("Agents option max_agents must be a non-negative integer, not {value}")
                    .into()
            })
    }

    pub fn render(activity: Option<&Activity>, now: u64, max_agents: usize) -> Option<SegmentData> {
        let activity = activity?;
        if activity.ended {
            return None;
        }
        let mut active: Vec<_> = activity
            .agents
            .values()
            .filter(|agent| agent.responded_at.is_none())
            .collect();
        if active.is_empty() {
            return None;
        }
        // Longest-running first; the stable sort keeps agent ID order for equal start times.
        active.sort_by_key(|agent| agent.started_at);
        let responded = activity.agents.len() - active.len();
        let mut text = format!("Agents: {} active", active.len());
        if responded > 0 {
            text.push_str(&format!(" · {responded} responded"));
        }
        for agent in active.iter().take(max_agents) {
            let elapsed = now.saturating_sub(agent.started_at);
            let duration = if elapsed < 60 {
                format!("{elapsed}s")
            } else {
                format!("{}m{:02}s", elapsed / 60, elapsed % 60)
            };
            // Agent names are untrusted display text, not terminal control sequences.
            let name: String = agent
                .agent_type
                .chars()
                .filter(|c| !c.is_control())
                .collect();
            text.push_str(&format!(" · {name} {duration}"));
        }
        let hidden = active.len().saturating_sub(max_agents);
        // A limit of zero lists nobody, and the count already covers everyone.
        if hidden > 0 && max_agents > 0 {
            text.push_str(&format!(" · +{hidden} more"));
        }
        Some(Self::data(text))
    }

    fn data(primary: String) -> SegmentData {
        SegmentData {
            primary,
            secondary: String::new(),
            metadata: HashMap::new(),
        }
    }
}
