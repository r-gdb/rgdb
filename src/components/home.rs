use super::{gdbtty, Component};
use crate::{action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
// use tracing::debug;
use crate::tool;
use serde::{Deserialize, Serialize};
use strum::Display;
use tui_term::widget::PseudoTerminal;

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    vt100_parser: vt100::Parser,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Up(usize),
    Down(usize),
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
            area: s.area,
        }
    }
    fn get_text_hight(&mut self, _area: &Rect) -> usize {
        let now_scrollback = self.vt100_parser.screen().scrollback();
        self.vt100_parser.set_scrollback(usize::MAX);
        let ret = self.vt100_parser.screen().scrollback();
        self.vt100_parser.set_scrollback(now_scrollback);
        ret
    }
    fn scroll_down(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(n);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }
    fn scroll_up(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(n);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }
    fn set_area(&mut self, area: &layout::Size) {
        let area = Rect::new(0, 0, area.width, area.height);
        let [_, _, area] = tool::get_layout(area);
        self.area = area;
    }
    fn set_vt100_area(&mut self, area: &layout::Size) {
        let area = Rect::new(0, 0, area.width, area.height);
        let [_, _, area] = tool::get_layout(area);
        let in_size = area
            .inner(Margin {
                vertical: 1,
                horizontal: 1,
            })
            .as_size();
        self.vt100_parser.set_size(in_size.height, in_size.width);
    }
}

impl Component for Home {
    fn init(&mut self, area: Size) -> Result<()> {
        self.set_area(&area);
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<action::Action>) -> Result<()> {
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
    ) -> Result<Option<action::Action>> {
        // debug!("gen mouseEvent {:?}", &mouse);
        let is_in = self
            .area
            .contains(ratatui::layout::Position::new(mouse.column, mouse.row));
        match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => match is_in {
                true => Ok(Some(action::Action::Home(Action::Up(3 as usize)))),
                false => Ok(None),
            },
            crossterm::event::MouseEventKind::ScrollDown => match is_in {
                true => Ok(Some(action::Action::Home(Action::Down(3 as usize)))),
                false => Ok(None),
            },
            _ => Ok(None),
        }
    }
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        match action {
            action::Action::Tick => {
                // add any logic here that should run on every tick
            }
            action::Action::Render => {
                // add any logic here that should run on every render
            }
            action::Action::Resize(x, y) => {
                self.set_area(&layout::Size::new(x, y));
                self.set_vt100_area(&layout::Size::new(x, y));
            }
            action::Action::Home(Action::Up(s)) => {
                self.scroll_up(s);
            }
            action::Action::Home(Action::Down(s)) => {
                self.scroll_down(s);
            }
            action::Action::Gdbtty(gdbtty::Action::Out(out)) => {
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
        let [_, _, area] = tool::get_layout(area);
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
