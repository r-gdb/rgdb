use std::time::{Duration, Instant};

use super::{
    gdbtty,
    mouse_select::{self, MouseSelect, SelectionRange, TextSelection},
    Component,
};
use crate::{action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use symbols::scrollbar;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};
// use tracing::debug;
use crate::app::Mode;
use crate::tool;
use serde::{Deserialize, Serialize};
use strum::Display;
use tui_term::widget::PseudoTerminal;

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    vt100_parser: vt100::Parser,
    vt100_parser_buffer: Vec<u8>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    area: Rect,
    area_change_time: Option<Instant>,
    mode: Mode,
    is_horizontal: bool,
    focus: bool,
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
            vt100_parser_buffer: s.vt100_parser_buffer,
            area_change_time: None,
            mode: s.mode,
            is_horizontal: s.is_horizontal,
            focus: true,
        }
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
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
        tool::Layouts { gdb: self.area, .. } = (area, self.is_horizontal).into();
    }
    fn set_vt100_area(&mut self, area: &layout::Size) {
        let area = Rect::new(0, 0, area.width, area.height);
        let tool::Layouts { gdb: area, .. } = (area, self.is_horizontal).into();
        let in_size = area
            .inner(Margin {
                vertical: 0,
                horizontal: 1,
            })
            .as_size();
        debug!("start resize {}", self.vt100_parser_buffer.len());
        self.vt100_parser = vt100::Parser::new(in_size.height, in_size.width, usize::MAX);
        self.vt100_parser
            .process(self.vt100_parser_buffer.as_slice());
        debug!("end resize {}", self.vt100_parser_buffer.len());
    }
    fn set_scroll_bar_status(&mut self, test_len: usize) {
        self.vertical_scroll = self.vertical_scroll.min(test_len);
        self.vertical_scroll_state = self.vertical_scroll_state.content_length(test_len);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(test_len - self.vertical_scroll);
    }
    fn draw_cmd(&mut self, frame: &mut Frame, area: Rect) {
        self.vt100_parser.set_scrollback(self.vertical_scroll);
        let screen = self.vt100_parser.screen();
        let cursor_show = self.vertical_scroll == 0 && self.mode == Mode::Gdb && self.focus;
        let cursor_style = Style::default().fg(Color::Rgb(255, 204, 0));
        let pseudo_term = PseudoTerminal::new(screen)
            .cursor(
                tui_term::widget::Cursor::default()
                    .style(cursor_style)
                    .overlay_style(cursor_style.add_modifier(Modifier::REVERSED))
                    .visibility(cursor_show),
            )
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(pseudo_term, area);

        // debug!("end one draw");
    }
    fn draw_scroll(&mut self, frame: &mut Frame, area: Rect, test_len: usize) {
        let [area_in, _] =
            Layout::horizontal(vec![Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        let text_scroll_status = match self.vertical_scroll {
            0 => String::new(),
            _ => format!("[{}/{}]", test_len - self.vertical_scroll, test_len),
        };
        let scroll_block = Block::default().title(
            Line::from(text_scroll_status)
                .right_aligned()
                .fg(Color::White),
        );
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        frame.render_widget(scroll_block, area_in);
        frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
    }
    fn chenge_tui_poisition_to_tty_position(&self, row: u16, column: u16) -> Option<(u16, u16)> {
        let (start_row, start_col) = (
            row.saturating_sub(self.area.y),
            column.saturating_sub(self.area.x),
        );
        Some((start_row, start_col))
    }
}

impl Component for Home {
    fn init(&mut self, area: Size) -> Result<()> {
        self.set_area(&area);
        self.set_vt100_area(&area);
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<action::Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }
    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> Result<Option<crate::action::Action>> {
        use crate::tui::Event;
        // debug!("event {:?}", &event);
        let action = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            Some(Event::FocusLost) => {
                self.focus = false;
                None
            }
            Some(Event::FocusGained) => {
                self.focus = true;
                None
            }
            _ => None,
        };
        Ok(action)
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
        let action = match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => match is_in {
                true => Some(action::Action::Home(Action::Up(3_usize))),
                false => None,
            },
            crossterm::event::MouseEventKind::ScrollDown => match is_in {
                true => Some(action::Action::Home(Action::Down(3_usize))),
                false => None,
            },
            _ => None,
        };
        Ok(action)
    }
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        let ret_action = match action {
            action::Action::Resize(x, y) => {
                self.set_area(&layout::Size::new(x, y));
                // self.set_vt100_area(&layout::Size::new(x, y));
                self.area_change_time = Some(Instant::now());
                None
            }
            action::Action::Home(Action::Up(s)) => {
                self.scroll_up(s);
                None
            }
            action::Action::Home(Action::Down(s)) => {
                self.scroll_down(s);
                None
            }
            action::Action::Mode(mode) => {
                self.set_mode(mode);
                None
            }
            action::Action::Gdbtty(gdbtty::Action::Out(out)) => {
                self.vt100_parser_buffer.append(out.clone().as_mut());
                self.vt100_parser.process(out.as_slice());
                self.vt100_parser.set_scrollback(0);
                self.vertical_scroll = 0;
                None
            }
            action::Action::SwapHV => {
                self.is_horizontal = !self.is_horizontal;
                None
            }
            action::Action::MouseSelect(mouse_select::Action::SelectionRange(select_action)) => {
                match select_action {
                    (true, select) => {
                        if let Some(send) = self.command_tx.clone() {
                            tool::send_action(
                                &send,
                                action::Action::MouseSelect(
                                    mouse_select::Action::DelectSelectionRange(
                                        mouse_select::SelectionRangeType::GdbtTTYWindeow,
                                    ),
                                ),
                            );
                        } else {
                            error!("{}", "send mouse select error");
                        }
                        let action = self
                            .get_selected_text(&select)
                            .and_then(|text| Some(action::Action::CopyStr(text)));
                        action
                    }
                    (false, select) => match self.get_selected_area(&select) {
                        Some(select_area) => {
                            let action = Some(action::Action::MouseSelect(
                                mouse_select::Action::AddSelectionRange((
                                    mouse_select::SelectionRangeType::GdbtTTYWindeow,
                                    select_area,
                                )),
                            ));
                            action
                        }
                        None => None,
                    },
                }
            }
            _ => None,
        };
        Ok(ret_action)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // debug!("start one draw");
        if let Some(now) = self.area_change_time {
            match now.elapsed() > Duration::from_millis(400) {
                true => {
                    self.area_change_time = None;
                    self.set_vt100_area(&area.as_size());
                }
                false => {
                    return Ok(());
                }
            }
        }
        let tool::Layouts { gdb: area, .. } = (area, self.is_horizontal).into();
        let n = self.get_text_hight(&area);
        self.set_scroll_bar_status(n);
        self.draw_cmd(frame, area);
        self.draw_scroll(frame, area, n);
        Ok(())
    }
}

impl TextSelection for Home {
    fn get_selected_text(&self, select: &MouseSelect) -> Option<String> {
        debug!("get selected area {:?}", &select);
        let pos_start = Position::new(select.start.1, select.start.0);
        let pos_end = Position::new(select.end.1, select.end.0);
        if !(self.area.contains(pos_start) && self.area.contains(pos_end)) {
            return None;
        }
        let screen = self.vt100_parser.screen();
        let MouseSelect {
            start: (select_start_row, select_start_col),
            end: (select_end_row, select_end_col),
        } = select;
        let tty_select_start =
            self.chenge_tui_poisition_to_tty_position(*select_start_row, *select_start_col)?;
        let tty_select_end =
            self.chenge_tui_poisition_to_tty_position(*select_end_row, *select_end_col)?;
        let ans = screen.contents_between(
            tty_select_start.0,
            tty_select_start.1,
            tty_select_end.0,
            tty_select_end.1,
        );
        Some(ans)
    }

    fn get_selected_area(&self, select: &MouseSelect) -> Option<Vec<SelectionRange>> {
        debug!("get selected area {:?}", &select);
        let pos_start = Position::new(select.start.1, select.start.0);
        let pos_end = Position::new(select.end.1, select.end.0);
        if !(self.area.contains(pos_start) && self.area.contains(pos_end)) {
            return None;
        }
        let (_start_row, start_col) = (self.area.y, self.area.x);
        let screen = self.vt100_parser.screen();
        let (_, width) = screen.size();
        let MouseSelect {
            start: (select_start_row, select_start_col),
            end: (select_end_row, select_end_col),
        } = select;
        let tty_select_start =
            self.chenge_tui_poisition_to_tty_position(*select_start_row, *select_start_col)?;
        let tty_select_end =
            self.chenge_tui_poisition_to_tty_position(*select_end_row, *select_end_col)?;
        let select_row_len: usize =
            (select_end_row.saturating_sub(*select_start_row) as usize).saturating_add(1);
        let ret = (*select_start_row..*select_end_row + 1)
            .enumerate()
            .map(|(id, line_id)| {
                let id = id + 1;
                let start = match id == 1 {
                    true => tty_select_start.1,
                    false => 0,
                } as usize;
                let end = match id == select_row_len {
                    true => tty_select_end.1,
                    false => width,
                } as usize;
                SelectionRange {
                    line_number: line_id as usize,
                    start_column: start.saturating_add(start_col as usize),
                    end_column: end.saturating_add(start_col as usize),
                }
            })
            .collect::<Vec<SelectionRange>>();
        debug!("get selected area {:?}", &ret);
        Some(ret)
    }
}
