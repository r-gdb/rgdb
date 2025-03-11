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
    is_horizontal: bool,
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
            is_horizontal: false,
            mode: Mode::default(),
        }
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    fn is_show(&self) -> bool {
        self.is_show
    }
    // fn set_is_show(&mut self, val: bool) {
    //     self.is_show = val;
    // }
    fn hit_text(&self) -> Vec<Span<'_>> {
        let hits = match self.mode {
            Mode::Gdb => vec!["<Ctrl-q> Exit", "<Esc> CODE"],
            Mode::Code => vec![
                "<←↓↑→> Scroll Code",
                "<Ctrl-w> Swap",
                "<Ctrl-q> Exit",
                "<Esc> GDB",
            ],
        };
        hits.into_iter()
            .map(|hit| {
                Span::from(hit)
                    .bg(Color::Rgb(160, 160, 160))
                    .fg(Color::Black)
            })
            .collect::<Vec<_>>()
    }

    fn mode_text(&self) -> Span<'_> {
        let mode_name = match self.mode {
            Mode::Gdb => "GDB",
            Mode::Code => "CODE",
        };
        Span::from(mode_name).fg(Color::Gray).bg(Color::Black)
    }

    fn draw_all(&mut self, frame: &mut Frame, area: Rect) {
        if self.is_show() {
            let tool::Layouts { status: area, .. } = (area, self.is_horizontal).into();
            self.draw_status(frame, area);
        }
    }
    fn draw_status(&self, frame: &mut Frame, area_status: Rect) {
        let mode_name = self.mode_text();
        let hit = self.hit_text();
        let s = Span::from(" ").bg(Color::Black);
        let hits = hit
            .into_iter()
            .flat_map(|it| vec![it, s.clone()])
            .chain(std::iter::once(mode_name));
        let line = Line::from_iter(hits);
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
                action::Action::SwapHV => self.is_horizontal = !self.is_horizontal,
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
