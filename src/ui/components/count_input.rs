use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Default)]
pub struct CountInputComponent {
    pub is_open: bool,
    pub input: String,
    pub error: Option<String>,
}

impl CountInputComponent {
    pub fn open(&mut self, value: usize) {
        self.is_open = true;
        self.input = value.to_string();
        self.error = None;
    }

    pub fn value(&mut self) -> Option<usize> {
        match self.input.parse::<usize>() {
            Ok(value) => Some(value),
            Err(_) => {
                self.error = Some("Enter a non-negative whole number".into());
                None
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let width = 58.min(area.width);
        let height = 7.min(area.height);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        let text = Text::from(vec![
            Line::from(format!("> {}", self.input)),
            Line::from("0: counts only; fewer names appear on narrow terminals"),
            Line::styled(
                self.error.as_deref().unwrap_or(""),
                Style::default().fg(Color::Red),
            ),
            Line::from("[Enter] Apply  [Esc] Cancel  [Ctrl+U] Clear"),
        ]);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Max agents")),
            popup,
        );
    }
}
