//! Rows for Claude Code's subagent panel, rendered through the `subagentStatusLine` setting.
//!
//! Claude Code sends the visible rows on stdin and replaces each row whose `id` ccline writes
//! back; see https://code.claude.com/docs/en/statusline#subagent-status-lines.

use super::format_elapsed;
use crate::config::{Config, ModelConfig, SegmentId};
use crate::core::format::printable;
use crate::core::segments::context_window::format_tokens;
use crate::core::statusline::{visible_width, StatusLineGenerator};
use crate::ui::themes::ThemePresets;
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

/// Replace only row bodies: the host keeps the tree connectors, indentation, status bullets and
/// selection controls. The status field has a semantic color; the rest uses the Agents text style.
/// Without an enabled Agents segment nothing is written, so the host keeps its default rows.
pub fn rows(input: &PanelInput, config: &Config, now: u64, models: &ModelConfig) -> Vec<PanelRow> {
    let Some(segment) = config
        .segments
        .iter()
        .find(|segment| segment.id == SegmentId::Agents && segment.enabled)
    else {
        return Vec::new();
    };
    let renderer = StatusLineGenerator::new(config.clone());
    let colors = ThemePresets::agent_status_colors(&config.theme);
    let mut status_style = segment.clone();
    input
        .tasks
        .iter()
        .map(|task| {
            let (mark, color) = match task.status.as_str() {
                "running" => (">", &colors.running),
                "completed" => ("+", &colors.completed),
                "failed" => ("!", &colors.failed),
                "killed" => ("x", &colors.killed),
                _ => ("?", &colors.unknown),
            };
            let state = if task.status == "running" {
                format_elapsed(now.saturating_sub(task.start_time / 1000))
            } else {
                printable(&task.status)
            };
            let fields = fit(task, format!("[{mark}] {state}"), models, input.columns);
            status_style.colors.text = Some(color.clone());
            let mut content =
                renderer.render_text(segment, &format!("{} · {} · ", fields[0], fields[1]));
            content.push_str(&renderer.render_text(&status_style, &fields[2]));
            if fields.len() > 3 {
                content.push_str(
                    &renderer.render_text(segment, &format!(" · {}", fields[3..].join(" · "))),
                );
            }
            PanelRow {
                id: task.id.clone(),
                content,
            }
        })
        .collect()
}

/// Drop trailing details (effort, model, tokens), then shorten the label, until the row fits.
fn fit(task: &PanelTask, state: String, models: &ModelConfig, columns: usize) -> Vec<String> {
    let name = task
        .agent_type
        .as_deref()
        .or(task.name.as_deref())
        .unwrap_or(&task.kind);
    let mut fields = vec![printable(name), printable(&task.label), state];
    let details = details(task, models);
    for count in (0..=details.len()).rev() {
        let mut candidate = fields.clone();
        candidate.extend_from_slice(&details[..count]);
        let row = candidate.join(" · ");
        if visible_width(&row) <= columns {
            return candidate;
        }
    }
    let label: Vec<char> = fields[1].chars().collect();
    for length in (0..label.len()).rev() {
        fields[1] = label[..length].iter().chain(['…'].iter()).collect();
        let row = fields.join(" · ");
        if visible_width(&row) <= columns {
            return fields;
        }
    }
    // Nothing shorter is meaningful; the host clips what remains.
    fields
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
