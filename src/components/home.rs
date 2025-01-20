use super::{gdbtty, Component};
use crate::{action::Action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use std::usize;
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;
// use tracing::debug;
use tui_term::widget::PseudoTerminal;

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,

    vt100_parser: vt100::Parser,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl Home {
    pub fn new() -> Self {
        let s = Self::default();
        Self {
            command_tx: s.command_tx,
            config: s.config,
            vt100_parser: vt100::Parser::new(24, 80, usize::MAX),
            vertical_scroll_state: s.vertical_scroll_state,
            vertical_scroll: s.vertical_scroll,
        }
    }
    fn get_text_hight(&mut self, _area: &Rect) -> usize {
        let now_scrollback = self.vt100_parser.screen().scrollback();
        self.vt100_parser.set_scrollback(usize::MAX);
        let ret = self.vt100_parser.screen().scrollback();
        self.vt100_parser.set_scrollback(now_scrollback);
        ret
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
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            Action::Down => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            Action::Gdbtty(gdbtty::Action::Out(out)) => {
                self.vt100_parser.process(out.as_slice());
                self.vt100_parser.set_scrollback(0);
                self.vertical_scroll = 0;
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
        let in_size = area
            .inner(Margin {
                vertical: 1,
                horizontal: 1,
            })
            .as_size();
        self.vt100_parser.set_size(in_size.height, in_size.width);
        let n = self.get_text_hight(&area);
        self.vertical_scroll = self.vertical_scroll.min(n);
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(n);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(n - self.vertical_scroll);
        self.vt100_parser.set_scrollback(self.vertical_scroll);
        let screen = self.vt100_parser.screen();
        let cursor_show = self.vertical_scroll == 0;
        // self.vt100_parser.set_scrollback(2);
        let title = format!(
            "gdb cmd {}/{} {} area size {:?}",
            n - self.vertical_scroll,
            n,
            screen.scrollback(),
            screen.size()
        );
        let pseudo_term = PseudoTerminal::new(screen)
            .block(Block::default().title(title).borders(Borders::ALL))
            .cursor(tui_term::widget::Cursor::default().visibility(cursor_show))
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(pseudo_term, area);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
        // debug!("end one draw");
        Ok(())
    }
}
