use super::action;
use super::breakpoint::*;
use crate::components::code;
use crate::mi::frame::Frame;
use crate::tool;
use crate::tool::{FileData, HighlightFileData, TextFileData};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use syntect::easy::HighlightLines;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SrcFileData {
    pub file_name: Rc<String>,
    lines: Vec<String>,
    lines_highlight: Vec<Vec<(ratatui::style::Color, String)>>,
    read_done: bool,
    highlight_done: bool,
}

impl TextFileData for SrcFileData {
    fn get_file_name(&self) -> String {
        self.file_name.as_ref().clone()
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
    fn get_lines(&self) -> &Vec<String> {
        self.lines.as_ref()
    }
    fn get_breakpoint_need_show_in_range(
        &self,
        breakpoints: Vec<&BreakPointData>,
        start_line: usize,
        end_line: usize,
    ) -> HashMap<u64, bool> {
        let file_name = self.get_file_name();
        breakpoints
            .iter()
            .flat_map(|bp| match bp {
                BreakPointData::Signal(bp) => match &bp.src {
                    Some(src) => {
                        vec![(&src.fullname, src.line, bp.enabled)]
                    }
                    None => vec![],
                },
                BreakPointData::Multiple(bpm) => bpm
                    .bps
                    .iter()
                    .filter_map(|bp| match &bp.src {
                        Some(src) => Some((&src.fullname, src.line, (bp.enabled && bpm.enabled))),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            })
            .filter(|(name, line, _)| {
                **name == file_name && start_line <= *line as usize && *line as usize <= end_line
            })
            .map(|(_, line, enable)| (line, enable))
            .fold(HashMap::new(), |mut m, (line, enable)| {
                m.entry(line)
                    .and_modify(|enable_old| *enable_old |= enable)
                    .or_insert(enable);
                m
            })
    }
}

impl HighlightFileData for SrcFileData {
    fn get_highlight_done(&self) -> bool {
        self.highlight_done
    }
    fn set_highlight_done(&mut self) {
        self.highlight_done = true;
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
    pub fn add_line(&mut self, line: String) {
        self.lines.push(line);
    }
    pub fn add_highlight_line(&mut self, line: Vec<(ratatui::style::Color, String)>) {
        self.lines_highlight.push(line);
    }

    pub async fn highlight_file(
        file_name: String,
        lines: Vec<String>,
        send: UnboundedSender<action::Action>,
    ) {
        let theme = tool::get_theme();
        let ext = Path::new(&file_name).extension().and_then(OsStr::to_str);
        if let Some(ext) = ext {
            let ps = tool::get_syntax_set(ext);
            if let Some(syntax) = ps.find_syntax_by_extension(ext) {
                let mut h = HighlightLines::new(syntax, &theme);
                
                // 每处理几行就主动交出控制权
                const YIELD_INTERVAL: usize = 200;
                for (i, s) in lines.iter().enumerate() {
                    // 每处理YIELD_INTERVAL行就交出控制权
                    if i % YIELD_INTERVAL == 0 && i > 0 {
                        tokio::task::yield_now().await;
                    }
                    
                    match h.highlight_line(s, &ps) {
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
                            tool::send_action(
                                &send,
                                action::Action::Code(code::Action::FilehighlightLine((
                                    file_name.clone(),
                                    e,
                                ))),
                            );
                        }
                        std::result::Result::Err(e) => {
                            error!("file {} highlight fail {} {}", &file_name, &s, e);
                        }
                    }
                }
                tool::send_action(
                    &send,
                    action::Action::Code(code::Action::FilehighlightEnd(file_name)),
                );
            } else {
                error!("file {} not have extension", &ext);
            }
        } else {
            error!("file {} not have extension", &file_name);
        }
    }
    pub fn read_file_filter(line: String) -> String {
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

    pub async fn read_file(file: String, frame: Frame, send: UnboundedSender<action::Action>) {
        match File::open(&file).await {
            std::result::Result::Ok(f) => {
                let mut f = tokio::io::BufReader::new(f);
                let mut line_count = 0;
                const YIELD_INTERVAL: usize = 200; // 每读取200行让出一次控制权
                
                loop {
                    let mut line = String::new();
                    match f.read_line(&mut line).await {
                        std::result::Result::Ok(0) => {
                            tool::send_action(
                                &send,
                                action::Action::Code(code::Action::FileReadEnd(file)),
                            );
                            break;
                        }
                        std::result::Result::Ok(_n) => {
                            line = SrcFileData::read_file_filter(line);
                            tool::send_action(
                                &send,
                                action::Action::Code(code::Action::FileReadOneLine((
                                    file.clone(),
                                    line,
                                ))),
                            );
                            
                            // 每读取YIELD_INTERVAL行就让出控制权
                            line_count += 1;
                            if line_count % YIELD_INTERVAL == 0 {
                                tokio::task::yield_now().await;
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
                tool::send_action(
                    &send,
                    action::Action::Code(code::Action::FileReadFail((file.clone(), frame))),
                );
            }
        }
    }
}

impl crate::tool::HashSelf<String> for SrcFileData {
    fn get_key(&self) -> Rc<String> {
        self.file_name.clone()
    }
}

impl crate::tool::StatusFileData for SrcFileData {}

impl FileData for SrcFileData {}
