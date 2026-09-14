use crate::config::{Config, SegmentId, StyleMode};
use crate::ui::components::{
    color_picker::{ColorPickerComponent, NavDirection},
    help::HelpComponent,
    icon_selector::IconSelectorComponent,
    name_input::NameInputComponent,
    preview::PreviewComponent,
    segment_list::{FieldSelection, Panel, SegmentListComponent},
    separator_editor::SeparatorEditorComponent,
    settings::SettingsComponent,
    theme_selector::ThemeSelectorComponent,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

pub struct App {
    config: Config,
    selected_segment: usize,
    selected_panel: Panel,
    selected_field: FieldSelection,
    should_quit: bool,
    color_picker: ColorPickerComponent,
    icon_selector: IconSelectorComponent,
    name_input: NameInputComponent,
    preview: PreviewComponent,
    segment_list: SegmentListComponent,
    separator_editor: SeparatorEditorComponent,
    settings: SettingsComponent,
    theme_selector: ThemeSelectorComponent,
    help: HelpComponent,
    status_message: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let config = Self::with_optional_segments(config);
        let mut app = Self {
            config: config.clone(),
            selected_segment: 0,
            selected_panel: Panel::SegmentList,
            selected_field: FieldSelection::Enabled,
            should_quit: false,
            color_picker: ColorPickerComponent::new(),
            icon_selector: IconSelectorComponent::new(),
            name_input: NameInputComponent::new(),
            preview: PreviewComponent::new(),
            segment_list: SegmentListComponent::new(),
            separator_editor: SeparatorEditorComponent::new(),
            settings: SettingsComponent::new(),
            theme_selector: ThemeSelectorComponent::new(),
            help: HelpComponent::new(),
            status_message: None,
        };
        app.preview.update_preview(&config);
        app
    }

    /// Expose optional segments in the editor without enabling them implicitly.
    /// Only an explicit save writes newly added entries.
    fn with_optional_segments(mut config: Config) -> Config {
        if !config.segments.iter().any(|s| s.id == SegmentId::Effort) {
            let mut effort = crate::ui::themes::ThemePresets::effort_segment(&config.theme);
            effort.enabled = false;
            if let Some(index) = config
                .segments
                .iter()
                .position(|s| s.id == SegmentId::Model)
            {
                config.segments.insert(index + 1, effort);
            } else {
                config.segments.push(effort);
            }
        }
        if !config.segments.iter().any(|s| s.id == SegmentId::Agents) {
            config
                .segments
                .push(crate::ui::themes::ThemePresets::agents_segment(
                    &config.theme,
                ));
        }
        config
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        // Ensure themes directory and built-in themes exist
        if let Err(e) = crate::config::loader::ConfigLoader::init_themes() {
            eprintln!("Warning: Failed to initialize themes: {}", e);
        }

        // Load config
        let config = Config::load()?;

        // Terminal setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut app = App::new(config);

        // Main loop
        let result = loop {
            terminal.draw(|f| app.ui(f))?;

            if let Event::Key(key) = event::read()? {
                // Only handle KeyDown events to prevent double triggering on Windows
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle popup events first
                if app.name_input.is_open {
                    match key.code {
                        KeyCode::Esc => app.name_input.close(),
                        KeyCode::Enter => {
                            if let Some(name) = app.name_input.get_input() {
                                app.save_as_new_theme(&name);
                            }
                            app.name_input.close();
                        }
                        KeyCode::Char(c) => app.name_input.input_char(c),
                        KeyCode::Backspace => app.name_input.backspace(),
                        _ => {}
                    }
                } else if app.separator_editor.is_open {
                    match key.code {
                        KeyCode::Esc => app.separator_editor.close(),
                        KeyCode::Enter => {
                            let new_separator = app.separator_editor.get_separator();
                            app.config.style.separator = new_separator;
                            app.separator_editor.close();
                            app.preview.update_preview(&app.config);
                            app.status_message = Some("Separator updated!".to_string());
                        }
                        KeyCode::Tab => {
                            app.separator_editor.input.clear();
                            app.separator_editor.selected_preset = None;
                        }
                        KeyCode::Up => app.separator_editor.move_preset_selection(-1),
                        KeyCode::Down => app.separator_editor.move_preset_selection(1),
                        KeyCode::Char(c) => app.separator_editor.input_char(c),
                        KeyCode::Backspace => app.separator_editor.backspace(),
                        _ => {}
                    }
                } else if app.color_picker.is_open {
                    match key.code {
                        KeyCode::Esc => app.color_picker.close(),
                        KeyCode::Up => app.color_picker.move_direction(NavDirection::Up),
                        KeyCode::Down => app.color_picker.move_direction(NavDirection::Down),
                        KeyCode::Left => app.color_picker.move_direction(NavDirection::Left),
                        KeyCode::Right => app.color_picker.move_direction(NavDirection::Right),
                        KeyCode::Tab => app.color_picker.cycle_mode(),
                        KeyCode::Char('r') => app.color_picker.switch_to_rgb(),
                        KeyCode::Enter => {
                            if let Some(color) = app.color_picker.get_selected_color() {
                                app.apply_selected_color(color);
                            }
                            app.color_picker.close();
                        }
                        KeyCode::Char(c) => app.color_picker.input_char(c),
                        KeyCode::Backspace => app.color_picker.backspace(),
                        _ => {}
                    }
                } else if app.icon_selector.is_open {
                    match key.code {
                        KeyCode::Esc => app.icon_selector.close(),
                        KeyCode::Up => app.icon_selector.move_selection(-1),
                        KeyCode::Down => app.icon_selector.move_selection(1),
                        KeyCode::Tab => app.icon_selector.toggle_style(),
                        KeyCode::Char('c') if !app.icon_selector.editing_custom => {
                            app.icon_selector.start_custom_input()
                        }
                        KeyCode::Enter => {
                            if app.icon_selector.editing_custom
                                && !app.icon_selector.finish_custom_input()
                            {
                                continue;
                            }
                            if let Some(icon) = app.icon_selector.get_selected_icon() {
                                app.apply_selected_icon(icon);
                            }
                            app.icon_selector.close();
                        }
                        KeyCode::Char(c) if app.icon_selector.editing_custom => {
                            app.icon_selector.input_char(c);
                        }
                        KeyCode::Backspace if app.icon_selector.editing_custom => {
                            app.icon_selector.backspace();
                        }
                        _ => {}
                    }
                } else {
                    // Handle main app events
                    match key.code {
                        KeyCode::Esc => app.should_quit = true,
                        KeyCode::Char('s') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                // Ctrl+S: Save as new theme with name input
                                app.name_input.open("Save as New Theme", "Enter theme name");
                            } else {
                                // s: Save config to config.toml
                                if let Err(e) = app.save_config() {
                                    app.status_message =
                                        Some(format!("Failed to save config: {}", e));
                                } else {
                                    app.status_message =
                                        Some("Configuration and agent hooks saved.".to_string());
                                }
                            }
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            // w/W: Write config to current theme
                            app.write_to_current_theme();
                        }
                        KeyCode::Up => {
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.move_segment_up();
                            } else {
                                app.move_selection(-1);
                            }
                        }
                        KeyCode::Down => {
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                app.move_segment_down();
                            } else {
                                app.move_selection(1);
                            }
                        }
                        KeyCode::Enter => app.toggle_current(),
                        KeyCode::Tab => app.switch_panel(),
                        KeyCode::Char('1') => app.switch_to_theme("default"),
                        KeyCode::Char('2') => app.switch_to_theme("minimal"),
                        KeyCode::Char('3') => app.switch_to_theme("gruvbox"),
                        KeyCode::Char('4') => app.switch_to_theme("nord"),
                        KeyCode::Char('p') => app.cycle_theme(),
                        KeyCode::Char('r') => app.reset_to_theme_defaults(),
                        KeyCode::Char('e') | KeyCode::Char('E') => app.open_separator_editor(),
                        _ => {}
                    }
                }
            }

            if app.should_quit {
                break Ok(());
            }
        };

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn calculate_theme_selector_height(&self, total_width: u16) -> u16 {
        // Get all available themes dynamically
        let available_themes = crate::ui::themes::ThemePresets::list_available_themes();

        // Calculate available width (minus borders only)
        let content_width = total_width.saturating_sub(2); // Remove borders

        // Simulate the line wrapping logic
        let mut line_count = 1;
        let mut current_line_length = 0;
        let mut first_line = true;

        for (i, theme) in available_themes.iter().enumerate() {
            let marker = if self.config.theme == *theme {
                "[✓]"
            } else {
                "[ ]"
            };
            let theme_part = format!("{} {}", marker, theme);
            let separator = if i == 0 { "" } else { "  " };
            let part_with_sep = format!("{}{}", separator, theme_part);

            let would_fit = current_line_length + part_with_sep.len() <= content_width as usize;

            if would_fit || first_line {
                current_line_length += part_with_sep.len();
                first_line = false;
            } else {
                line_count += 1;
                current_line_length = theme_part.len();
            }
        }

        // Return height: content lines + borders (top + bottom)
        line_count + 2
    }

    fn calculate_help_height(&self, total_width: u16) -> u16 {
        // Use same help_items as in help.render
        let help_items = if self.color_picker.is_open {
            vec![
                "[↑↓] Navigate",
                "[Tab] Mode",
                "[Enter] Select",
                "[Esc] Cancel",
            ]
        } else if self.icon_selector.is_open {
            vec![
                "[↑↓] Navigate",
                "[Tab] Style",
                "[C] Custom",
                "[Enter] Select",
                "[Esc] Cancel",
            ]
        } else {
            vec![
                "[Tab] Switch Panel",
                "[Enter] Toggle/Edit",
                "[Shift+↑↓] Reorder",
                "[1-4] Theme",
                "[P] Switch Theme",
                "[R] Reset",
                "[E] Edit Separator",
                "[S] Save Config",
                "[W] Write Theme",
                "[Ctrl+S] Save Theme",
                "[Esc] Quit",
            ]
        };

        let content_width = total_width.saturating_sub(2); // Remove borders
        let mut lines_needed = 1u16;
        let mut current_width = 0usize;

        // Use same logic as help.render for line wrapping
        for (i, item) in help_items.iter().enumerate() {
            let item_width = item.chars().count();
            let needs_separator = i > 0 && current_width > 0;
            let separator_width = if needs_separator { 2 } else { 0 };
            let total_width = item_width + separator_width;

            if current_width + total_width > content_width as usize {
                // Need new line
                lines_needed += 1;
                current_width = item_width;
            } else {
                if needs_separator {
                    current_width += 2;
                }
                current_width += item_width;
            }
        }

        // Add lines for status message if present (empty line + message)
        if self.status_message.is_some() {
            lines_needed += 2;
        }

        // Return height: content lines + borders, max 8
        (lines_needed + 2).clamp(3, 8)
    }

    fn ui(&mut self, f: &mut Frame) {
        // Calculate required heights for dynamic sections (using full width as estimate)
        let theme_selector_height = self.calculate_theme_selector_height(f.area().width);
        let help_height = self.calculate_help_height(f.area().width);

        // Initial layout to measure preview width
        let initial_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                     // Title
                Constraint::Min(3),                        // Preview (dynamic - will recalculate)
                Constraint::Length(theme_selector_height), // Theme selector (dynamic)
                Constraint::Min(3), // Main content shrinks before the preview
                Constraint::Length(help_height), // Help (dynamic)
            ])
            .split(f.area());

        // Update preview with measured width
        self.preview
            .update_preview_with_width(&self.config, initial_layout[1].width);

        // Calculate actual preview height after content update
        let preview_height = self.preview.calculate_height();

        // Final layout with correct preview height
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                     // Title
                Constraint::Length(preview_height),        // Preview (dynamic)
                Constraint::Length(theme_selector_height), // Theme selector (dynamic)
                Constraint::Min(3), // Main content shrinks before the preview
                Constraint::Length(help_height), // Help (dynamic)
            ])
            .split(f.area());

        // Title
        let title_text = format!("CCometixLine Configurator v{}", env!("CARGO_PKG_VERSION"));
        let title = Paragraph::new(title_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(title, layout[0]);

        // Preview - use TUI-optimized statusline generation with smart segment wrapping
        // Update preview if layout width differs from initial measurement
        if layout[1].width != initial_layout[1].width {
            self.preview
                .update_preview_with_width(&self.config, layout[1].width);
        }

        // Render preview
        self.preview.render(f, layout[1]);

        // Theme selector
        self.theme_selector.render(f, layout[2], &self.config);

        // Main content (split horizontally)
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(layout[3]);

        // Segment list
        self.segment_list.render(
            f,
            content_layout[0],
            &self.config,
            self.selected_segment,
            &self.selected_panel,
        );

        // Settings panel
        self.settings.render(
            f,
            content_layout[1],
            &self.config,
            self.selected_segment,
            &self.selected_panel,
            &self.selected_field,
        );

        // Help
        self.help.render(
            f,
            layout[4],
            self.status_message.as_deref(),
            self.color_picker.is_open,
            self.icon_selector.is_open,
        );

        // Render popups on top
        if self.color_picker.is_open {
            self.color_picker.render(f, f.area());
        }
        if self.icon_selector.is_open {
            self.icon_selector.render(f, f.area());
        }
        if self.name_input.is_open {
            self.name_input.render(f, f.area());
        }
        if self.separator_editor.is_open {
            self.separator_editor.render(f, f.area());
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.selected_panel {
            Panel::SegmentList => {
                let new_selection = (self.selected_segment as i32 + delta)
                    .max(0)
                    .min((self.config.segments.len() - 1) as i32)
                    as usize;
                self.selected_segment = new_selection;
            }
            Panel::Settings => {
                let field_count = 7; // Enabled, Icon, IconColor, TextColor, TextStyle, BackgroundColor, Options
                let current_field = match self.selected_field {
                    FieldSelection::Enabled => 0i32,
                    FieldSelection::Icon => 1,
                    FieldSelection::IconColor => 2,
                    FieldSelection::TextColor => 3,
                    FieldSelection::BackgroundColor => 4,
                    FieldSelection::TextStyle => 5,
                    FieldSelection::Options => 6,
                };
                let new_field = (current_field + delta).clamp(0, field_count - 1) as usize;
                self.selected_field = match new_field {
                    0 => FieldSelection::Enabled,
                    1 => FieldSelection::Icon,
                    2 => FieldSelection::IconColor,
                    3 => FieldSelection::TextColor,
                    4 => FieldSelection::BackgroundColor,
                    5 => FieldSelection::TextStyle,
                    6 => FieldSelection::Options,
                    _ => FieldSelection::Enabled,
                };
            }
        }
    }

    fn toggle_current(&mut self) {
        match self.selected_panel {
            Panel::SegmentList => {
                // Toggle segment enabled/disabled in segment list
                if let Some(segment) = self.config.segments.get_mut(self.selected_segment) {
                    segment.enabled = !segment.enabled;
                    let segment_name = match segment.id {
                        SegmentId::Model => "Model",
                        SegmentId::Effort => "Effort",
                        SegmentId::Agents => "Agents",
                        SegmentId::Directory => "Directory",
                        SegmentId::Git => "Git",
                        SegmentId::ContextWindow => "Context Window",
                        SegmentId::Usage => "Usage",
                        SegmentId::Cost => "Cost",
                        SegmentId::Session => "Session",
                        SegmentId::OutputStyle => "Output Style",
                        SegmentId::Update => "Update",
                    };
                    let is_enabled = segment.enabled;
                    self.status_message = Some(format!(
                        "{} segment {}",
                        segment_name,
                        if is_enabled { "enabled" } else { "disabled" }
                    ));
                    if segment.id == SegmentId::Agents {
                        self.status_message = Some(format!(
                            "Agents {}. Press S to {} hooks.",
                            if is_enabled { "enabled" } else { "disabled" },
                            if is_enabled { "install" } else { "uninstall" }
                        ));
                    }
                    self.preview.update_preview(&self.config);
                }
            }
            Panel::Settings => {
                // Edit field in settings panel
                match self.selected_field {
                    FieldSelection::Enabled => {
                        // Toggle enabled state in settings panel too
                        if let Some(segment) = self.config.segments.get_mut(self.selected_segment) {
                            segment.enabled = !segment.enabled;
                            let segment_name = match segment.id {
                                SegmentId::Model => "Model",
                                SegmentId::Effort => "Effort",
                                SegmentId::Agents => "Agents",
                                SegmentId::Directory => "Directory",
                                SegmentId::Git => "Git",
                                SegmentId::ContextWindow => "Context Window",
                                SegmentId::Usage => "Usage",
                                SegmentId::Cost => "Cost",
                                SegmentId::Session => "Session",
                                SegmentId::OutputStyle => "Output Style",
                                SegmentId::Update => "Update",
                            };
                            let is_enabled = segment.enabled;
                            self.status_message = Some(format!(
                                "{} segment {}",
                                segment_name,
                                if is_enabled { "enabled" } else { "disabled" }
                            ));
                            if segment.id == SegmentId::Agents {
                                self.status_message = Some(format!(
                                    "Agents {}. Press S to {} hooks.",
                                    if is_enabled { "enabled" } else { "disabled" },
                                    if is_enabled { "install" } else { "uninstall" }
                                ));
                            }
                            self.preview.update_preview(&self.config);
                        }
                    }
                    FieldSelection::Icon => self.open_icon_selector(),
                    FieldSelection::IconColor
                    | FieldSelection::TextColor
                    | FieldSelection::BackgroundColor => self.open_color_picker(),
                    FieldSelection::TextStyle => {
                        // Toggle text bold style
                        if let Some(segment) = self.config.segments.get_mut(self.selected_segment) {
                            segment.styles.text_bold = !segment.styles.text_bold;
                            self.status_message = Some(format!(
                                "Text bold {}",
                                if segment.styles.text_bold {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ));
                            self.preview.update_preview(&self.config);
                        }
                    }
                    FieldSelection::Options => {
                        // TODO: Implement options editor
                        self.status_message =
                            Some("Options editor not implemented yet".to_string());
                    }
                }
            }
        }
    }

    fn switch_panel(&mut self) {
        self.selected_panel = match self.selected_panel {
            Panel::SegmentList => Panel::Settings,
            Panel::Settings => Panel::SegmentList,
        };
    }

    fn open_color_picker(&mut self) {
        if self.selected_panel == Panel::Settings
            && (self.selected_field == FieldSelection::IconColor
                || self.selected_field == FieldSelection::TextColor
                || self.selected_field == FieldSelection::BackgroundColor)
        {
            self.color_picker.open();
        }
    }

    fn open_icon_selector(&mut self) {
        if self.selected_panel == Panel::Settings && self.selected_field == FieldSelection::Icon {
            self.icon_selector.open(self.config.style.mode);
        }
    }

    fn apply_selected_color(&mut self, color: crate::config::AnsiColor) {
        if let Some(segment) = self.config.segments.get_mut(self.selected_segment) {
            match self.selected_field {
                FieldSelection::IconColor => segment.colors.icon = Some(color),
                FieldSelection::TextColor => segment.colors.text = Some(color),
                FieldSelection::BackgroundColor => segment.colors.background = Some(color),
                _ => {}
            }
            self.preview.update_preview(&self.config);
        }
    }

    fn apply_selected_icon(&mut self, icon: String) {
        if let Some(segment) = self.config.segments.get_mut(self.selected_segment) {
            match self.config.style.mode {
                StyleMode::Plain => segment.icon.plain = icon,
                StyleMode::NerdFont | StyleMode::Powerline => segment.icon.nerd_font = icon,
            }
            self.preview.update_preview(&self.config);
        }
    }

    fn cycle_theme(&mut self) {
        let themes = crate::ui::themes::ThemePresets::list_available_themes();
        let current_theme = &self.config.theme;
        let current_index = themes.iter().position(|t| t == current_theme).unwrap_or(0);
        let next_index = (current_index + 1) % themes.len();
        let next_theme = &themes[next_index];

        self.status_message = Some(format!("Switching to theme: {}", next_theme));
        self.switch_to_theme(next_theme);
    }

    fn switch_to_theme(&mut self, theme_name: &str) {
        self.config =
            Self::with_optional_segments(crate::ui::themes::ThemePresets::get_theme(theme_name));
        self.selected_segment = 0;
        self.preview.update_preview(&self.config);
        self.status_message = Some(format!("Switched to {} theme", theme_name));
    }

    /// Reset current theme to its default configuration
    fn reset_to_theme_defaults(&mut self) {
        let current_theme = self.config.theme.clone();
        self.config = Self::with_optional_segments(crate::ui::themes::ThemePresets::get_theme(
            &current_theme,
        ));
        self.selected_segment = 0;
        self.preview.update_preview(&self.config);
        self.status_message = Some(format!("Reset {} theme to defaults", current_theme));
    }

    fn save_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        crate::agents::integration::configure(self.config.agents_enabled())?;
        self.config.save()?;
        Ok(())
    }

    /// Move the currently selected segment up in the list
    fn move_segment_up(&mut self) {
        if self.selected_panel == Panel::SegmentList && self.selected_segment > 0 {
            let current_idx = self.selected_segment;
            self.config.segments.swap(current_idx, current_idx - 1);
            self.selected_segment -= 1;
            self.preview.update_preview(&self.config);
            self.status_message = Some("Moved segment up".to_string());
        }
    }

    /// Move the currently selected segment down in the list
    fn move_segment_down(&mut self) {
        if self.selected_panel == Panel::SegmentList
            && self.selected_segment < self.config.segments.len() - 1
        {
            let current_idx = self.selected_segment;
            self.config.segments.swap(current_idx, current_idx + 1);
            self.selected_segment += 1;
            self.preview.update_preview(&self.config);
            self.status_message = Some("Moved segment down".to_string());
        }
    }

    /// Write current config to the current theme file
    fn write_to_current_theme(&mut self) {
        let current_theme = &self.config.theme;
        match crate::ui::themes::ThemePresets::save_theme(current_theme, &self.config) {
            Ok(_) => {
                self.status_message = Some(format!("Wrote config to theme: {}", current_theme));
            }
            Err(e) => {
                self.status_message =
                    Some(format!("Failed to write to theme {}: {}", current_theme, e));
            }
        }
    }

    /// Save current config as a new theme with the given name
    fn save_as_new_theme(&mut self, theme_name: &str) {
        match crate::ui::themes::ThemePresets::save_theme(theme_name, &self.config) {
            Ok(_) => {
                // Update current theme to the new one
                self.config.theme = theme_name.to_string();
                self.status_message = Some(format!("Saved as new theme: {}", theme_name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save theme {}: {}", theme_name, e));
            }
        }
    }

    /// Open separator editor with current separator
    fn open_separator_editor(&mut self) {
        self.status_message = Some("Opening separator editor...".to_string());
        self.separator_editor.open(&self.config.style.separator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnsiColor;
    use crate::ui::themes::ThemePresets;
    use ratatui::backend::TestBackend;

    fn screen(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.ui(f)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn agents_is_opt_in_and_both_tui_panels_toggle_and_preview_it() {
        let mut config = ThemePresets::get_default();
        config
            .segments
            .retain(|segment| segment.id != SegmentId::Agents);
        let original = serde_json::to_value(&config).unwrap();
        let mut app = App::new(config);
        app.selected_segment = app
            .config
            .segments
            .iter()
            .position(|s| s.id == SegmentId::Agents)
            .unwrap();
        assert!(!app.config.agents_enabled());
        assert!(!app.preview.get_preview_cache().contains("Agents:"));
        assert!(screen(&mut app, 120, 40).contains("Agents"));
        let mut unchanged = app.config.clone();
        unchanged.segments.retain(|s| s.id != SegmentId::Agents);
        assert_eq!(serde_json::to_value(&unchanged).unwrap(), original);
        app.toggle_current();
        assert!(app.config.agents_enabled());
        assert!(app.preview.get_preview_cache().contains("Agents: 4 active"));
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("install hooks"));
        let saved = toml::to_string_pretty(&app.config).unwrap();
        let mut reopened = App::new(toml::from_str(&saved).unwrap());
        assert!(reopened.config.agents_enabled());
        reopened.selected_segment = app.selected_segment;
        reopened.switch_panel();
        reopened.toggle_current();
        assert!(!reopened.config.agents_enabled());
        assert!(!reopened.preview.get_preview_cache().contains("Agents:"));
        assert!(reopened
            .status_message
            .as_ref()
            .unwrap()
            .contains("uninstall hooks"));
    }

    #[test]
    fn agents_style_reordering_and_theme_switch_use_the_existing_controls() {
        let mut app = App::new(ThemePresets::get_default());
        app.selected_segment = app
            .config
            .segments
            .iter()
            .position(|s| s.id == SegmentId::Agents)
            .unwrap();
        app.toggle_current();
        app.move_segment_up();
        assert_eq!(
            app.config.segments[app.selected_segment].id,
            SegmentId::Agents
        );
        app.switch_panel();
        app.selected_field = FieldSelection::TextColor;
        app.apply_selected_color(AnsiColor::Color16 { c16: 3 });
        assert_eq!(
            app.config.segments[app.selected_segment].colors.text,
            Some(AnsiColor::Color16 { c16: 3 })
        );
        assert!(screen(&mut app, 120, 40).contains("Agents: 4 active"));
        app.switch_to_theme("minimal");
        assert!(!app.config.agents_enabled());
        assert!(app
            .config
            .segments
            .iter()
            .any(|s| s.id == SegmentId::Agents));
        app.reset_to_theme_defaults();
        assert!(!app.config.agents_enabled());
    }

    #[test]
    fn agent_preview_is_visible_at_standard_terminal_height() {
        let mut config = ThemePresets::get_default();
        for segment in &mut config.segments {
            segment.enabled = !matches!(segment.id, SegmentId::Agents | SegmentId::Usage);
        }
        let mut app = App::new(config);
        app.selected_segment = app
            .config
            .segments
            .iter()
            .position(|s| s.id == SegmentId::Agents)
            .unwrap();
        app.toggle_current();
        let rendered = screen(&mut app, 80, 24);
        assert!(rendered.contains("Agents: 4 active"));
        assert!(rendered.contains("install hooks"));
    }

    #[test]
    fn config_without_effort_exposes_a_disabled_option_in_the_tui() {
        let mut config = ThemePresets::get_gruvbox();
        config.segments.retain(|s| s.id != SegmentId::Effort);
        let original = toml::to_string_pretty(&config).unwrap();
        let mut app = App::new(toml::from_str(&original).unwrap());

        let rendered = screen(&mut app, 120, 40);
        assert!(rendered.contains("Effort"));
        assert_eq!(app.config.segments[1].id, SegmentId::Effort);
        assert!(!app.config.segments[1].enabled);
        assert!(!app.preview.get_preview_cache().contains("high"));

        let mut unchanged = app.config.clone();
        unchanged.segments.retain(|s| s.id != SegmentId::Effort);
        assert_eq!(
            serde_json::to_value(&unchanged).unwrap(),
            serde_json::to_value(&config).unwrap()
        );

        app.selected_segment = 1;
        app.toggle_current();
        assert!(app.preview.get_preview_cache().contains("high"));
        let saved = toml::to_string_pretty(&app.config).unwrap();
        let reopened = App::new(toml::from_str(&saved).unwrap());
        assert!(reopened.config.segments[1].enabled);
        assert_eq!(reopened.config.segments.len(), config.segments.len() + 1);
    }

    #[test]
    fn missing_effort_uses_independent_styles_and_preserves_existing_effort() {
        let mut config = ThemePresets::get_powerline_dark();
        config.theme = "custom".to_string();
        config.segments.retain(|s| s.id != SegmentId::Effort);
        config.segments[0].colors.background = Some(AnsiColor::Rgb {
            r: 30,
            g: 40,
            b: 50,
        });
        config.segments[0].styles.text_bold = true;
        let mut app = App::new(config);
        assert_ne!(
            app.config.segments[1].colors.background,
            app.config.segments[0].colors.background
        );
        assert_ne!(
            app.config.segments[1].colors.text,
            app.config.segments[0].colors.text
        );
        assert!(!app.config.segments[1].styles.text_bold);

        app.selected_segment = 1;
        app.move_segment_down();
        app.config.segments[2].icon.plain = "E".to_string();
        app.config.segments[2].colors.text = Some(AnsiColor::Color16 { c16: 3 });
        for enabled in [true, false] {
            app.config.segments[2].enabled = enabled;
            let saved = toml::to_string_pretty(&app.config).unwrap();
            let reopened = App::new(toml::from_str(&saved).unwrap());
            assert_eq!(
                serde_json::to_value(&reopened.config).unwrap(),
                serde_json::to_value(&app.config).unwrap()
            );
        }
    }

    #[test]
    fn effort_is_available_when_model_is_not_in_the_layout() {
        let mut config = ThemePresets::get_default();
        config.segments.retain(|s| s.id == SegmentId::Directory);
        let mut app = App::new(config);
        assert_eq!(app.config.segments[1].id, SegmentId::Effort);
        app.selected_segment = 1;
        app.toggle_current();
        assert!(screen(&mut app, 80, 32).contains("high"));
    }

    #[test]
    fn switching_and_resetting_a_file_theme_keeps_effort_available() {
        let mut theme = ThemePresets::get_gruvbox();
        theme.segments.retain(|s| s.id != SegmentId::Effort);
        let original = toml::to_string_pretty(&theme).unwrap();
        let dir = std::env::temp_dir().join(format!("ccline-effort-theme-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("theme.toml");
        std::fs::write(&path, &original).unwrap();
        let theme_name = path.with_extension("");

        let mut app = App::new(ThemePresets::get_default());
        app.switch_to_theme(theme_name.to_str().unwrap());
        assert_eq!(app.config.segments[1].id, SegmentId::Effort);
        assert!(!app.config.segments[1].enabled);
        app.selected_segment = 1;
        app.toggle_current();
        app.reset_to_theme_defaults();
        assert_eq!(app.config.segments[1].id, SegmentId::Effort);
        assert!(!app.config.segments[1].enabled);
        assert!(screen(&mut app, 120, 40).contains("Effort"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn effort_toggle_updates_preview_and_is_preserved_when_reopened() {
        let mut app = App::new(ThemePresets::get_cometix());
        app.selected_segment = 1;
        let rendered = screen(&mut app, 120, 40);
        assert!(rendered.contains("Effort"));
        assert!(app.preview.get_preview_cache().contains("high"));

        app.toggle_current();
        assert!(!app.config.segments[1].enabled);
        assert!(app.config.segments[0].enabled);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Effort segment disabled")
        );
        assert!(!app.preview.get_preview_cache().contains("high"));

        let saved = toml::to_string_pretty(&app.config).unwrap();
        let mut reopened = App::new(toml::from_str(&saved).unwrap());
        reopened.selected_segment = 1;
        assert!(!reopened.config.segments[1].enabled);
        reopened.switch_panel();
        reopened.toggle_current();
        assert!(reopened.config.segments[1].enabled);
        assert!(reopened.preview.get_preview_cache().contains("high"));
    }

    #[test]
    fn effort_reordering_and_style_editing_use_existing_controls() {
        let mut app = App::new(ThemePresets::get_default());
        app.selected_segment = 1;
        app.move_segment_down();
        assert_eq!(app.selected_segment, 2);
        assert_eq!(app.config.segments[2].id, SegmentId::Effort);
        app.move_segment_up();
        assert_eq!(app.config.segments[1].id, SegmentId::Effort);

        app.switch_panel();
        app.selected_field = FieldSelection::Icon;
        app.apply_selected_icon("E".to_string());
        app.selected_field = FieldSelection::TextColor;
        app.apply_selected_color(AnsiColor::Color16 { c16: 3 });
        let rendered = screen(&mut app, 80, 32);
        assert!(rendered.contains("Effort"));
        assert!(rendered.contains("E high"));
        assert_eq!(
            app.config.segments[1].colors.text,
            Some(AnsiColor::Color16 { c16: 3 })
        );
    }

    #[test]
    fn selected_effort_remains_visible_after_moving_below_viewport() {
        let mut app = App::new(ThemePresets::get_default());
        app.selected_segment = 1;
        while app.selected_segment + 1 < app.config.segments.len() {
            app.move_segment_down();
        }
        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        terminal
            .draw(|f| {
                app.segment_list.render(
                    f,
                    f.area(),
                    &app.config,
                    app.selected_segment,
                    &app.selected_panel,
                )
            })
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Effort"));
    }
}
