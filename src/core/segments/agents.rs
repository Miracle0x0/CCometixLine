use super::SegmentData;
use crate::agents::{
    self,
    store::{Activity, ActivityStore},
    Result,
};
use crate::config::InputData;
use std::collections::HashMap;

pub struct AgentsSegment;

impl AgentsSegment {
    pub fn collect(input: &InputData) -> Result<Option<SegmentData>> {
        let Some(root) = agents::integration::active_state_dir()? else {
            return Ok(None);
        };
        let store = ActivityStore::new(root);
        let activity = store.read(&input.session_id)?;
        Ok(Self::render(activity.as_ref(), agents::store::now()?))
    }

    pub fn render(activity: Option<&Activity>, now: u64) -> Option<SegmentData> {
        let activity = activity?;
        if activity.ended {
            return None;
        }
        let active: Vec<_> = activity
            .agents
            .values()
            .filter(|agent| agent.responded_at.is_none())
            .collect();
        if active.is_empty() {
            return None;
        }
        let responded = activity.agents.len() - active.len();
        let mut text = format!("Agents: {} active", active.len());
        if responded > 0 {
            text.push_str(&format!(" · {responded} responded"));
        }
        for agent in active {
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
