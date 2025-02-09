use super::Component;
use crate::components::code::asmfuncdata::AsmFuncData;
use crate::components::code::breakpoint::BreakPointData;
use crate::components::code::srcfiledata::SrcFileData;
use crate::components::gdbmi;
use crate::tool;
use crate::tool::{FileData, HashSelf, HighlightFileData, TextFileData};
use crate::{action, config::Config};
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use strum::Display;
use symbols::scrollbar;
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

mod asmfuncdata;
mod breakpoint;
mod srcfiledata;
mod test;

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    files_set: HashMap<Rc<String>, SrcFileData>,
    asm_func_set: HashMap<Rc<String>, AsmFuncData>,
    breakpoint_set: HashMap<Rc<String>, BreakPointData>,
    file_need_show: FileNeedShow,
    vertical_scroll: usize,
    horizontial_scroll: usize,
    area: Rect,
}

#[derive(Default)]
pub enum FileNeedShow {
    #[default]
    None,
    SrcFile(FileNeedShowSrcFile),
    AsmFile(FileNeedShowAsmFunc),
}
pub struct FileNeedShowSrcFile {
    pub name: String,
    pub line: u64,
}
pub struct FileNeedShowAsmFunc {
    pub name: String,
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    FileReadOneLine((String, String)),
    FileReadEnd(String),
    AsmFileEnd,
    FilehighlightLine((String, Vec<(ratatui::style::Color, String)>)),
    FilehighlightEnd(String),
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
}

impl Code {
    pub fn new() -> Self {
        Self::default()
    }
    async fn highlight_file(
        file_name: String,
        lines: Vec<String>,
        send: UnboundedSender<action::Action>,
    ) {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = syntect::highlighting::ThemeSet::load_defaults();
        let ext = Path::new(&file_name).extension().and_then(OsStr::to_str);
        if let Some(ext) = ext {
            if let Some(syntax) = ps.find_syntax_by_extension(ext) {
                let mut h = HighlightLines::new(syntax, &ts.themes["base16-mocha.dark"]);
                lines.iter().for_each(|s| match h.highlight_line(s, &ps) {
                    std::result::Result::Ok(ranges) => {
                        let e = ranges
                            .into_iter()
                            .map(|(c, s)| {
                                (
                                    ratatui::style::Color::Rgb(
                                        c.foreground.r,
                                        c.foreground.g,
                                        c.foreground.b,
                                    ),
                                    s.to_string(),
                                )
                            })
                            .collect();
                        match send.send(action::Action::Code(Action::FilehighlightLine((
                            file_name.clone(),
                            e,
                        )))) {
                            std::result::Result::Ok(_) => {}
                            std::result::Result::Err(e) => {
                                error!("send error: {:?}", e);
                            }
                        }
                        // debug!("highlight {:?}", ranges);
                    }
                    std::result::Result::Err(e) => {
                        error!("file {} highlight fail {} {}", &file_name, &s, e);
                    }
                });
                match send.send(action::Action::Code(Action::FilehighlightEnd(file_name))) {
                    std::result::Result::Ok(_) => {}
                    std::result::Result::Err(e) => {
                        error!("send error: {:?}", e);
                    }
                }
            } else {
                error!("file {} not have extension", &ext);
            }
        } else {
            error!("file {} not have extension", &file_name);
        }
    }
    fn read_file_filter(line: String) -> String {
        line.replace("\u{0}", r##"\{NUL}"##)
            .replace("\u{1}", r##"\{SOH}"##)
            .replace("\u{2}", r##"\{STX}"##)
            .replace("\u{3}", r##"\{ETX}"##)
            .replace("\u{4}", r##"\{EOT}"##)
            .replace("\u{5}", r##"\{ENQ}"##)
            .replace("\u{6}", r##"\{ACK}"##)
            .replace("\u{7}", r##"\{BEL}"##)
            .replace("\u{8}", r##"\{BS}"##)
            .replace("\t", "    ") // \u{9}
            .replace("\u{b}", r##"\{VT}"##)
            .replace("\u{c}", r##"\{FF}"##)
            .replace("\r", "") //\u{d}
            .replace("\u{e}", r##"\{SO}"##)
            .replace("\u{f}", r##"\{SI}"##)
            .replace("\u{10}", r##"\{DLE}"##)
            .replace("\u{11}", r##"\{DC1}"##)
            .replace("\u{12}", r##"\{DC2}"##)
            .replace("\u{13}", r##"\{DC3}"##)
            .replace("\u{14}", r##"\{DC4}"##)
            .replace("\u{15}", r##"\{NAK}"##)
            .replace("\u{16}", r##"\{SYN}"##)
            .replace("\u{17}", r##"\{ETB}"##)
            .replace("\u{18}", r##"\{CAN}"##)
            .replace("\u{19}", r##"\{EM}"##)
            .replace("\u{1a}", r##"\{SUB}"##)
            .replace("\u{1b}", r##"\{ESC}"##)
            .replace("\u{1c}", r##"\{FS}"##)
            .replace("\u{1d}", r##"\{GS}"##)
            .replace("\u{1e}", r##"\{RS}"##)
            .replace("\u{1f}", r##"\{US}"##)
            .replace("\u{7f}", r##"\{DEL}"##)
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
                        std::result::Result::Ok(_n) => {
                            line = Self::read_file_filter(line);
                            match send.send(action::Action::Code(Action::FileReadOneLine((
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
    fn get_need_show_file(&self) -> Option<(&dyn FileData, u64)> {
        match &self.file_need_show {
            FileNeedShow::None => None,
            FileNeedShow::SrcFile(file) => match self.files_set.get(&Rc::new(file.name.clone())) {
                Some(file_data) => {
                    if file_data.get_read_done() {
                        Some((file_data as &dyn FileData, file.line))
                    } else {
                        info!("file {} not read done", &file.name);
                        None
                    }
                }
                _ => {
                    error!("file {} not found", &file.name);
                    None
                }
            },
            FileNeedShow::AsmFile(func) => {
                let name = &func.name;
                match self.asm_func_set.get(&Rc::new(name.clone())) {
                    Some(asm_file) => match asm_file.get_read_done() {
                        true => match asm_file.get_line_id(&func.addr) {
                            Some(id) => Some((asm_file, id)),
                            _ => {
                                error!(
                                    "asm file {} not find {:?}, in {:?}",
                                    name, &func.addr, &asm_file
                                );
                                None
                            }
                        },
                        _ => {
                            info!("asm file {} not read done", name);
                            None
                        }
                    },
                    _ => {
                        error!("asm {} not found", &name);
                        None
                    }
                }
            }
        }
    }
    fn set_area(&mut self, area: &layout::Size) {
        let area = Rect::new(0, 0, area.width, area.height);
        let [area, _, _, _] = tool::get_layout(area);
        self.area = area;
    }
    fn file_down(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(n);
    }
    fn file_up(&mut self, n: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(n);
    }
    fn file_left(&mut self, n: usize) {
        self.horizontial_scroll = self.horizontial_scroll.saturating_sub(n);
    }
    fn file_right(&mut self, n: usize) {
        self.horizontial_scroll = self.horizontial_scroll.saturating_add(n);
    }
    fn legalization_vertical_scroll_range(&mut self, hight: usize, n: usize) -> (usize, usize) {
        let up_half = hight.div_euclid(2);
        let down_half = hight.div_ceil(2);
        let start = up_half.saturating_add(1);
        let end = n.saturating_sub(down_half).saturating_add(1).max(start);
        self.vertical_scroll = self.vertical_scroll.max(start);
        self.vertical_scroll = self.vertical_scroll.min(end);
        (start, end)
    }
    fn legalization_horizontial_scroll_range(
        &mut self,
        width: usize,
        text_len: usize,
    ) -> (usize, usize) {
        let end = text_len.saturating_add(2_usize).saturating_sub(width);
        let start = 0_usize;
        self.horizontial_scroll = self.horizontial_scroll.max(start);
        self.horizontial_scroll = self.horizontial_scroll.min(end);
        (start, end)
    }
    fn get_windows_show_file_range(&self, hight: usize) -> (usize, usize) {
        let up_half = hight.div_euclid(2);
        let start = self.vertical_scroll.saturating_sub(up_half);
        let end = start.saturating_add(hight);
        (start, end)
    }
    fn get_breakpoint_in_file_range(
        &self,
        file: &dyn FileData,
        start_line: usize,
        end_line: usize,
    ) -> HashMap<u64, bool> {
        let file_name = file.get_file_name();
        let ans = self
            .breakpoint_set
            .iter()
            .flat_map(|(_, val)| match val {
                BreakPointData::Signal(p) => {
                    vec![(p.fullname.clone(), p.line, p.enabled)]
                }
                BreakPointData::Multiple(p) => p
                    .bps
                    .iter()
                    .map(|bp| (bp.fullname.clone(), bp.line, bp.enabled && p.enabled))
                    .collect::<Vec<_>>(),
            })
            .filter(|(name, line, _)| {
                *name == file_name && start_line <= *line as usize && *line as usize <= end_line
            })
            .map(|(_, line, enable)| (line, enable))
            .fold(HashMap::new(), |mut m, (line, enable)| {
                m.entry(line)
                    .and_modify(|enable_old| *enable_old |= enable)
                    .or_insert(enable);
                m
            });
        ans
    }
    fn draw_all(
        &self,
        frame: &mut Frame,
        start_line: usize,
        end_line: usize,
        line_id: usize,
        n: usize,
        file: &dyn FileData,
        area_ids: Rect,
        area_split: Rect,
        area_src: Rect,
        area_status: Rect,
    ) {
        let line_id_start_0 = if start_line <= line_id {
            Some(line_id.saturating_sub(start_line))
        } else {
            None
        };
        self.draw_src(frame, file, start_line, end_line, area_src);
        self.draw_breakpoint(frame, file, start_line, end_line, area_ids);
        self.draw_id(frame, start_line, end_line, line_id, area_ids);
        self.draw_split(frame, area_split);
        self.draw_currect_pointer(
            frame,
            file,
            start_line,
            &line_id_start_0,
            area_src.union(area_split),
        );
        self.draw_status(frame, file, area_status);
        self.draw_scroll(frame, area_src, n);
    }
    fn draw_currect_pointer(
        &self,
        frame: &mut Frame,
        file: &dyn FileData,
        start_line: usize,
        line_id_start_0: &Option<usize>,
        area_currect_pointer: Rect,
    ) {
        if let Some(line_id_start_0) = line_id_start_0 {
            let [_, area_pointer, _] = Layout::vertical([
                Constraint::Length(*line_id_start_0 as u16),
                Constraint::Max(1_u16),
                Constraint::Fill(1),
            ])
            .areas(area_currect_pointer);
            let point_line = start_line.saturating_add(*line_id_start_0);
            let pointer_size = match file.get_read_done() {
                true => file
                    .get_lines_range(point_line, point_line + 1)
                    .0
                    .iter()
                    .nth(0)
                    .and_then(|s| (**s).chars().enumerate().find(|(_, c)| *c != ' '))
                    .map(|(id, _)| id.saturating_sub(self.horizontial_scroll)),
                _ => None,
            };

            if let Some(n) = pointer_size {
                let text_pointer = Line::from_iter(
                    std::iter::once(
                        Span::raw('├'.to_string()).style(Style::default().light_green()),
                    )
                    .chain(std::iter::repeat_n(
                        Span::raw('─'.to_string()).style(Style::default().light_green()),
                        n.saturating_sub(1),
                    ))
                    .chain(std::iter::once(
                        Span::raw('>'.to_string()).style(Style::default().light_green()),
                    )),
                );

                let paragraph_pointer = Paragraph::new(text_pointer);
                frame.render_widget(paragraph_pointer, area_pointer);
            }
        }
    }
    fn draw_src(
        &self,
        frame: &mut Frame,
        file: &dyn FileData,
        start_line: usize,
        end_line: usize,
        area_src: Rect,
    ) {
        let src = match (file.get_read_done(), file.get_highlight_done()) {
            (true, true) => file.get_highlight_lines_range(start_line, end_line).0,
            (false, true) => file
                .get_lines_range(start_line, end_line)
                .0
                .iter()
                .map(|s| vec![(ratatui::style::Color::White, s.to_string())])
                .collect(),
            _ => vec![],
        };
        let text_src = Text::from_iter(
            src.iter()
                .map(|s| Line::from_iter(s.iter().map(|(c, s)| Span::raw(s).fg(*c)))),
        );
        let paragraph_src = Paragraph::new(text_src).scroll((0, self.horizontial_scroll as u16));
        frame.render_widget(paragraph_src, area_src);
    }
    fn draw_status(&self, frame: &mut Frame, file: &dyn FileData, area_status: Rect) {
        let title = file.get_file_name();
        let scroll_x = title.len().saturating_sub(self.area.width as usize) as u16;
        let paragraph_status = Paragraph::new(title)
            .fg(Color::Black)
            .bg(Color::Gray)
            .scroll((0, scroll_x));
        frame.render_widget(paragraph_status, area_status);
    }
    fn draw_id(
        &self,
        frame: &mut Frame,
        start_line: usize,
        end_line: usize,
        line_id: usize,

        area_ids: Rect,
    ) {
        let ids: Vec<usize> = (start_line..end_line.saturating_add(1)).collect::<Vec<_>>();
        let text_ids = Text::from_iter(ids.iter().map(|s| {
            let line = Line::from_iter(s.to_string().chars().map(|c| Span::raw(c.to_string())));
            if *s == line_id {
                line.style(Style::default().light_green())
            } else {
                line
            }
        }));

        let paragraph_id = Paragraph::new(text_ids).right_aligned();
        frame.render_widget(paragraph_id, area_ids);
    }
    fn draw_breakpoint(
        &self,
        frame: &mut Frame,
        file: &dyn FileData,
        start_line: usize,
        end_line: usize,
        area_ids: Rect,
    ) {
        let bp = self.get_breakpoint_in_file_range(file, start_line, end_line);
        let ids: Vec<usize> = (start_line..end_line.saturating_add(1)).collect::<Vec<_>>();
        let text_ids = Text::from_iter(ids.iter().map(|s| {
            if let Some(enable) = bp.get(&(*s as u64)) {
                let line = Line::from_iter(s.to_string().chars().map(|c| Span::raw(c.to_string())));
                match enable {
                    true => line.style(Style::default().fg(Color::Rgb(255, 0, 0))),
                    false => line.style(Style::default().fg(Color::Rgb(255, 128, 0))), //orange
                }
            } else {
                Line::from("")
            }
        }));

        let paragraph_id = Paragraph::new(text_ids).right_aligned();
        frame.render_widget(paragraph_id, area_ids);
    }

    fn draw_split(&self, frame: &mut Frame, area_split: Rect) {
        let test_split = Text::from_iter(std::iter::repeat_n(
            Line::from("│ "),
            area_split.height as usize,
        ));
        let paragraph_split = Paragraph::new(test_split);
        frame.render_widget(paragraph_split, area_split);
    }
    fn draw_scroll(&self, frame: &mut Frame, area_src: Rect, text_len: usize) {
        let hight = area_src.height as usize;
        let up_half = hight.div_euclid(2);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);

        let mut state = ScrollbarState::new(text_len.saturating_sub(hight))
            .position(self.vertical_scroll.saturating_sub(up_half));
        frame.render_stateful_widget(scrollbar, area_src, &mut state);
    }
    fn set_vertical_to_stop_point(&mut self, file_name: &String) {
        match self.get_need_show_file() {
            Some((file, line_id)) => {
                if *file_name == file.get_file_name() {
                    self.vertical_scroll = line_id as usize;
                } else {
                    error!("file not same '{}' '{}'", file_name, file.get_file_name());
                }
            }
            _ => {
                error!("ReadAsmFunc set line fail");
            }
        };
    }
}

impl Component for Code {
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
                true => Ok(Some(action::Action::Code(Action::Up(3)))),
                false => Ok(None),
            },
            crossterm::event::MouseEventKind::ScrollDown => match is_in {
                true => Ok(Some(action::Action::Code(Action::Down(3)))),
                false => Ok(None),
            },
            _ => Ok(None),
        }
    }
    fn update(&mut self, action: action::Action) -> Result<Option<action::Action>> {
        let mut ret = None;
        match action {
            action::Action::Tick => {
                // add any logic here that should run on every tick
            }
            action::Action::Render => {
                // add any logic here that should run on every render
            }
            action::Action::Resize(x, y) => {
                self.set_area(&layout::Size::new(x, y));
            }
            action::Action::Code(Action::Up(p)) => {
                self.file_up(p);
            }
            action::Action::Code(Action::Down(p)) => {
                self.file_down(p);
            }
            action::Action::Code(Action::Left(p)) => {
                self.file_left(p);
            }
            action::Action::Code(Action::Right(p)) => {
                self.file_right(p);
            }
            action::Action::Gdbmi(gdbmi::Action::ShowFile((file, line_id))) => {
                self.file_need_show = FileNeedShow::SrcFile(FileNeedShowSrcFile {
                    name: file.clone(),
                    line: line_id,
                });
                match self.files_set.contains_key(&file) {
                    false => {
                        if let Some(send) = self.command_tx.clone() {
                            let file_data = SrcFileData::new(file.clone());
                            self.files_set.insert(file_data.get_key(), file_data);
                            let read_therad = Self::read_file(file.clone(), send.clone());
                            tokio::spawn(async {
                                read_therad.await;
                            });
                            debug!("read file {} start", file);
                        } else {
                            let msg = format!("read file {} thread not start", &file);
                            error!("{}", &msg);
                        }
                    }
                    true => {
                        debug!("file {} has read", &file);
                    }
                }
                self.set_vertical_to_stop_point(&file);
            }
            action::Action::Gdbmi(gdbmi::Action::Breakpoint(bkpt)) => {
                let val = BreakPointData::from(&bkpt);
                let key = val.get_key();
                self.breakpoint_set.remove(&key);
                self.breakpoint_set.insert(key, val);
            }
            action::Action::Gdbmi(gdbmi::Action::BreakpointDeleted(id)) => {
                self.breakpoint_set.remove(&Rc::new(id.to_string()));
            }
            action::Action::Code(Action::FileReadOneLine((file_name, line))) => {
                self.files_set.entry(file_name.into()).and_modify(|file| {
                    file.add_line(line);
                });
            }
            action::Action::Code(Action::FileReadEnd(file)) => {
                match self.files_set.remove_entry(&file) {
                    Some((name, mut file_data)) => {
                        if let Some(send) = self.command_tx.clone() {
                            file_data.set_read_done();
                            let lines = file_data.get_lines().clone();
                            self.files_set.insert(name, file_data);
                            let highlight_thread =
                                Self::highlight_file(file.clone(), lines, send.clone());
                            tokio::spawn(async {
                                highlight_thread.await;
                            });
                            debug!("highlight file {} start", file);
                        } else {
                            let msg = format!("read file {} thread not start", &file);
                            error!("{}", &msg);
                        }
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
                self.set_vertical_to_stop_point(&file);
            }
            action::Action::Code(Action::FilehighlightLine((file_name, line))) => {
                self.files_set.entry(file_name.into()).and_modify(|file| {
                    file.add_highlight_line(line);
                });
            }
            action::Action::Code(Action::FilehighlightEnd(file)) => {
                match self.files_set.remove_entry(&file) {
                    Some((name, mut file_data)) => {
                        file_data.set_highlight_done();
                        self.files_set.insert(name, file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
            }
            action::Action::Gdbmi(gdbmi::Action::ReadAsmFunc(func)) => {
                self.asm_func_set
                    .entry(func.func.clone().into())
                    .and_modify(|asm| {
                        asm.add_lines(&func);
                        asm.set_read_done();
                        asm.add_highlight_lines(&func);
                        asm.set_highlight_done();
                        ret = Some(action::Action::Code(Action::AsmFileEnd));
                    });
                self.set_vertical_to_stop_point(&func.func);
            }
            action::Action::Gdbmi(gdbmi::Action::ShowAsm((func, addr))) => {
                self.file_need_show = FileNeedShow::AsmFile(FileNeedShowAsmFunc {
                    name: func.clone(),
                    addr,
                });
                match self.asm_func_set.contains_key(&func) {
                    false => {
                        let file_data = AsmFuncData::new(func.clone());
                        self.asm_func_set.insert(file_data.get_key(), file_data);
                        debug!("asm file {} start", &func);
                        ret = Some(action::Action::Gdbmi(gdbmi::Action::DisassembleAsm(
                            func.clone(),
                        )));
                    }
                    true => {
                        debug!("asm {} has read", &func);
                    }
                }
                self.set_vertical_to_stop_point(&func);
            }
            _ => {}
        }
        Ok(ret)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let ans = self.get_need_show_file().map(|(file, _line_id)| {
            let n = file.get_lines_len();
            let num_len = n.to_string().len() as u16;
            let [area, area_status, _, _] = tool::get_layout(area);
            let [area_ids, area_split, area_src] = Layout::horizontal([
                Constraint::Min(num_len),
                Constraint::Min(2),
                Constraint::Percentage(100),
            ])
            .areas(area);
            (n, area_ids, area_split, area_src, area_status)
        });

        ans.map(|(n, _, _, area_src, _)| (area_src.height, n))
            .map(|(height, n)| {
                self.legalization_vertical_scroll_range(height as usize, n);
                Some(())
            });

        self.get_need_show_file()
            .map(|(file, _)| {
                if let Some((_, _, _, area_src, _)) = ans {
                    let (start_line, end_line) =
                        self.get_windows_show_file_range(area_src.height as usize);
                    let (src_text, _, _) = file.get_lines_range(start_line, end_line);
                    let text_len = src_text.iter().map(|s| s.len()).max().unwrap_or(0);
                    (area_src.width, text_len)
                } else {
                    (0, 0_usize)
                }
            })
            .map(|(width, text_len)| {
                self.legalization_horizontial_scroll_range(width as usize, text_len);
                Some(())
            });

        let ans = if let (
            Some((n, area_ids, area_split, area_src, area_status)),
            Some((file, line_id)),
        ) = (ans, self.get_need_show_file())
        {
            let (start_line, end_line) = self.get_windows_show_file_range(area_src.height as usize);
            let (_, start_line, end_line) = file.get_lines_range(start_line, end_line);
            Some((
                start_line,
                end_line,
                line_id,
                n,
                file,
                area_ids,
                area_split,
                area_src,
                area_status,
            ))
        } else {
            None
        };

        if let Some((
            start_line,
            end_line,
            line_id,
            n,
            file,
            area_ids,
            area_split,
            area_src,
            area_status,
        )) = ans
        {
            self.draw_all(
                frame,
                start_line,
                end_line,
                line_id as usize,
                n,
                file,
                area_ids,
                area_split,
                area_src,
                area_status,
            );
        };
        Ok(())
    }
}
