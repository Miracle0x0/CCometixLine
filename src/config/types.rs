use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Main config structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub style: StyleConfig,
    pub segments: Vec<SegmentConfig>,
    pub theme: String,
}

// Default implementation moved to ui/themes/presets.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    pub mode: StyleMode,
    pub separator: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleMode {
    Plain,
    NerdFont,
    Powerline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub id: SegmentId,
    pub enabled: bool,
    pub icon: IconConfig,
    pub colors: ColorConfig,
    pub styles: TextStyleConfig,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconConfig {
    pub plain: String,
    pub nerd_font: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub icon: Option<AnsiColor>,
    pub text: Option<AnsiColor>,
    pub background: Option<AnsiColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextStyleConfig {
    pub text_bold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnsiColor {
    Color16 { c16: u8 },
    Color256 { c256: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentId {
    Model,
    Effort,
    Agents,
    Directory,
    Git,
    ContextWindow,
    Usage,
    Cost,
    Session,
    OutputStyle,
    Update,
}

// Legacy compatibility structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SegmentsConfig {
    pub directory: bool,
    pub git: bool,
    pub model: bool,
    // pub usage: bool,
}

// Data structures compatible with existing main.rs
#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct Workspace {
    pub current_dir: String,
}

#[derive(Deserialize)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    pub total_api_duration_ms: Option<u64>,
    pub total_lines_added: Option<u32>,
    pub total_lines_removed: Option<u32>,
}

#[derive(Deserialize)]
pub struct OutputStyle {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Effort {
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitWindow {
    pub used_percentage: f64,
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub resets_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendPeriod {
    Daily,
    Weekly,
    Monthly,
}

/// A Claude apps gateway spend limit. The percentage and reset time arrive with every response;
/// the dollar amounts and period come from a separate, less frequent request and may be absent.
#[derive(Debug, Deserialize)]
pub struct SpendLimit {
    pub used_percentage: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub resets_at: chrono::DateTime<chrono::Utc>,
    pub used_usd: Option<f64>,
    pub limit_usd: Option<f64>,
    pub period: Option<SpendPeriod>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimits {
    pub five_hour: Option<RateLimitWindow>,
    pub seven_day: Option<RateLimitWindow>,
    pub spend_limit: Option<SpendLimit>,
}

/// Token counts from the most recent API call.
#[derive(Debug, Deserialize)]
pub struct CurrentUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl CurrentUsage {
    /// Tokens occupying the context window, by the same input-only formula as `used_percentage`.
    pub fn context_tokens(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}

/// Claude Code's live context window. `used_percentage` may be null early in a session, and
/// `current_usage` is null before the first API call and again right after `/compact`.
#[derive(Debug, Deserialize)]
pub struct ContextWindow {
    pub context_window_size: u64,
    pub used_percentage: Option<f64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Deserialize)]
pub struct InputData {
    pub session_id: String,
    pub model: Model,
    pub effort: Option<Effort>,
    pub workspace: Workspace,
    pub context_window: ContextWindow,
    pub cost: Option<Cost>,
    pub output_style: Option<OutputStyle>,
    pub rate_limits: Option<RateLimits>,
}

impl Config {
    pub fn agents_enabled(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.id == SegmentId::Agents && segment.enabled)
    }

    /// Check if current config matches the specified theme preset
    pub fn matches_theme(&self, theme_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let theme_preset = crate::ui::themes::ThemePresets::get_theme(theme_name)?;

        // Compare style config
        if self.style.mode != theme_preset.style.mode
            || self.style.separator != theme_preset.style.separator
        {
            return Ok(false);
        }

        // Compare segments count, order and each segment config
        Ok(self.segments.len() == theme_preset.segments.len()
            && self
                .segments
                .iter()
                .zip(theme_preset.segments.iter())
                .all(|(current, preset)| self.segment_matches(current, preset)))
    }

    /// Check if current config has been modified from the selected theme
    pub fn is_modified_from_theme(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(!self.matches_theme(&self.theme)?)
    }

    /// Compare two segment configs for equality
    fn segment_matches(&self, current: &SegmentConfig, preset: &SegmentConfig) -> bool {
        current.id == preset.id
            && current.enabled == preset.enabled
            && current.icon.plain == preset.icon.plain
            && current.icon.nerd_font == preset.icon.nerd_font
            && self.color_matches(&current.colors.icon, &preset.colors.icon)
            && self.color_matches(&current.colors.text, &preset.colors.text)
            && self.color_matches(&current.colors.background, &preset.colors.background)
            && current.styles.text_bold == preset.styles.text_bold
            && current.options == preset.options
    }

    /// Compare two optional colors for equality
    fn color_matches(&self, current: &Option<AnsiColor>, preset: &Option<AnsiColor>) -> bool {
        match (current, preset) {
            (None, None) => true,
            (Some(c1), Some(c2)) => c1 == c2,
            _ => false,
        }
    }
}

impl PartialEq for AnsiColor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AnsiColor::Color16 { c16: a }, AnsiColor::Color16 { c16: b }) => a == b,
            (AnsiColor::Color256 { c256: a }, AnsiColor::Color256 { c256: b }) => a == b,
            (
                AnsiColor::Rgb {
                    r: r1,
                    g: g1,
                    b: b1,
                },
                AnsiColor::Rgb {
                    r: r2,
                    g: g2,
                    b: b2,
                },
            ) => r1 == r2 && g1 == g2 && b1 == b2,
            _ => false,
        }
    }
}
