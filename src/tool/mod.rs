use crate::components::code::breakpoint::BreakPointData;
use color_eyre::{eyre::Ok, Result};
use libc::ptsname;
use ratatui::layout::{Constraint, Layout, Rect};
use std::collections::HashMap;
use std::ffi::CStr;
use std::hash::Hash;
use std::rc::Rc;
use tracing::error;
const NORD_THEME: &str = include_str!("../themes/Nord.tmTheme");
const ASSEMBLY_X86_64: &str = include_str!("../syntaxes/assembly_x86_64.sublime-syntax");

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
    fn get_read_done(&self) -> bool;
    fn set_read_done(&mut self);
    fn get_lines_len(&self) -> usize;
    fn get_lines_range(&self, start: usize, end: usize) -> (Vec<&String>, usize, usize);
    fn get_lines(&self) -> &Vec<String>;
    fn get_breakpoint_need_show_in_range(
        &self,
        breakpoints: Vec<&BreakPointData>,
        start_line: usize,
        end_line: usize,
    ) -> HashMap<u64, bool>;
}

pub trait HighlightFileData {
    fn get_highlight_done(&self) -> bool;
    fn set_highlight_done(&mut self);
    fn get_highlight_lines_range(
        &self,
        start: usize,
        end: usize,
    ) -> (Vec<Vec<(ratatui::style::Color, String)>>, usize, usize);
}
pub trait FileData: TextFileData + HighlightFileData + HashSelf<std::string::String> {}

pub fn addr_to_u64(value: &str) -> Option<u64> {
    match (value.starts_with("0x"), value.get(2..value.len())) {
        (true, Some(addr)) => u64::from_str_radix(addr, 16).ok(),
        _ => None,
    }
}

pub fn get_theme() -> syntect::highlighting::Theme {
    let mut nord_theme = std::io::Cursor::new(NORD_THEME.as_bytes());
    match syntect::highlighting::ThemeSet::load_from_reader(&mut nord_theme) {
        std::result::Result::Ok(theme) => theme,
        std::result::Result::Err(_) => syntect::highlighting::Theme::default(),
    }
}

pub fn get_syntax_set(ext: &str) -> syntect::parsing::SyntaxSet {
    let syntax_set = match ext {
        "asm" => {
            let mut builder = syntect::parsing::SyntaxSetBuilder::new();

            match syntect::parsing::syntax_definition::SyntaxDefinition::load_from_str(
                ASSEMBLY_X86_64,
                true,
                None,
            ) {
                std::result::Result::Ok(a) => {
                    builder.add(a);
                }
                std::result::Result::Err(_) => {
                    error!("Failed to load syntaxes from asm");
                }
            };
            builder.build()
        }
        _ => syntect::parsing::SyntaxSet::load_defaults_newlines(),
    };
    syntax_set
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addr_to_u64() {
        assert_eq!(addr_to_u64("0x1234"), Some(0x1234_u64));
        assert_eq!(addr_to_u64("0x00001234"), Some(0x1234_u64));
        assert_eq!(addr_to_u64("1234"), None);
    }

    #[test]
    fn test_theme() {
        get_theme();
    }

    #[test]
    fn test_syntax() {
        let syntax_set = get_syntax_set("asm");
        assert!(syntax_set.find_syntax_by_extension("asm").is_some());
        let syntax_set = get_syntax_set("cpp");
        assert!(syntax_set.find_syntax_by_extension("cpp").is_some());
        assert!(syntax_set.find_syntax_by_extension("h").is_some());
    }
}
