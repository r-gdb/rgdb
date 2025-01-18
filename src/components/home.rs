use std::str::Bytes;

use ansi_to_tui::IntoText;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use symbols::scrollbar;
use text::ToLine;
use tokio::sync::mpsc::UnboundedSender;

use super::{gdbtty, Component};
use crate::{action::Action, config::Config};
use tracing::error;

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
}

impl Component for Home {
    fn init(&mut self, area: Size) -> Result<()> {
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
                self.vertical_scroll = self
                    .vertical_scroll
                    .saturating_add(1)
                    .min(self.text.len() - 1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            Action::GdbRead(gdbtty::Action::Newline(mut out)) => {
                let mut line = self.text.pop().map_or(vec![], |s| s);
                line.append(&mut out);
                self.text.push(line);
                return Ok(Some(Action::Render));
            }
            Action::GdbRead(gdbtty::Action::Oldline(out)) => {
                self.text.push(out);
                return Ok(Some(Action::Render));
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, _, area] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .areas(area);
        let text = self
            .text
            .iter()
            .map(|vec| String::from_iter(vec.iter().map(|c| char::from(*c))))
            .map(|mut s| {
                s.push('\n');
                s
            })
            .collect::<String>();
        let text = text.into_text().unwrap();

        let n = self.text.len();
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(n);
        let title = format!("gdb cmd {}/{}", self.vertical_scroll + 1, n);
        let paragraph = Paragraph::new(text)
            .gray()
            .block(Block::bordered().gray().title(title))
            .scroll((self.vertical_scroll as u16, 0));
        frame.render_widget(paragraph, area);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
        Ok(())
    }
}
