use super::Component;
use crate::components::gdbmi;
use crate::mi::breakpointmi::{BreakPointAction, BreakPointMultipleAction, BreakPointSignalAction};
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

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    files_set: HashMap<Rc<String>, Box<dyn FileData>>,
    breakpoint_set: HashMap<Rc<String>, BreakPointData>,
    file_need_show: Option<(String, u64)>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontial_scroll: usize,
    area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    FileReadOneLine((String, String)),
    FileReadEnd(String),
    FilehighlightLine((String, Vec<(ratatui::style::Color, String)>)),
    FilehighlightEnd(String),
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SrcFileData {
    pub file_name: Rc<String>,
    lines: Vec<String>,
    lines_highlight: Vec<Vec<(ratatui::style::Color, String)>>,
    read_done: bool,
    highlight_done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointMultipleData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub bps: Vec<BreakPointSignalData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointSignalData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub fullname: String,
    pub line: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPointData {
    Signal(BreakPointSignalData),
    Multiple(BreakPointMultipleData),
}

impl From<&BreakPointSignalAction> for BreakPointSignalData {
    fn from(a: &BreakPointSignalAction) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            fullname: a.fullname.clone(),
            line: a.line,
        }
    }
}

impl From<&BreakPointMultipleAction> for BreakPointMultipleData {
    fn from(a: &BreakPointMultipleAction) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            bps: a.bps.iter().map(BreakPointSignalData::from).collect(),
        }
    }
}

impl From<&BreakPointAction> for BreakPointData {
    fn from(a: &BreakPointAction) -> Self {
        match a {
            BreakPointAction::Signal(p) => Self::Signal(BreakPointSignalData::from(p)),
            BreakPointAction::Multiple(p) => Self::Multiple(BreakPointMultipleData::from(p)),
        }
    }
}

impl crate::tool::HashSelf<String> for BreakPointData {
    fn get_key(&self) -> Rc<String> {
        match self {
            Self::Signal(p) => p.number.clone(),
            Self::Multiple(p) => p.number.clone(),
        }
    }
}

impl TextFileData for SrcFileData {
    fn get_file_name(&self) -> String {
        self.file_name.as_ref().clone()
    }
    fn add_line(&mut self, line: String) {
        self.lines.push(line);
    }
    fn get_read_done(&self) -> bool {
        self.read_done
    }
    fn set_read_done(&mut self) {
        self.read_done = true;
    }
    fn get_lines_len(&self) -> usize {
        self.lines.len()
    }
    fn get_lines_range(&self, start: usize, end: usize) -> (Vec<&String>, usize, usize) {
        let n = self.lines.len().saturating_add(1);
        let end = n.min(end);
        (
            self.lines
                .iter()
                .skip(start.saturating_sub(1))
                .take(end.saturating_sub(start))
                .collect(),
            start,
            end,
        )
    }
}

impl HighlightFileData for SrcFileData {
    fn add_highlight_line(&mut self, line: Vec<(ratatui::style::Color, String)>) {
        self.lines_highlight.push(line);
    }
    fn get_highlight_done(&self) -> bool {
        self.highlight_done
    }
    fn set_highlight_done(&mut self) {
        self.highlight_done = true;
    }
    fn get_lines(&self) -> &Vec<String> {
        self.lines.as_ref()
    }
    fn get_highlight_lines_range(
        &self,
        start: usize,
        end: usize,
    ) -> (Vec<Vec<(ratatui::style::Color, String)>>, usize, usize) {
        let n = self.lines_highlight.len().saturating_add(1);
        let end = n.min(end);
        (
            self.lines_highlight
                .iter()
                .skip(start.saturating_sub(1))
                .take(end.saturating_sub(start))
                .cloned()
                .collect::<Vec<Vec<_>>>(),
            start,
            end,
        )
    }
}

impl SrcFileData {
    pub fn new(file_name: String) -> Self {
        Self {
            file_name: Rc::new(file_name),
            lines: vec![],
            lines_highlight: vec![],
            read_done: false,
            highlight_done: false,
        }
    }
}

impl crate::tool::HashSelf<String> for SrcFileData {
    fn get_key(&self) -> Rc<String> {
        self.file_name.clone()
    }
}

impl FileData for SrcFileData {}
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
    fn get_need_show_file(&self) -> Option<(&Box<dyn FileData>, u64)> {
        match self.file_need_show {
            Some((ref file, line_id)) => match self.files_set.get(&Rc::new(file.clone())) {
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
            },
            _ => None,
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
        file_name: &String,
        start_line: usize,
        end_line: usize,
    ) -> HashMap<u64, bool> {
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
                name == file_name && start_line <= *line as usize && *line as usize <= end_line
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
        &mut self,
        frame: &mut Frame,
        start_line: usize,
        end_line: usize,
        line_id: usize,
        n: usize,
        file_name: String,
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
        self.draw_src(frame, &file_name, start_line, end_line, area_src);
        self.draw_breakpoint(frame, &file_name, start_line, end_line, area_ids);
        self.draw_id(frame, start_line, end_line, line_id, area_ids);
        self.draw_split(frame, area_split);
        self.draw_currect_pointer(
            frame,
            &file_name,
            start_line,
            &line_id_start_0,
            area_src.union(area_split),
        );
        self.draw_status(frame, file_name, area_status);
        self.draw_scroll(frame, area_src, n);
    }
    fn draw_currect_pointer(
        &mut self,
        frame: &mut Frame,
        file_name: &String,
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
            let pointer_size = self
                .files_set
                .get(&Rc::new(file_name.clone()))
                .and_then(|file| match file.get_read_done() {
                    true => file
                        .get_lines_range(point_line, point_line + 1)
                        .0
                        .iter()
                        .nth(0)
                        .and_then(|s| (**s).chars().enumerate().find(|(_, c)| *c != ' '))
                        .map(|(id, _)| id.saturating_sub(self.horizontial_scroll)),
                    _ => None,
                });

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
        &mut self,
        frame: &mut Frame,
        file_name: &String,
        start_line: usize,
        end_line: usize,
        area_src: Rect,
    ) {
        // let empty_vec: (Vec<Vec<(_, _)>>, Vec<_>) = (vec![], vec![]);
        let src = match self.files_set.get(&Rc::new(file_name.clone())) {
            Some(file) => match (file.get_read_done(), file.get_highlight_done()) {
                (true, true) => file.get_highlight_lines_range(start_line, end_line).0,
                (false, true) => file
                    .get_lines_range(start_line, end_line)
                    .0
                    .iter()
                    .map(|s| vec![(ratatui::style::Color::White, s.to_string())])
                    .collect(),
                _ => vec![],
            },
            None => vec![],
        };
        let text_src = Text::from_iter(
            src.iter()
                .map(|s| Line::from_iter(s.iter().map(|(c, s)| Span::raw(s).fg(*c)))),
        );
        let paragraph_src = Paragraph::new(text_src).scroll((0, self.horizontial_scroll as u16));
        frame.render_widget(paragraph_src, area_src);
    }
    fn draw_status(&mut self, frame: &mut Frame, file_name: String, area_status: Rect) {
        let title = file_name;
        let scroll_x = title.len().saturating_sub(self.area.width as usize) as u16;
        let paragraph_status = Paragraph::new(title)
            .fg(Color::Black)
            .bg(Color::Gray)
            .scroll((0, scroll_x));
        frame.render_widget(paragraph_status, area_status);
    }
    fn draw_id(
        &mut self,
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
        &mut self,
        frame: &mut Frame,
        file_name: &String,
        start_line: usize,
        end_line: usize,
        area_ids: Rect,
    ) {
        let bp = self.get_breakpoint_in_file_range(file_name, start_line, end_line);
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

    fn draw_split(&mut self, frame: &mut Frame, area_split: Rect) {
        let test_split = Text::from_iter(std::iter::repeat_n(
            Line::from("│ "),
            area_split.height as usize,
        ));
        let paragraph_split = Paragraph::new(test_split);
        frame.render_widget(paragraph_split, area_split);
    }
    fn draw_scroll(&mut self, frame: &mut Frame, area_src: Rect, text_len: usize) {
        let hight = area_src.height as usize;
        let up_half = hight.div_euclid(2);
        let scrollbar =
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(text_len.saturating_sub(hight))
            .position(self.vertical_scroll.saturating_sub(up_half));
        frame.render_stateful_widget(scrollbar, area_src, &mut self.vertical_scroll_state);
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
                self.file_need_show = Some((file.clone(), line_id));
                self.vertical_scroll = line_id as usize;
                match self.files_set.contains_key(&file) {
                    false => {
                        if let Some(send) = self.command_tx.clone() {
                            let file_data = Box::new(SrcFileData::new(file.clone()));
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
            action::Action::Code(Action::FileReadOneLine((file, line))) => {
                match self.files_set.remove_entry(&file) {
                    Some((name, mut file_data)) => {
                        file_data.add_line(line);
                        self.files_set.insert(name, file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
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
            }
            action::Action::Code(Action::FilehighlightLine((file, line))) => {
                match self.files_set.remove_entry(&file) {
                    Some((name, mut file_data)) => {
                        file_data.add_highlight_line(line);
                        self.files_set.insert(name, file_data);
                    }
                    _ => {
                        error!("file {} not found", &file);
                    }
                }
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
            _ => {}
        }
        Ok(None)
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
                file.get_file_name(),
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
            file_name,
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
                file_name,
                area_ids,
                area_split,
                area_src,
                area_status,
            );
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::components::code::BreakPointData;
    use crate::components::code::Code;
    use crate::components::code::SrcFileData;
    use crate::mi::breakpointmi::{BreakPointAction, BreakPointSignalAction};
    use crate::tool::HashSelf;
    use crate::tool::TextFileData;
    use std::collections::HashMap;
    #[test]
    fn test_crtl_ascii_00_0f() {
        let line = "\u{0}\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}\u{7}\u{8}\u{b}\u{c}\r\u{e}\u{f}";
        let line = Code::read_file_filter(line.to_string());
        println!("{:?}", line);
        assert!(
            line == r##"\{NUL}\{SOH}\{STX}\{ETX}\{EOT}\{ENQ}\{ACK}\{BEL}\{BS}\{VT}\{FF}\{SO}\{SI}"##
        );
    }
    #[test]

    fn test_crtl_ascii_10_1f() {
        let line = "\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}\u{1a}\u{1b}\u{1c}\u{1d}\u{1e}\u{1f}\u{7f}";
        let line = Code::read_file_filter(line.to_string());
        assert!(
            line == r##"\{DLE}\{DC1}\{DC2}\{DC3}\{DC4}\{NAK}\{SYN}\{ETB}\{CAN}\{EM}\{SUB}\{ESC}\{FS}\{GS}\{RS}\{US}\{DEL}"##
        );
    }
    #[test]
    fn test_crtl_ascii_7f() {
        let line = "\u{7f}";
        let line = Code::read_file_filter(line.to_string());
        assert!(line == r##"\{DEL}"##);
    }
    #[test]
    fn test_crtl_ascii_tab() {
        let line = "\t";
        let line = Code::read_file_filter(line.to_string());
        assert!(line == "    ");
    }
    #[test]
    fn test_scroll_range() {
        let mut code = Code::default();
        let a = code.legalization_vertical_scroll_range(32, 64);
        println! {"let {:?}",a};
        assert!(a == (17_usize, 49_usize));
    }

    #[test]
    fn test_scroll_range_2() {
        let mut code = Code::new();
        let a = code.legalization_vertical_scroll_range(31, 64);
        println! {"let {:?}",a};
        assert!(a == (16_usize, 49_usize));
    }

    #[test]
    fn test_scroll_range_3() {
        let mut code = Code::new();
        let a = code.legalization_vertical_scroll_range(31, 2);
        println! {"let {:?}",a};
        assert!(a == (16_usize, 16_usize));
    }

    #[test]
    fn test_show_file_range() {
        let mut code = Code::new();
        code.vertical_scroll = 0;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (1_usize, 33_usize));
    }

    #[test]
    fn test_show_file_range_2() {
        let mut code = Code::new();
        code.vertical_scroll = 200;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (33_usize, 65_usize));
    }

    #[test]
    fn test_show_file_range_3() {
        let mut code = Code::new();
        code.vertical_scroll = 20;
        code.legalization_vertical_scroll_range(32, 64);
        let a = code.get_windows_show_file_range(32);
        println! {"let {:?}",a};
        assert!(a == (4_usize, 36_usize));
    }

    #[test]
    fn test_show_file_range_4() {
        let mut code = Code::new();
        code.vertical_scroll = 20;
        code.legalization_vertical_scroll_range(31, 64);
        let a = code.get_windows_show_file_range(31);
        println! {"let {:?}",a};
        assert!(a == (5_usize, 36_usize));
    }

    #[test]
    fn test_file_range_1() {
        let mut file = SrcFileData::new("a".to_string());
        (1..62).for_each(|i| {
            file.add_line(format!("{:?}\n", i));
        });
        file.set_read_done();
        let (src, s, e) = file.get_lines_range(4_usize, 36_usize);
        assert!(s == 4_usize);
        assert!(e == 36_usize);
        println!("file range{:?} {} {}", src, s, e);
        (4..37).zip(src.iter()).for_each(|(i, s)| {
            assert!(format!("{:?}\n", i) == **s);
        });
    }

    #[test]
    fn test_file_range_2() {
        let mut file = SrcFileData::new("a".to_string());
        (1..62).for_each(|i| {
            file.add_line(format!("{:?}\n", i));
        });
        file.set_read_done();
        let (src, s, e) = file.get_lines_range(50_usize, 65_usize);
        println!("file range{:?} {} {}", src, s, e);
        assert!(s == 50_usize);
        assert!(e == 62_usize);
        (50..62).zip(src.iter()).for_each(|(i, s)| {
            assert!(format!("{:?}\n", i) == **s);
        });
    }

    #[test]
    fn f_breakpoint_range() {
        use crate::mi::breakpointmi::BreakPointMultipleAction;
        let a = BreakPointAction::Multiple(BreakPointMultipleAction {
            number: "5".to_string(),
            enabled: false,
            bps: vec![
                BreakPointSignalAction {
                    number: "5.1".to_string(),
                    enabled: true,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                },
                BreakPointSignalAction {
                    number: "5.1".to_string(),
                    enabled: false,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                },
            ],
        });
        let a = BreakPointData::from(&a);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        let ans = code.get_breakpoint_in_file_range(
            &"/home/shizhilvren/tmux/environ.c".to_string(),
            22,
            39,
        );
        assert!(ans == HashMap::from([(34_u64, false)]));
    }

    #[test]
    fn f_breakpoint_range_2() {
        use crate::mi::breakpointmi::BreakPointMultipleAction;
        let a = BreakPointAction::Multiple(BreakPointMultipleAction {
            number: "5".to_string(),
            enabled: true,
            bps: vec![
                BreakPointSignalAction {
                    number: "5.1".to_string(),
                    enabled: true,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                },
                BreakPointSignalAction {
                    number: "5.1".to_string(),
                    enabled: false,
                    line: 34_u64,
                    fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                },
            ],
        });

        let a = BreakPointData::from(&a);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        let ans = code.get_breakpoint_in_file_range(
            &"/home/shizhilvren/tmux/environ.c".to_string(),
            22,
            39,
        );
        assert!(ans == HashMap::from([(34_u64, true)]));
    }

    #[test]
    fn f_breakpoint_range_3() {
        let a = BreakPointAction::Signal(BreakPointSignalAction {
            number: "2".to_string(),
            enabled: true,
            line: 34_u64,
            fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
        });
        let b = BreakPointAction::Signal(BreakPointSignalAction {
            number: "6".to_string(),
            enabled: true,
            line: 37_u64,
            fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
        });
        let a = BreakPointData::from(&a);
        let b = BreakPointData::from(&b);
        let mut code = Code::new();
        code.breakpoint_set.insert(a.get_key(), a);
        code.breakpoint_set.insert(b.get_key(), b);
        let ans = code.get_breakpoint_in_file_range(
            &"/home/shizhilvren/tmux/environ.c".to_string(),
            22,
            36,
        );
        assert!(ans == HashMap::from([(34_u64, true)]));
    }
}
