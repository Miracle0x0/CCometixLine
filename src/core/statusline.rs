use crate::config::{AnsiColor, Config, SegmentConfig, StyleMode};
use crate::core::format::without_hyperlinks;
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
        self.generate_with_width(segments, None)
    }

    /// Wrap at segment boundaries. An oversized segment stays intact on its own row.
    /// `None` means no wrapping, for callers without a terminal width.
    pub fn generate_with_width(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: Option<usize>,
    ) -> String {
        self.render_lines(segments, max_width).join("\n")
    }

    fn render_lines(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: Option<usize>,
    ) -> Vec<String> {
        let enabled: Vec<_> = segments
            .into_iter()
            .filter(|(config, _)| config.enabled)
            .collect();
        let output = self.render_segments(&enabled);
        let separator_width = visible_width(&self.config.style.separator);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut row_width = 0;
        for (index, segment) in output.iter().enumerate() {
            let width = visible_width(segment);
            let next_width = row_width + if index > start { separator_width } else { 0 } + width;
            if index > start && max_width.is_some_and(|limit| next_width > limit) {
                lines.push(self.join_segments(&output[start..index], &enabled[start..index]));
                start = index;
                row_width = width;
            } else {
                row_width = next_width;
            }
        }
        if start < output.len() {
            lines.push(self.join_segments(&output[start..], &enabled[start..]));
        }
        lines
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

        if let Ok(text) = without_hyperlinks(&full_output).as_bytes().into_text() {
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
        let lines = self.render_lines(segments, Some(usize::from(max_width)));

        // Convert string lines to ratatui Text
        let mut tui_lines = Vec::new();
        for line in lines {
            if let Ok(text) = without_hyperlinks(&line).as_bytes().into_text() {
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
            format!("{}{}\x1b[0m", bg_code, segment_content)
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
            SegmentId::SessionName => IdentitySegment::SessionName.collect(input),
            SegmentId::Worktree => IdentitySegment::Worktree.collect(input),
            SegmentId::PullRequest => IdentitySegment::PullRequest.collect(input),
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
    use crate::config::SegmentId;
    use crate::ui::themes::ThemePresets;

    fn wrapping_segments(config: &Config) -> Vec<(SegmentConfig, SegmentData)> {
        [
            (SegmentId::Model, "中e\u{301}👩‍💻"),
            (SegmentId::Effort, "high"),
            (SegmentId::PullRequest, "\x1b]8;;https://example.com/a/very/long/repository/pull/123\x1b\\PR #123\x1b]8;;\x1b\\"),
        ].into_iter().map(|(id, primary)| {
            let mut segment = config.segments.iter().find(|s| s.id == id).unwrap().clone();
            segment.enabled = true;
            (segment, SegmentData { primary: primary.into(), secondary: String::new(), metadata: Default::default() })
        }).collect()
    }

    #[test]
    fn wraps_at_exact_terminal_cell_boundaries_in_every_theme() {
        for theme in ThemePresets::BUILTIN_THEMES {
            let config = ThemePresets::builtin(theme).unwrap();
            let segments = wrapping_segments(&config);
            let renderer = StatusLineGenerator::new(config);
            let first_two = renderer.generate(segments[..2].to_vec());
            let width = visible_width(&first_two);
            let third = renderer.generate(segments[2..].to_vec());
            let wrapped = renderer.generate_with_width(segments.clone(), Some(width));
            assert_eq!(wrapped, format!("{first_two}\n{third}"), "{theme}");
            let narrower = renderer.generate_with_width(segments[..2].to_vec(), Some(width - 1));
            assert_eq!(narrower.lines().count(), 2, "{theme}: {narrower}");
            assert_eq!(
                renderer.generate_with_width(segments.clone(), Some(1000)),
                renderer.generate(segments.clone())
            );
            for terminator in ["\x1b\\", "\x07"] {
                let mut preview_segments = segments.clone();
                preview_segments[2].1.primary =
                    preview_segments[2].1.primary.replace("\x1b\\", terminator);
                let preview = renderer.generate_for_tui_preview(preview_segments, width as u16);
                let preview_widths: Vec<_> =
                    preview.lines.iter().map(|line| line.width()).collect();
                assert_eq!(
                    preview_widths,
                    wrapped.lines().map(visible_width).collect::<Vec<_>>(),
                    "{theme}"
                );
                assert!(preview
                    .lines
                    .last()
                    .unwrap()
                    .to_string()
                    .contains("PR #123"));
            }
        }
    }

    #[test]
    fn oversized_segments_stay_intact_and_disabled_segments_leave_no_empty_rows() {
        let config = ThemePresets::get_powerline_dark();
        let mut segments = wrapping_segments(&config);
        let renderer = StatusLineGenerator::new(config);
        segments[1].0.enabled = false;
        let wrapped = renderer.generate_with_width(segments.clone(), Some(1));
        assert_eq!(wrapped.lines().count(), 2);
        assert!(wrapped.contains("中e\u{301}👩‍💻"));
        assert!(wrapped.contains("\x1b\\PR #123\x1b]8;;\x1b\\"));
        assert!(!wrapped.contains('\u{e0b0}'));
        assert!(wrapped.lines().all(|line| line.ends_with("\x1b[0m")));
        for (config, _) in &mut segments {
            config.enabled = false;
        }
        assert!(renderer.generate_with_width(segments, Some(1)).is_empty());
    }

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
