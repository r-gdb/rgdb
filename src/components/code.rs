use super::{gdbtty, Component};
use crate::{action::Action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use std::usize;
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;
// use tracing::debug;
use crate::tool;
use tui_term::widget::PseudoTerminal;

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,

    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl Code {
    pub fn new() -> Self {
        let s = Self::default();
        Self {
            command_tx: s.command_tx,
            config: s.config,
            vertical_scroll_state: s.vertical_scroll_state,
            vertical_scroll: s.vertical_scroll,
        }
    }
}

impl Component for Code {
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
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // debug!("start one draw");
        let texts = (1..200).map(|i| format!("{} aaaa", i)).collect::<Vec<_>>();
        let ids = (1..200).map(|i| format!("{:>}", i)).collect::<Vec<_>>();

        let [area, _, _] = tool::get_layout(area);
        let [id, src] =
            Layout::horizontal([Constraint::Min(5), Constraint::Percentage(100)]).areas(area);
        let in_size = area
            .inner(Margin {
                vertical: 1,
                horizontal: 1,
            })
            .as_size();

        let n = texts.len();
        self.vertical_scroll = self.vertical_scroll.min(n);
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(n);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(n - self.vertical_scroll);
        // self.vt100_parser.set_scrollback(2);
        let title = format!("gdb cmd {}/{} ", n - self.vertical_scroll, n);
        let b0 = Block::new()
            // .block(Block::default().title(title).borders(Borders::ALL))
            .style(
                Style::default()
                    .fg(Color::White)
                    // .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::vertical(1))
            .title(title);
        let text = Text::from(
            texts
                .into_iter()
                .map(|s| {
                    Line::from(
                        s.chars()
                            .into_iter()
                            .map(|c| Span::raw(c.to_string()))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
        );
        let id_text = Text::from(
            ids.into_iter()
                .map(|s| {
                    Line::from(
                        s.chars()
                            .into_iter()
                            .map(|c| Span::raw(c.to_string()))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
        );
        let b1 = Block::new()
            .borders(Borders::RIGHT)
            .padding(Padding::vertical(1));
        let p1 = Paragraph::new(id_text).block(b1);
        let p2 = Paragraph::new(text).block(b1);

        frame.render_widget(p1, id);
        frame.render_widget(p2, src);
        frame.render_widget(b0, area);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
        // debug!("end one draw");
        Ok(())
    }
}
