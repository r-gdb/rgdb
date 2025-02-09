use crate::mi::disassemble::DisassembleFunction;
use crate::tool::{FileData, HashSelf, HighlightFileData, TextFileData};
use ratatui::prelude::*;
use std::rc::Rc;
use tracing::error;

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
            self.create_addr_map(lines);
        }
    }
    pub fn add_highlight_lines(&mut self, _func: &DisassembleFunction) {
        let lines = self.get_lines();
        self.lines_highlight = lines
            .iter()
            .map(|line| vec![(Color::White, line.clone())])
            .collect();
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
    fn create_addr_map(&mut self, func: &DisassembleFunction) {
        self.addrs = func
            .insts
            .iter()
            .enumerate()
            .filter_map(|(id, line)| {
                match (
                    line.address.starts_with("0x"),
                    line.address.get(2..line.address.len()),
                ) {
                    (true, Some(addr)) => {
                        let id = id.saturating_add(1);
                        u64::from_str_radix(addr, 16).map_or(None, |addr| Some((addr, id as u64)))
                    }
                    _ => {
                        error!("asm addr {} not an hex address", &line.address);
                        None
                    }
                }
            })
            .collect();
        self.addrs.sort();
    }
}
impl FileData for AsmFuncData {}
