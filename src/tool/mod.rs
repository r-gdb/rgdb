use color_eyre::{eyre::Ok, Result};
use libc::ptsname;
use ratatui::layout::{Constraint, Layout, Rect};
use std::ffi::CStr;
use std::hash::Hash;
use std::rc::Rc;

pub fn get_pty_name(fd: i32) -> Result<String> {
    let name = unsafe { ptsname(fd) };
    let c_str = unsafe { CStr::from_ptr(name) }.to_str()?;
    Ok(c_str.to_string())
}

pub fn get_layout(area: Rect) -> [Rect; 4] {
    Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area)
}

pub trait HashSelf<T: Hash> {
    fn get_key(&self) -> Rc<T>;
}

pub trait TextFileData {
    fn get_file_name(&self) -> String;
    fn add_line(&mut self, line: String);
    fn get_read_done(&self) -> bool;
    fn set_read_done(&mut self);
    fn get_lines_len(&self) -> usize;
    fn get_lines_range(&self, start: usize, end: usize) -> (Vec<&String>, usize, usize);
}

pub trait HighlightFileData {
    fn add_highlight_line(&mut self, line: Vec<(ratatui::style::Color, String)>);
    fn get_highlight_done(&self) -> bool;
    fn set_highlight_done(&mut self);
    fn get_lines(&self) -> &Vec<String>;
    fn get_highlight_lines_range(
        &self,
        start: usize,
        end: usize,
    ) -> (Vec<Vec<(ratatui::style::Color, String)>>, usize, usize);
}

pub trait FileData: TextFileData + HighlightFileData + HashSelf<std::string::String> {}
