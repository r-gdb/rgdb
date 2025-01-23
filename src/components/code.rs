use super::{gdbtty, Component};
use crate::components::gdbmi;
use crate::tool;
use crate::{action, config::Config};
use block::Title;
use color_eyre::eyre;
use color_eyre::{eyre::Ok, Result};
use futures::stream::iter;
use futures::StreamExt;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Div;
use std::usize;
use strum::Display;
use symbols::scrollbar;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use tui_term::widget::PseudoTerminal;

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    files_set: HashSet<SrcFileData>,
    file_need_show: Option<(String, u64)>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    FileReadOutLine((String, String)),
    FileReadEnd(String),
}

#[derive(Clone, Eq, Debug)]
pub struct SrcFileData {
    pub file_name: String,
    lines: Vec<String>,
    read_done: bool,
}

impl PartialEq for SrcFileData {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
    fn ne(&self, other: &Self) -> bool {
        self.file_name != other.file_name
    }
}

impl SrcFileData {
    pub fn new(file_name: String) -> Self {
        Self {
            file_name,
            lines: vec![],
            read_done: false,
        }
    }
    pub fn add_line(&mut self, line: String) {
        self.lines.push(line);
    }
    pub fn get_read_done(&self) -> bool {
        self.read_done
    }
    pub fn set_read_done(&mut self) {
        self.read_done = true;
    }
    pub fn get_lines(&self) -> &Vec<String> {
        &self.lines
    }
    pub fn get_lines_len(&self) -> usize {
        self.lines.len()
    }
    pub fn get_lines_range(&self, start: u64, end: u64) -> Vec<&String> {
        self.lines
            .iter()
            .skip(start as usize)
            .take(end.saturating_sub(start) as usize)
            .map(|s| s)
            .collect()
    }
}

impl Hash for SrcFileData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_name.hash(state);
    }
}

impl Code {
    pub fn new() -> Self {
        Self::default()
    }
    async fn read_file(file: String, send: UnboundedSender<action::Action>) {
        match File::open(&file).await {
            std::result::Result::Ok(f) => {
                let mut f = tokio::io::BufReader::new(f);
                loop {
                    let mut line = String::new();
                    match f.read_line(&mut line).await {
                        std::result::Result::Ok(0) => {
                            match send.send(action::Action::Code(Action::FileReadEnd(file))) {
                                std::result::Result::Ok(_) => {}
                                std::result::Result::Err(e) => {
                                    error!("send error: {:?}", e);
                                }
                            }
                            break;
                        }
                        std::result::Result::Ok(n) => {
                            line = line.replace("\t", "    ");
                            match send.send(action::Action::Code(Action::FileReadOutLine((
                                file.clone(),
                                line,
                            )))) {
                                std::result::Result::Ok(_) => {}
                                std::result::Result::Err(e) => {
                                    error!("send error: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("file {} parse error: {:?}", &file, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("open file {} error: {:?}", &file, e);
            }
        }
    }
    fn get_need_show_file(&self) -> Option<(&SrcFileData, u64)> {
        match self.file_need_show {
            Some((ref file, line_id)) => {
                match self.files_set.get(&SrcFileData::new(file.clone())) {
                    Some(file_data) => {
                        if file_data.get_read_done() {
                            Some((file_data, line_id))
                        } else {
                            info!("file {} not read done", &file);
                            None
                        }
                    }
                    _ => {
                        error!("file {} not found", &file);
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

impl Component for Code {
    fn init(&mut self, _area: Size) -> Result<()> {
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
        let ret = match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => Ok(Some(action::Action::Up)),
            crossterm::event::MouseEventKind::ScrollDown => Ok(Some(action::Action::Down)),
            _ => Ok(None),
        };
        ret
    }
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        match action {
            action::Action::Tick => {
                // add any logic here that should run on every tick
            }
            action::Action::Render => {
                // add any logic here that should run on every render
            }
            action::Action::Up => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            action::Action::Down => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            action::Action::Gdbmi(gdbmi::Action::ShowFile((file, line_id))) => {
                self.file_need_show = Some((file.clone(), line_id));
                match self.files_set.contains(&SrcFileData::new(file.clone())) {
                    false => {
                        if let Some(send) = self.command_tx.clone() {
                            self.files_set.insert(SrcFileData::new(file.clone()));
                            let read_therad = Self::read_file(file.clone(), send.clone());
                            Some(tokio::spawn(async {
                                read_therad.await;
                            }));
                            debug!("read file {} start", file)
                        } else {
                            let msg = format!("read file {} thread not start", &file);
                            error!("{}", &msg);
                        }
                    }
                    true => {
                        debug!("file {} has read", &file);
                    }
                }
            }
            action::Action::Code(Action::FileReadOutLine((file, line))) => {
                match self.files_set.take(&SrcFileData::new(file.clone())) {
                    Some(mut file_data) => {
                        file_data.add_line(line);
                        self.files_set.insert(file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
            }
            action::Action::Code(Action::FileReadEnd(file)) => {
                match self.files_set.take(&SrcFileData::new(file.clone())) {
                    Some(mut file_data) => {
                        file_data.set_read_done();
                        self.files_set.insert(file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        match self.get_need_show_file() {
            Some((file, line_id)) => {
                let n = file.get_lines_len();
                let num_len = n.to_string().len() as u16;
                let [area, _] = tool::get_layout(area);
                let [area, area_status] =
                    Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)]).areas(area);
                let [area_ids, area_split, area_src] = Layout::horizontal([
                    Constraint::Min(num_len),
                    Constraint::Min(2),
                    Constraint::Percentage(100),
                ])
                .areas(area);

                let up_len = area_src.height.div_ceil(2);
                let start_line = line_id.saturating_sub(up_len as u64);
                let end_line = line_id
                    .saturating_add(area_src.height as u64)
                    .saturating_add(1);
                let src = file.get_lines_range(start_line, end_line);
                let ids = (start_line..end_line)
                    .into_iter()
                    .map(|i| (i + 1).to_string())
                    .collect::<Vec<_>>();

                // self.vertical_scroll = self.vertical_scroll.min(n);
                // self.vertical_scroll_state = self.vertical_scroll_state.content_length(n);
                // self.vertical_scroll_state = self
                //     .vertical_scroll_state
                //     .position(n - self.vertical_scroll);
                let block_split = Block::new().borders(Borders::LEFT);

                let block_src = Block::new()
                    .borders(Borders::RIGHT)
                    .style(Style::default())
                    // .title(title)
                    .title_alignment(Alignment::Center);

                let text_src = Text::from(
                    src.iter()
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
                let text_ids = Text::from(
                    ids.iter()
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
                let paragraph_id = Paragraph::new(text_ids).right_aligned();
                let paragraph_src = Paragraph::new(text_src);
                let title = format!("{} cmd {}/{} ", file.file_name, n - self.vertical_scroll, n);
                let paragraph_status = Paragraph::new(title);

                frame.render_widget(paragraph_id, area_ids);
                frame.render_widget(block_split, area_split);
                frame.render_widget(paragraph_src, area_src);
                frame.render_widget(paragraph_status, area_status);
                frame.render_widget(block_src, area_src);
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .symbols(scrollbar::VERTICAL);
                frame.render_stateful_widget(scrollbar, area, &mut self.vertical_scroll_state);
            }
            _ => {}
        }
        Ok(())
    }
}
