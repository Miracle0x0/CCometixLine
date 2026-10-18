//! Rows for Claude Code's subagent panel, rendered through the `subagentStatusLine` setting.
//!
//! Claude Code sends the visible rows on stdin and replaces each row whose `id` ccline writes
//! back; see https://code.claude.com/docs/en/statusline#subagent-status-lines.

use super::{format_elapsed, printable};
use crate::config::{Config, ModelConfig, SegmentId};
use crate::core::segments::context_window::format_tokens;
use crate::core::statusline::{visible_width, StatusLineGenerator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PanelInput {
    pub session_id: String,
    /// Usable row width in terminal cells.
    pub columns: usize,
    pub tasks: Vec<PanelTask>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTask {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub agent_type: Option<String>,
    pub status: String,
    pub label: String,
    /// Milliseconds since the Unix epoch.
    pub start_time: u64,
    pub model: Option<String>,
    pub effort: Option<Effort>,
    pub context_window_size: Option<u64>,
    pub token_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Effort {
    Level(String),
    Budget(u64),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PanelRow {
    pub id: String,
    pub content: String,
}

/// One row per task, styled with the Agents segment's text color and bold setting. Without an enabled
/// Agents segment nothing is written, so Claude Code keeps its default rows.
pub fn rows(input: &PanelInput, config: &Config, now: u64, models: &ModelConfig) -> Vec<PanelRow> {
    let Some(segment) = config
        .segments
        .iter()
        .find(|segment| segment.id == SegmentId::Agents && segment.enabled)
    else {
        return Vec::new();
    };
    let renderer = StatusLineGenerator::new(config.clone());
    let render = |fields: &[String]| renderer.render_text(segment, &fields.join(" · "));
    input
        .tasks
        .iter()
        .map(|task| PanelRow {
            id: task.id.clone(),
            content: fit(task, now, models, input.columns, &render),
        })
        .collect()
}

/// Drop trailing details (effort, model, tokens), then shorten the label, until the row fits.
fn fit(
    task: &PanelTask,
    now: u64,
    models: &ModelConfig,
    columns: usize,
    render: &dyn Fn(&[String]) -> String,
) -> String {
    let name = task
        .agent_type
        .as_deref()
        .or(task.name.as_deref())
        .unwrap_or(&task.kind);
    let state = if task.status == "running" {
        format_elapsed(now.saturating_sub(task.start_time / 1000))
    } else {
        printable(&task.status)
    };
    let mut fields = vec![printable(name), printable(&task.label), state];
    let details = details(task, models);
    for count in (0..=details.len()).rev() {
        let mut candidate = fields.clone();
        candidate.extend_from_slice(&details[..count]);
        let row = render(&candidate);
        if visible_width(&row) <= columns {
            return row;
        }
    }
    let label: Vec<char> = fields[1].chars().collect();
    for length in (0..label.len()).rev() {
        fields[1] = label[..length].iter().chain(['…'].iter()).collect();
        let row = render(&fields);
        if visible_width(&row) <= columns {
            return row;
        }
    }
    // Nothing shorter is meaningful; the host clips what remains.
    render(&fields)
}

fn details(task: &PanelTask, models: &ModelConfig) -> Vec<String> {
    let mut tokens = format_tokens(task.token_count);
    if let Some(size) = task.context_window_size.filter(|size| *size > 0) {
        tokens.push_str(&format!(
            " ({:.0}%)",
            task.token_count as f64 * 100.0 / size as f64
        ));
    }
    let mut details = vec![tokens];
    if let Some(model) = &task.model {
        details.push(printable(
            &models
                .get_display_name(model)
                .unwrap_or_else(|| model.clone()),
        ));
    }
    match &task.effort {
        Some(Effort::Level(level)) => details.push(printable(level)),
        Some(Effort::Budget(budget)) => details.push(format!("{} budget", format_tokens(*budget))),
        None => {}
    }
    details
}
