use super::Component;
use crate::action;
use crate::app::Mode;
use crate::tool;
use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    widgets::Paragraph,
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct StatusBar {
    is_show: bool,
    mode: Mode,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            is_show: true,
            mode: Mode::default(),
        }
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    fn is_show(&self) -> bool {
        self.is_show
    }
    fn set_is_show(&mut self, val: bool) {
        self.is_show = val;
    }
    fn hit_text(&self) -> Span<'_> {
        let change_mode = match self.mode {
            Mode::Gdb => "CODE",
            Mode::Code => "GDB",
        };
        let change_mode_hit = format!("<Esc> {}", change_mode);
        let hit = Span::from(change_mode_hit).bg(Color::Gray).fg(Color::Black);
        hit
    }

    fn mode_text(&self) -> Span<'_> {
        let mode_name = match self.mode {
            Mode::Gdb => "GDB",
            Mode::Code => "CODE",
        };
        let hit = Span::from(mode_name).fg(Color::Gray).bg(Color::Black);
        hit
    }

    fn draw_all(&mut self, frame: &mut Frame, area: Rect) {
        if self.is_show() {
            let [_, _, _, area] = tool::get_layout(area);
            self.draw_status(frame, area);
        }
    }
    fn draw_status(&self, frame: &mut Frame, area_status: Rect) {
        let mode_name = self.mode_text();
        let hit = self.hit_text();
        let s = Span::from(" ").bg(Color::Black);
        let line = Line::from_iter(vec![hit, s, mode_name]);
        let paragraph_status = Paragraph::new(line)
            .fg(Color::Gray)
            .bg(Color::Black)
            .right_aligned();
        frame.render_widget(paragraph_status, area_status);
    }
}

impl Component for StatusBar {
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        if self.is_show() {
            match action {
                action::Action::Mode(mode) => self.set_mode(mode),
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
