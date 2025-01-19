use std::io::Read;

use super::{gdbtty, Component};
use crate::{action::Action, config::Config};
use ansi_to_tui::IntoText;
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,

    text: Vec<Vec<u8>>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
    fn get_area_hight(&self, area: &Rect) -> usize {
        area.as_size().height.saturating_sub(2) as usize
    }
    fn get_text_hight(&self, area: &Rect) -> usize {
        // debug!(
        //     "text_hight {:?} {:?}",
        //     self.text.len(),
        //     area.as_size().height
        // );
        let hight = self.get_area_hight(area);
        self.text.len().saturating_sub(hight)
    }
    fn get_text_draw_rage(&self, area: &Rect) -> (usize, usize) {
        let n = self.get_text_hight(area);
        let hight = self.get_area_hight(area);

        (
            self.vertical_scroll,
            self.vertical_scroll
                .saturating_add(hight)
                .min(n.saturating_add(hight)),
        )
    }
}

impl Component for Home {
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }
    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Result<Option<Action>> {
        // debug!("gen mouseEvent {:?}", &mouse);
        let ret = match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => Ok(Some(Action::Up)),
            crossterm::event::MouseEventKind::ScrollDown => Ok(Some(Action::Down)),
            _ => Ok(None),
        };
        ret
    }
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::Up => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            Action::Down => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            Action::GdbRead(gdbtty::Action::Newline(mut out)) => {
                let mut line = self.text.pop().map_or(vec![], |s| s);
                line.append(&mut out);
                self.text.push(line);
                self.vertical_scroll = self.text.len();
                return Ok(None);
            }
            Action::GdbRead(gdbtty::Action::Oldline(out)) => {
                self.text.push(out);
                self.vertical_scroll = self.text.len();
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // debug!("start one draw");
        let [_, _, area] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .areas(area);
        // debug!(
        //     "vertical sroll {} all len {} {:?}",
        //     &self.vertical_scroll,
        //     self.get_text_hight(&area),
        //     area
        // );
        let n = self.get_text_hight(&area);
        self.vertical_scroll = self.vertical_scroll.min(n);
        let line_range = self.get_text_draw_rage(&area);
        // debug!("vertical sroll {}", self.vertical_scroll);
        let text = self.text.as_slice()[line_range.0..line_range.1.min(self.text.len())]
            .iter()
            .enumerate()
            .map(|(id, vec)| {
                let id = line_range.0.saturating_add(id);
                let mut parser = vt100::Parser::new(24, 80, 0);
                let vec = vec
                    .into_iter()
                    .map(|c| match c {
                        b'\t' => vec![b' ', b' ', b' ', b' '],
                        _ => vec![*c],
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                parser.process(&vec);
                let vec_format = parser.screen().contents_formatted();
                let s = String::from_iter(vec_format.into_iter().map(|c| char::from(c)));
                let s = format!("{}|{}\n", id, s);
                s
            })
            .collect::<String>();
        let text = text.into_text().unwrap();

        self.vertical_scroll_state = self.vertical_scroll_state.content_length(n);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        let title = format!("gdb cmd {}/{}", self.vertical_scroll, n);
        let paragraph = Paragraph::new(text)
            .gray()
            .block(Block::bordered().gray().title(title))
            // .scroll((self.vertical_scroll as u16, 0))
            ;
        frame.render_widget(paragraph, area);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
        // debug!("end one draw");
        Ok(())
    }
}
