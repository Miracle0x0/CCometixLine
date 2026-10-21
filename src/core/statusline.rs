use crate::config::{AnsiColor, Config, SegmentConfig, StyleMode};
use crate::core::segments::SegmentData;
use unicode_width::UnicodeWidthStr;

/// Measure terminal cells, excluding CSI formatting and OSC hyperlinks.
pub(crate) fn visible_width(text: &str) -> usize {
    static ESCAPES: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let escapes = ESCAPES.get_or_init(|| {
        regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")
            .expect("valid ANSI escape expression")
    });
    UnicodeWidthStr::width(escapes.replace_all(text, "").as_ref())
}

pub struct StatusLineGenerator {
    config: Config,
}

impl StatusLineGenerator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate(&self, segments: Vec<(SegmentConfig, SegmentData)>) -> String {
        let enabled: Vec<_> = segments
            .into_iter()
            .filter(|(config, _)| config.enabled)
            .collect();
        let output = self.render_segments(&enabled);
        self.join_segments(&output, &enabled)
    }

    fn render_segments(&self, segments: &[(SegmentConfig, SegmentData)]) -> Vec<String> {
        segments
            .iter()
            .map(|(config, data)| self.render_segment(config, data))
            .collect()
    }

    fn join_segments(
        &self,
        output: &[String],
        segments: &[(SegmentConfig, SegmentData)],
    ) -> String {
        // Handle Powerline arrow separators with color transition
        if self.config.style.separator == "\u{e0b0}" {
            self.join_with_powerline_arrows(output, segments)
        } else {
            // For all other separators, use white color and simple join
            self.join_with_white_separators(output)
        }
    }

    /// Generate statusline for TUI preview with proper width calculation
    /// This method handles ANSI escape sequences properly for ratatui rendering
    pub fn generate_for_tui(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
    ) -> ratatui::text::Line<'static> {
        use ansi_to_tui::IntoText;
        use ratatui::text::{Line, Span};

        // Use the same generate method and convert to TUI
        let full_output = self.generate(segments);

        if let Ok(text) = full_output.into_text() {
            if let Some(line) = text.lines.into_iter().next() {
                return line;
            }
        }

        // Fallback to raw text
        Line::from(vec![Span::raw(full_output)])
    }

    /// Generate TUI-optimized text with intelligent wrapping by segment for preview
    pub fn generate_for_tui_preview(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: u16,
    ) -> ratatui::text::Text<'_> {
        use ansi_to_tui::IntoText;
        use ratatui::text::{Line, Span, Text};
        let enabled: Vec<_> = segments
            .into_iter()
            .filter(|(config, _)| config.enabled)
            .collect();
        let rendered_segments = self.render_segments(&enabled);
        let segment_configs: Vec<_> = enabled.iter().map(|(config, _)| config).collect();

        // Pre-calculate separators between segments
        let mut separators = Vec::new();
        for i in 0..rendered_segments.len().saturating_sub(1) {
            let separator = if self.config.style.separator == "\u{e0b0}" {
                // Powerline arrows with color transition
                let prev_bg = segment_configs
                    .get(i)
                    .and_then(|config| config.colors.background.as_ref());
                let curr_bg = segment_configs
                    .get(i + 1)
                    .and_then(|config| config.colors.background.as_ref());
                self.create_powerline_arrow(prev_bg, curr_bg)
            } else {
                // Regular separators with white color
                format!("\x1b[37m{}\x1b[0m", self.config.style.separator)
            };
            separators.push(separator);
        }

        // Intelligent line wrapping by segment
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0usize;
        let max_w = max_width as usize;

        for i in 0..rendered_segments.len() {
            let segment = &rendered_segments[i];
            let segment_width = visible_width(segment);

            // Check if adding this segment would exceed max_width
            if current_width > 0 && current_width + segment_width > max_w {
                // Current line would overflow, start a new line
                lines.push(current_line.clone());
                current_line.clear();
                current_width = 0;
            }

            // Add the segment to current line
            current_line.push_str(segment);
            current_width += segment_width;

            // Handle separator if not the last segment
            if i < separators.len() {
                let separator = &separators[i];
                let separator_width = visible_width(separator);

                // Check if next segment exists
                if i + 1 < rendered_segments.len() {
                    let next_segment = &rendered_segments[i + 1];
                    let next_width = visible_width(next_segment);

                    // Check if separator AND next segment both fit
                    if current_width + separator_width + next_width <= max_w {
                        // Both fit, add separator and continue on same line
                        current_line.push_str(separator);
                        current_width += separator_width;
                    } else {
                        // Separator and/or next segment don't fit
                        // Don't add separator, just break line
                        lines.push(current_line.clone());
                        current_line.clear();
                        current_width = 0;
                    }
                }
            }
        }

        // Add the last line if it's not empty
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Convert string lines to ratatui Text
        let mut tui_lines = Vec::new();
        for line in lines {
            if let Ok(text) = line.into_text() {
                for tui_line in text.lines {
                    tui_lines.push(tui_line);
                }
            } else {
                tui_lines.push(Line::from(vec![Span::raw(line)]));
            }
        }

        // Ensure we have at least one line
        if tui_lines.is_empty() {
            tui_lines.push(Line::default());
        }

        Text::from(tui_lines)
    }

    fn render_segment(&self, config: &SegmentConfig, data: &SegmentData) -> String {
        let icon = if let Some(dynamic_icon) = data.metadata.get("dynamic_icon") {
            dynamic_icon.clone()
        } else {
            self.get_icon(config)
        };

        // Apply background color to the entire segment if set
        if let Some(bg_color) = &config.colors.background {
            let bg_code = self.apply_background_color(bg_color);

            // Build the entire segment content first
            let icon_colored = if let Some(icon_color) = &config.colors.icon {
                self.apply_color(&icon, Some(icon_color))
                    .replace("\x1b[0m", "")
            } else {
                icon.clone()
            };

            let text_styled = self
                .render_text(config, &data.primary)
                .replace("\x1b[0m", "");

            let mut segment_content = format!(" {} {} ", icon_colored, text_styled);

            if !data.secondary.is_empty() {
                let secondary_styled = self
                    .render_text(config, &data.secondary)
                    .replace("\x1b[0m", "");
                segment_content.push_str(&format!("{} ", secondary_styled));
            }

            // Apply background to the entire content and reset at the end
            format!("{}{}\x1b[49m", bg_code, segment_content)
        } else {
            // No background color, use original logic
            let icon_colored = self.apply_color(&icon, config.colors.icon.as_ref());
            let text_styled = self.render_text(config, &data.primary);

            let mut segment = format!("{} {}", icon_colored, text_styled);

            if !data.secondary.is_empty() {
                segment.push_str(&format!(" {}", self.render_text(config, &data.secondary)));
            }

            segment
        }
    }

    fn get_icon(&self, config: &SegmentConfig) -> String {
        match self.config.style.mode {
            StyleMode::Plain => config.icon.plain.clone(),
            StyleMode::NerdFont => config.icon.nerd_font.clone(),
            StyleMode::Powerline => config.icon.nerd_font.clone(), // Future: use Powerline icons
        }
    }

    fn apply_color(&self, text: &str, color: Option<&AnsiColor>) -> String {
        match color {
            Some(AnsiColor::Color16 { c16 }) => {
                let code = if *c16 < 8 { 30 + c16 } else { 90 + (c16 - 8) };
                format!("\x1b[{}m{}\x1b[0m", code, text)
            }
            Some(AnsiColor::Color256 { c256 }) => {
                format!("\x1b[38;5;{}m{}\x1b[0m", c256, text)
            }
            Some(AnsiColor::Rgb { r, g, b }) => {
                format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
            }
            None => text.to_string(),
        }
    }

    /// Apply the segment's text styling without an icon, background, padding or separator.
    pub(crate) fn render_text(&self, config: &SegmentConfig, text: &str) -> String {
        let mut codes = Vec::new();

        // Add style codes
        if config.styles.text_bold {
            codes.push("1".to_string()); // Bold: \x1b[1m
        }

        // Add color codes
        match config.colors.text.as_ref() {
            Some(AnsiColor::Color16 { c16 }) => {
                let color_code = if *c16 < 8 { 30 + c16 } else { 90 + (c16 - 8) };
                codes.push(color_code.to_string());
            }
            Some(AnsiColor::Color256 { c256 }) => {
                codes.push("38".to_string());
                codes.push("5".to_string());
                codes.push(c256.to_string());
            }
            Some(AnsiColor::Rgb { r, g, b }) => {
                codes.push("38".to_string());
                codes.push("2".to_string());
                codes.push(r.to_string());
                codes.push(g.to_string());
                codes.push(b.to_string());
            }
            None => {}
        }

        if codes.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
        }
    }

    fn apply_background_color(&self, color: &AnsiColor) -> String {
        match color {
            AnsiColor::Color16 { c16 } => {
                let code = if *c16 < 8 { 40 + c16 } else { 100 + (c16 - 8) };
                format!("\x1b[{}m", code)
            }
            AnsiColor::Color256 { c256 } => {
                format!("\x1b[48;5;{}m", c256)
            }
            AnsiColor::Rgb { r, g, b } => {
                format!("\x1b[48;2;{};{};{}m", r, g, b)
            }
        }
    }

    /// Join segments with white separators (non-Powerline)
    fn join_with_white_separators(&self, rendered_segments: &[String]) -> String {
        if rendered_segments.is_empty() {
            return String::new();
        }

        // Use white color for separator
        let white_separator = format!("\x1b[37m{}\x1b[0m", self.config.style.separator);
        rendered_segments.join(&white_separator)
    }

    /// Join segments with Powerline arrow separators with proper color transitions
    fn join_with_powerline_arrows(
        &self,
        rendered_segments: &[String],
        segment_configs: &[(SegmentConfig, SegmentData)],
    ) -> String {
        if rendered_segments.is_empty() {
            return String::new();
        }

        if rendered_segments.len() == 1 {
            return rendered_segments[0].clone();
        }

        let mut result = rendered_segments[0].clone();

        for (i, _) in rendered_segments.iter().enumerate().skip(1) {
            let prev_bg = segment_configs
                .get(i - 1)
                .and_then(|(config, _)| config.colors.background.as_ref());
            let curr_bg = segment_configs
                .get(i)
                .and_then(|(config, _)| config.colors.background.as_ref());

            // Create Powerline arrow with color transition
            let arrow = self.create_powerline_arrow(prev_bg, curr_bg);

            result.push_str(&arrow);
            result.push_str(&rendered_segments[i]);
        }

        // Reset colors at the end
        result.push_str("\x1b[0m");
        result
    }

    /// Create a Powerline arrow with proper color transition
    fn create_powerline_arrow(
        &self,
        prev_bg: Option<&AnsiColor>,
        curr_bg: Option<&AnsiColor>,
    ) -> String {
        let arrow_char = "\u{e0b0}";

        match (prev_bg, curr_bg) {
            (Some(prev), Some(curr)) => {
                // Arrow foreground = previous segment's background
                // Arrow background = current segment's background
                let fg_code = self.color_to_foreground_code(prev);
                let bg_code = self.apply_background_color(curr);
                format!("{}{}{}\x1b[0m", bg_code, fg_code, arrow_char)
            }
            (Some(prev), None) => {
                // Previous segment has background, current doesn't
                let fg_code = self.color_to_foreground_code(prev);
                format!("{}{}\x1b[0m", fg_code, arrow_char)
            }
            (None, Some(curr)) => {
                // Current segment has background, previous doesn't
                let bg_code = self.apply_background_color(curr);
                format!("{}{}\x1b[0m", bg_code, arrow_char)
            }
            (None, None) => {
                // Neither segment has background color
                arrow_char.to_string()
            }
        }
    }

    /// Convert AnsiColor to foreground color code
    fn color_to_foreground_code(&self, color: &AnsiColor) -> String {
        match color {
            AnsiColor::Color16 { c16 } => {
                let code = if *c16 < 8 { 30 + c16 } else { 90 + (c16 - 8) };
                format!("\x1b[{}m", code)
            }
            AnsiColor::Color256 { c256 } => {
                format!("\x1b[38;5;{}m", c256)
            }
            AnsiColor::Rgb { r, g, b } => {
                format!("\x1b[38;2;{};{};{}m", r, g, b)
            }
        }
    }
}

pub fn collect_all_segments(
    config: &Config,
    models: &crate::config::ModelConfig,
    input: &crate::config::InputData,
) -> Result<Vec<(SegmentConfig, SegmentData)>, Box<dyn std::error::Error>> {
    use crate::config::SegmentId;
    use crate::core::segments::*;

    let mut results = Vec::new();

    for segment_config in &config.segments {
        // Skip disabled segments to avoid unnecessary API requests
        if !segment_config.enabled {
            continue;
        }

        let segment_data = match segment_config.id {
            SegmentId::Agents => crate::agents::summary::collect(
                &crate::config::paths::claude_dir()?,
                &input.session_id,
                segment_config,
                crate::agents::now()?,
            )?,
            SegmentId::Model => ModelSegment::new(models).collect(input),
            SegmentId::Effort => EffortSegment::new().collect(input),
            SegmentId::Directory => DirectorySegment::new().collect(input),
            SegmentId::Git => {
                let show_sha = segment_config
                    .options
                    .get("show_sha")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                GitSegment::new().with_sha(show_sha).collect(input)
            }
            SegmentId::ContextWindow => ContextWindowSegment::new().collect(input),
            SegmentId::PromptCache => PromptCacheSegment.collect(input),
            SegmentId::Usage => UsageSegment::new().collect(input),
            SegmentId::Cost => CostSegment::new().collect(input),
            SegmentId::Session => SessionSegment::new().collect(input),
            SegmentId::OutputStyle => OutputStyleSegment::new().collect(input),
            SegmentId::Update => {
                UpdateSegment::new(crate::config::paths::update_state_path()?).collect(input)
            }
        };

        if let Some(data) = segment_data {
            results.push((segment_config.clone(), data));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_cells_exclude_formatting_and_count_unicode_width() {
        assert_eq!(visible_width("\x1b[38;2;1;2;3m中e\u{301}👩‍💻\x1b[0m"), 5);
        assert_eq!(
            visible_width("\x1b]8;;https://example.com\x1b\\中\x1b]8;;\x1b\\"),
            2
        );
        assert_eq!(
            visible_width("\x1b]8;;https://example.com\x07中\x1b]8;;\x07"),
            2
        );
    }
}
