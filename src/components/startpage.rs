use super::Component;
use crate::action;
use crate::components::code;
use crate::tool;
use color_eyre::Result;
use ratatui::text::Line;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::Paragraph,
    Frame,
};
use tui_widgets::big_text::{BigText, PixelSize};

#[derive(Debug, Clone, PartialEq)]
pub struct StartPage {
    is_start: bool,
    is_horizontal: bool,
}

impl Default for StartPage {
    fn default() -> Self {
        Self::new()
    }
}

impl StartPage {
    pub fn new() -> Self {
        Self {
            is_start: true,
            is_horizontal: false,
        }
    }
    fn is_start(&self) -> bool {
        self.is_start
    }
    fn set_is_start(&mut self, val: bool) {
        self.is_start = val;
    }

    fn draw_all(&mut self, frame: &mut Frame, area: Rect) {
        if self.is_start() {
            let tool::Layouts {
                src: area,
                src_status: area_status,
                ..
            } = tool::Layouts::from((area, self.is_horizontal));
            let half = area.height.saturating_sub(12).div_euclid(2);
            let [_, area, area_version] = Layout::vertical([
                Constraint::Max(half),
                Constraint::Length(9),
                Constraint::Min(3),
            ])
            .areas(area);
            self.draw_status(frame, area_status);
            self.draw_page(frame, area);
            self.draw_version(frame, area_version);
        }
    }
    fn draw_status(&self, frame: &mut Frame, area_status: Rect) {
        let title = "*";
        let paragraph_status = Paragraph::new(title)
            .fg(Color::Black)
            .bg(Color::Gray)
            .right_aligned();
        frame.render_widget(paragraph_status, area_status);
    }
    fn draw_page(&self, frame: &mut Frame, area: Rect) {
        let big_text = BigText::builder()
            .pixel_size(PixelSize::Full)
            .style(Style::new().blue())
            .lines(vec!["rgdb".into()])
            .centered()
            .build();
        frame.render_widget(big_text, area);
    }
    fn draw_version(&self, frame: &mut Frame, area: Rect) {
        let version_str = format!("version {}", env!("CARGO_PKG_VERSION"));
        let lines = vec![
            Line::from("No Code No Bug"),
            Line::from("a tui debugger"),
            Line::from(version_str),
        ];
        let paragraph_version = Paragraph::new(lines).fg(Color::Blue).centered();
        frame.render_widget(paragraph_version, area);
    }
}

impl Component for StartPage {
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        if self.is_start() {
            match action {
                action::Action::Code(code::Action::FileReadEnd(_))
                | action::Action::Code(code::Action::AsmFileEnd) => {
                    self.set_is_start(false);
                }
                action::Action::SwapHV => {
                    self.is_horizontal = !self.is_horizontal;
                }
                _ => {}
            };
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.draw_all(frame, area);
        Ok(())
    }
}
