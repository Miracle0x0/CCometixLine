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
    ) -> Result<Option<AgentSummary>> {
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

    pub fn render(
        activity: Option<&Activity>,
        now: u64,
        max_agents: usize,
    ) -> Option<AgentSummary> {
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
        Some(AgentSummary {
            responded: activity.agents.len() - active.len(),
            active: active
                .into_iter()
                .map(|agent| ActiveAgent {
                    name: agent
                        .agent_type
                        .chars()
                        .filter(|c| !c.is_control())
                        .collect(),
                    elapsed: now.saturating_sub(agent.started_at),
                })
                .collect(),
            max_agents,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ActiveAgent {
    pub name: String,
    pub elapsed: u64,
}

#[derive(Debug, Clone)]
pub struct AgentSummary {
    pub active: Vec<ActiveAgent>,
    pub responded: usize,
    pub max_agents: usize,
}

impl AgentSummary {
    pub fn text(&self, count: usize) -> String {
        let count = count.min(self.max_agents).min(self.active.len());
        let mut text = format!("Agents: {} active", self.active.len());
        if self.responded > 0 {
            text.push_str(&format!(" · {} responded", self.responded));
        }
        for agent in self.active.iter().take(count) {
            let elapsed = agent.elapsed;
            let duration = if elapsed < 60 {
                format!("{elapsed}s")
            } else {
                format!("{}m{:02}s", elapsed / 60, elapsed % 60)
            };
            text.push_str(&format!(" · {} {duration}", agent.name));
        }
        let hidden = self.active.len() - count;
        // With no names, the active count already summarizes every agent.
        if hidden > 0 && count > 0 {
            text.push_str(&format!(" · +{hidden} more"));
        }
        text
    }
}
