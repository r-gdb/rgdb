use crate::components::code::breakpoint::BreakPointData;
use crate::mi::disassemble::DisassembleFunction;
use crate::tool;
use crate::tool::{addr_to_u64, FileData, HashSelf, HighlightFileData, TextFileData};
use ratatui::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::error;

use super::breakpoint::BreakPointSignalData;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AsmFuncData {
    pub func_name: Rc<String>,
    pub addrs: Vec<(u64, u64)>,
    pub lines: Vec<String>,
    pub lines_highlight: Vec<Vec<(ratatui::style::Color, String)>>,
    pub read_done: bool,
    pub highlight_done: bool,
}

impl HashSelf<String> for AsmFuncData {
    fn get_key(&self) -> Rc<String> {
        self.func_name.clone()
    }
}

impl TextFileData for AsmFuncData {
    fn get_file_name(&self) -> String {
        self.func_name.as_ref().clone()
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
        breakpoints
            .iter()
            .flat_map(|bp| match bp {
                BreakPointData::Signal(BreakPointSignalData::Asm(bp)) => {
                    vec![(&bp.addr, bp.enabled)]
                }
                BreakPointData::Multiple(bpm) => bpm
                    .bps
                    .iter()
                    .filter_map(|bp| match bp {
                        BreakPointSignalData::Asm(bp) => {
                            Some((&bp.addr, (bp.enabled && bpm.enabled)))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .filter_map(|(addr, enable)| {
                let line = self.get_line_id(addr);
                match line {
                    Some(line) => match start_line <= line as usize && line as usize <= end_line {
                        true => Some((line, enable)),
                        false => None,
                    },
                    _ => None,
                }
            })
            .fold(HashMap::new(), |mut m, (line, enable)| {
                m.entry(line)
                    .and_modify(|enable_old| *enable_old |= enable)
                    .or_insert(enable);
                m
            })
    }
}

impl HighlightFileData for AsmFuncData {
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
impl AsmFuncData {
    pub fn new(func_name: String) -> Self {
        Self {
            func_name: Rc::new(func_name),
            lines: vec![],
            lines_highlight: vec![],
            addrs: vec![],
            read_done: false,
            highlight_done: false,
        }
    }
    pub fn add_lines(&mut self, lines: &DisassembleFunction) {
        let len = lines
            .insts
            .iter()
            .last()
            .map(|l| l.offset.to_string().len());
        if let Some(len) = len {
            self.lines.push(format!(
                "Dump of assembler code for function {}:\n",
                &lines.func
            ));
            lines.insts.iter().for_each(|line| {
                let space =
                    std::iter::repeat_n(" ", len.saturating_sub(line.offset.to_string().len()))
                        .collect::<String>();
                let line = format!(
                    "    {} <+{}>:{} {}\n",
                    line.address, line.offset, space, line.inst
                );
                self.lines.push(line);
            });
            self.lines.push("End of assembler dump.".to_string());

            self.create_addr_map(lines, 1_usize);
        }
    }
    pub fn add_highlight_lines(&mut self, _func: &DisassembleFunction) {
        let ext = "asm";
        let theme = tool::get_theme();
        let ps = tool::get_syntax_set(ext);
        let lines = self.get_lines();

        if let Some(syntax) = ps.find_syntax_by_extension(ext) {
            let mut h = syntect::easy::HighlightLines::new(syntax, &theme);
            self.lines_highlight = lines
                .iter()
                .enumerate()
                .map(|(id, line)| {
                    if id == 0 {
                        vec![
                            (
                                Color::White,
                                "Dump of assembler code for function ".to_string(),
                            ),
                            (Color::Blue, self.get_file_name()),
                            (Color::White, ":\n".to_string()),
                        ]
                    } else if id == lines.len().saturating_sub(1) {
                        vec![(Color::White, line.clone())]
                    } else {
                        match h.highlight_line(line, &ps) {
                            std::result::Result::Ok(ranges) => ranges
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
                                .collect(),
                            std::result::Result::Err(e) => {
                                error!(
                                    "file {} highlight fail {} {}",
                                    &self.get_file_name(),
                                    &line,
                                    e
                                );
                                vec![(Color::White, line.clone())]
                            }
                        }
                    }
                })
                .collect();
        } else {
            error!("file {} not have extension", &ext);
            self.lines_highlight = lines
                .iter()
                .map(|line| vec![(Color::White, line.clone())])
                .collect();
        }
    }
    pub fn get_line_id(&self, addr: &String) -> Option<u64> {
        match (addr.starts_with("0x"), addr.get(2..addr.len())) {
            (true, Some(addr_id_str)) => {
                u64::from_str_radix(addr_id_str, 16).map_or(None, |addr_id| {
                    match self
                        .addrs
                        .as_slice()
                        .binary_search_by_key(&addr_id, |&(a, _)| a)
                    {
                        std::result::Result::Ok(id) => self.addrs.get(id).map(|(_, b)| b).cloned(),
                        _ => {
                            error!(
                                "asm addr {}  not find in asm func {:?},",
                                &addr, &self.addrs
                            );
                            None
                        }
                    }
                })
            }
            _ => None,
        }
    }
    fn create_addr_map(&mut self, func: &DisassembleFunction, base_offset: usize) {
        self.addrs = func
            .insts
            .iter()
            .enumerate()
            .filter_map(|(id, line)| match addr_to_u64(&line.address) {
                Some(addr) => {
                    let id = id.saturating_add(1).saturating_add(base_offset);
                    Some((addr, id as u64))
                }
                _ => {
                    error!("asm addr {} not an hex address", &line.address);
                    None
                }
            })
            .collect();
        self.addrs.sort();
    }
}

impl crate::tool::StatusFileData for AsmFuncData {
    fn get_status(&self) -> String {
        self.addrs
            .first()
            .and_then(|(start, _)| self.addrs.last().map(|(end, _)| (*start, *end)))
            .map(|(start, end)| {
                format!(
                    "** Dump of assembler code for function {}: (0x{:x} - 0x{:x}) **",
                    self.func_name, start, end
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "** Dump of assembler code for function {}: **",
                    self.func_name
                )
            })
    }
}

impl FileData for AsmFuncData {}
