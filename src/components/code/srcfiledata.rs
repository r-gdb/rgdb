use crate::tool::{FileData, HighlightFileData, TextFileData};
use std::rc::Rc;

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
}

impl crate::tool::HashSelf<String> for SrcFileData {
    fn get_key(&self) -> Rc<String> {
        self.file_name.clone()
    }
}

impl FileData for SrcFileData {}
