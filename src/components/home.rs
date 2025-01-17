use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,

    text: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Home {
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
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
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
        let text = vec!["hello world", "aaaaa", "ccccc", "ddddd"]
            .into_iter()
            .map(|s| Line::from(s))
            .collect::<Vec<_>>();
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(text.len());
        let paragraph = Paragraph::new(text)
            .gray()
            .block(Block::bordered().gray().title("gdb cmd"))
            .scroll((self.vertical_scroll as u16, 0));
        frame.render_widget(paragraph, area);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
        Ok(())
    }
}
