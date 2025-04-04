use super::Component;
use crate::components::code::asmfuncdata::AsmFuncData;
use crate::components::code::breakpoint::BreakPointData;
use crate::components::code::srcfiledata::SrcFileData;
use crate::components::gdbmi;
use crate::mi::frame::Frame as FrameMi;
use crate::tool;
use crate::tool::{FileData, HashSelf, HighlightFileData, TextFileData};
use crate::{action, config::Config};
use arboard::Clipboard;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::{eyre::Ok, Result};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use strum::Display;
use symbols::scrollbar;

use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

mod asmfuncdata;
pub mod breakpoint;
mod srcfiledata;
mod test;

#[derive(Default)]
pub struct Code {
    command_tx: Option<UnboundedSender<action::Action>>,
    config: Config,

    files_set: HashMap<Rc<String>, SrcFileData>,
    read_fail_files_set: HashSet<String>,
    asm_func_set: HashMap<Rc<String>, AsmFuncData>,
    breakpoint_set: HashMap<Rc<String>, BreakPointData>,
    file_need_show: FileNeedShow,
    vertical_scroll: usize,
    horizontial_scroll: usize,
    area: Rect,
    is_horizontal: bool,
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

pub enum FileDataReal<'a> {
    None,
    SrcFile((&'a SrcFileData, u64)),
    AsmFile((&'a AsmFuncData, u64)),
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    FileReadOneLine((String, String)),
    FileReadEnd(String),
    FileReadFail((String, FrameMi)),
    AsmFileEnd,
    FilehighlightLine((String, Vec<(ratatui::style::Color, String)>)),
    FilehighlightEnd(String),
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
}

#[derive(Default)]
struct LineInfo {
    start_line: usize,
    end_line: usize,
    line_id: usize,
    n: usize,
}

#[derive(Default)]
struct Areas {
    ids: Rect,
    split: Rect,
    src: Rect,
    status: Rect,
}

impl Code {
    pub fn new() -> Self {
        Self::default()
    }
    fn get_file_need_show(&self) -> Option<(&dyn FileData, u64)> {
        match self.get_file_need_show_return_file() {
            FileDataReal::None => None,
            FileDataReal::SrcFile((file, id)) => Some((file, id)),
            FileDataReal::AsmFile((func, id)) => Some((func, id)),
        }
    }
    fn get_file_need_show_return_file(&self) -> FileDataReal {
        match &self.file_need_show {
            FileNeedShow::None => FileDataReal::None,
            FileNeedShow::SrcFile(file) => match self.files_set.get(&file.name) {
                Some(file_data) => {
                    if file_data.get_read_done() {
                        FileDataReal::SrcFile((file_data, file.line))
                    } else {
                        info!("file {} not read done", &file.name);
                        FileDataReal::None
                    }
                }
                _ => {
                    error!("file {} not found", &file.name);
                    FileDataReal::None
                }
            },
            FileNeedShow::AsmFile(func) => {
                let name = &func.name;
                match self.asm_func_set.get(&Rc::new(name.clone())) {
                    Some(asm_file) => match asm_file.get_read_done() {
                        true => match asm_file.get_line_id(&func.addr) {
                            Some(id) => FileDataReal::AsmFile((asm_file, id)),
                            _ => {
                                error!(
                                    "asm file {} not find {:?}, in {:?}",
                                    name, &func.addr, &asm_file
                                );
                                FileDataReal::None
                            }
                        },
                        _ => {
                            info!("asm file {} not read done", name);
                            FileDataReal::None
                        }
                    },
                    _ => {
                        error!("asm {} not found", &name);
                        FileDataReal::None
                    }
                }
            }
        }
    }
    fn get_breakpoints(&self) -> Vec<&BreakPointData> {
        self.breakpoint_set.values().collect()
    }
    fn set_area(&mut self, area: &layout::Size) {
        let area = Rect::new(0, 0, area.width, area.height);
        tool::Layouts { src: self.area, .. } = (area, self.is_horizontal).into();
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

    fn draw_all(&self, frame: &mut Frame, file: &dyn FileData, line_info: LineInfo, areas: Areas) {
        let line_id_start_0 = if line_info.start_line <= line_info.line_id {
            Some(line_info.line_id.saturating_sub(line_info.start_line))
        } else {
            None
        };
        self.draw_src(
            frame,
            file,
            line_info.start_line,
            line_info.end_line,
            areas.src,
        );
        self.draw_id(
            frame,
            line_info.start_line,
            line_info.end_line,
            line_info.line_id,
            areas.ids,
        );
        self.draw_breakpoint(
            frame,
            file,
            line_info.start_line,
            line_info.end_line,
            areas.ids,
        );
        self.draw_split(frame, areas.split);
        self.draw_currect_pointer(
            frame,
            file,
            line_info.start_line,
            &line_id_start_0,
            areas.src.union(areas.split),
        );
        self.draw_status(frame, file, areas.status);
        self.draw_scroll(frame, areas.src, line_info.n);
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
                    .get(0)
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
            (true, false) => file
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
        let title = file.get_status();
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
        let bp =
            file.get_breakpoint_need_show_in_range(self.get_breakpoints(), start_line, end_line);
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
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .style(Style::default().fg(Color::White));

        let mut state = ScrollbarState::new(text_len.saturating_sub(hight))
            .position(self.vertical_scroll.saturating_sub(up_half));
        frame.render_stateful_widget(scrollbar, area_src, &mut state);
    }
    fn set_vertical_to_stop_point(&mut self, file_name: &String) {
        match self.get_file_need_show() {
            Some((file, line_id)) => {
                if *file_name == file.get_file_name() {
                    self.vertical_scroll = line_id as usize;
                } else {
                    error!("file not same '{}' '{}'", file_name, file.get_file_name());
                }
            }
            _ => {
                info!("{} read not finish set show line fail", &file_name);
            }
        };
    }
    fn show_file(&mut self, file: String, line_id: u64, frame: FrameMi) -> Option<action::Action> {
        let mut ret = None;
        match self.read_fail_files_set.contains(&file) {
            true => match &frame.func {
                Some(func) => {
                    ret = Some(action::Action::Gdbmi(gdbmi::Action::ShowAsm((
                        func.clone(),
                        frame.addr.clone(),
                        frame,
                    ))));
                }
                _ => {}
            },
            false => {
                self.file_need_show = FileNeedShow::SrcFile(FileNeedShowSrcFile {
                    name: file.clone(),
                    line: line_id,
                });
                match self.files_set.contains_key(&file) {
                    false => {
                        if let Some(send) = self.command_tx.clone() {
                            let file_data = SrcFileData::new(file.clone());
                            self.files_set.insert(file_data.get_key(), file_data);
                            let read_therad =
                                SrcFileData::read_file(file.clone(), frame.clone(), send.clone());
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
        }
        ret
    }
}

impl Component for Code {
    fn init(&mut self, area: Size) -> Result<()> {
        // let mut clipboard = Clipboard::new()?;
        // clipboard.set_text("Hello, clipboard!")?;
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
            action::Action::SwapHV => {
                self.is_horizontal = !self.is_horizontal;
            }
            action::Action::Gdbmi(gdbmi::Action::ShowFile((file, line_id, frame))) => {
                ret = self.show_file(file, line_id, frame);
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
                                SrcFileData::highlight_file(file.clone(), lines, send.clone());
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
            action::Action::Code(Action::FileReadFail((file, frame))) => {
                self.files_set.remove(&file);
                self.read_fail_files_set.insert(file);
                self.file_need_show = FileNeedShow::None;
                match &frame.func {
                    Some(func) => {
                        ret = Some(action::Action::Gdbmi(gdbmi::Action::ShowAsm((
                            func.clone(),
                            frame.addr.clone(),
                            frame,
                        ))));
                    }
                    _ => {}
                }
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
                debug!("asm_func_set{:?}", &self.asm_func_set.keys());
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
            action::Action::Gdbmi(gdbmi::Action::ShowAsm((func, addr, _))) => {
                self.file_need_show = FileNeedShow::AsmFile(FileNeedShowAsmFunc {
                    name: func.clone(),
                    addr: addr.clone(),
                });
                match self.asm_func_set.contains_key(&func) {
                    false => {
                        let file_data = AsmFuncData::new(func.clone());
                        self.asm_func_set.insert(file_data.get_key(), file_data);
                        debug!("asm file {} start", &func);
                        ret = Some(action::Action::Gdbmi(gdbmi::Action::DisassembleAsm(addr)));
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

    /// 绘制代码视图的主要函数
    ///
    /// # 参数
    /// * `frame` - 用于绘制UI的Frame
    /// * `area` - 绘制区域的矩形范围
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // 获取需要显示的文件信息和布局信息
        let layout_info = self.get_file_need_show().and_then(|(file, _line_id)| {
            // 获取文件行数和行号宽度
            let total_lines = file.get_lines_len();
            let line_num_width = total_lines.to_string().len() as u16;

            // 获取布局区域
            let tool::Layouts {
                src: main_area,
                src_status: status_area,
                ..
            } = tool::Layouts::from((area, self.is_horizontal));

            // 划分主区域为行号、分隔符和代码内容三部分
            let [line_nums_area, separator_area, code_area] = Layout::horizontal([
                Constraint::Min(line_num_width),
                Constraint::Min(2),
                Constraint::Percentage(100),
            ])
            .areas(main_area);

            Some((
                total_lines,
                line_nums_area,
                separator_area,
                code_area,
                status_area,
            ))
        });

        // 根据布局信息调整垂直滚动范围
        if let Some((total_lines, _, _, code_area, _)) = layout_info {
            let visible_height = code_area.height as usize;
            self.legalization_vertical_scroll_range(visible_height, total_lines);
        }

        // 调整水平滚动范围
        self.get_file_need_show()
            .map(|(file, _)| {
                if let Some((_, _, _, area_src, _)) = layout_info {
                    // 获取当前显示范围内的文本
                    let (start_line, end_line) =
                        self.get_windows_show_file_range(area_src.height as usize);
                    let (src_text, _, _) = file.get_lines_range(start_line, end_line);
                    // 计算最长行的长度
                    let text_len = src_text.iter().map(|s| s.len()).max().unwrap_or(0);
                    (area_src.width, text_len)
                } else {
                    (0, 0_usize)
                }
            })
            .map(|(width, text_len)| {
                self.legalization_horizontial_scroll_range(width as usize, text_len);
            });

        // 准备绘制所需的所有信息
        let draw_info = match layout_info {
            Some((n, area_ids, area_split, area_src, area_status)) => {
                if let Some((file, line_id)) = self.get_file_need_show() {
                    let (start_line, end_line) =
                        self.get_windows_show_file_range(area_src.height as usize);
                    let (_, start_line, end_line) = file.get_lines_range(start_line, end_line);
                    Some((
                        file,
                        LineInfo {
                            start_line,
                            end_line,
                            line_id: line_id as usize,
                            n,
                        },
                        Areas {
                            ids: area_ids,
                            split: area_split,
                            src: area_src,
                            status: area_status,
                        },
                    ))
                } else {
                    None
                }
            }
            None => None,
        };

        // 执行实际的绘制操作
        if let Some((file, line_info, areas)) = draw_info {
            self.draw_all(frame, file, line_info, areas);
        }
        Ok(())
    }
}
