use crate::action;
use crate::components::code::breakpoint::BreakPointData;
use color_eyre::{eyre::Ok, Result};
use libc::ptsname;
use ratatui::layout::{Constraint, Layout, Rect};
use std::collections::HashMap;
use std::ffi::CStr;
use std::hash::Hash;
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
const NORD_THEME: &str = include_str!("../themes/Nord.tmTheme");
const ASSEMBLY_X86_64: &str = include_str!("../syntaxes/assembly_x86_64.sublime-syntax");

pub fn get_pty_name(fd: i32) -> Result<String> {
    let name = unsafe { ptsname(fd) };
    let c_str = unsafe { CStr::from_ptr(name) }.to_str()?;
    Ok(c_str.to_string())
}

fn get_layout_vertical(area: Rect) -> [Rect; 4] {
    Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area)
}
fn get_layout_horizontal(area: Rect) -> [Rect; 4] {
    let [src, gdb] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Fill(1)]).areas(area);
    let [src, src_status] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(src);
    let [gdb, status] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(gdb);
    [src, src_status, gdb, status]
}

pub struct Layouts {
    pub src: Rect,
    pub src_status: Rect,
    pub gdb: Rect,
    pub status: Rect,
}

impl From<(ratatui::layout::Rect, bool)> for Layouts {
    fn from(area: (ratatui::layout::Rect, bool)) -> Self {
        let (area, is_horizontal) = area;
        match is_horizontal {
            false => {
                let [src, src_status, gdb, status] = get_layout_vertical(area);
                Layouts {
                    src,
                    src_status,
                    gdb,
                    status,
                }
            }
            true => {
                let [src, src_status, gdb, status] = get_layout_horizontal(area);
                Layouts {
                    src,
                    src_status,
                    gdb,
                    status,
                }
            }
        }
    }
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

pub trait StatusFileData: TextFileData {
    fn get_status(&self) -> String {
        self.get_file_name()
    }
}

pub trait HighlightFileData
where
    Self: TextFileData,
{
    fn get_highlight_done(&self) -> bool;
    fn set_highlight_done(&mut self);
    fn get_highlight_lines_range(
        &self,
        start: usize,
        end: usize,
    ) -> (Vec<Vec<(ratatui::style::Color, String)>>, usize, usize);
}
pub trait FileData:
    TextFileData + HighlightFileData + StatusFileData + HashSelf<std::string::String>
{
}

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

pub fn send_action(send: &UnboundedSender<action::Action>, action: action::Action) {
    match send.send(action) {
        std::result::Result::Ok(_) => {}
        std::result::Result::Err(e) => {
            error!("send error: {:?}", e);
        }
    }
}

// 返回值是一个元组，包含三个元素：
// 1. 截取的字符串

pub fn get_str_by_display_range(
    s: &String,
    start: usize,
    end: usize,
) -> Option<(&str, usize, usize)> {
    let ret = UnicodeSegmentation::grapheme_indices(s.as_str(), true)
        .map(|(id, s)| (id, s.width(), s))
        .scan(0_usize, |len, (index, display_width, s)| {
            let display_start = len.clone();
            *len += display_width;
            Some((index, display_start, display_width, s))
        })
        .skip_while(|(_, display_start, _, _)| *display_start < start)
        .take_while(|(_, display_start, display_width, _)| *display_start + *display_width <= end)
        .fold(
            None,
            |range, (id, display_start, display_width, s)| match range {
                None => Some((
                    id,
                    id + s.len(),
                    display_start,
                    display_start + display_width,
                )),
                Some((start, _, display_start_org, _)) => Some((
                    start,
                    id + s.len(),
                    display_start_org,
                    display_start + display_width,
                )),
            },
        )
        .and_then(|(start, end, display_start, display_end)| {
            if start < end && end <= s.len() {
                Some((&s[start..end], display_start, display_end))
            } else {
                None
            }
        });
    ret
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
    #[test]
    fn display_get_1() {
        let s = "1234567890".to_string();
        let start = 0;
        let end = 10;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("1234567890", 0, 10)));
    }
    #[test]
    fn display_get_2() {
        let s = "1234567890".to_string();
        let start = 0;
        let end = 5;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("12345", 0, 5)));
    }
    #[test]
    fn display_get_3() {
        let s = "1234567890".to_string();
        let start = 1;
        let end = 9;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("23456789", 1, 9)));
    }
    #[test]
    fn display_get_4() {
        let s = "0123中文😀567".to_string();
        let start = 1;
        let end = 11;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("123中文😀5", 1, 11)));
    }
    #[test]
    fn display_get_5() {
        let s = "0123中文😀567".to_string();
        let start = 5;
        let end = 11;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("文😀5", 6, 11)));
    }
    #[test]
    fn display_get_6() {
        let s = "0123中文😀567".to_string();
        let start = 5;
        let end = 9;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("文", 6, 8)));
    }
    #[test]
    fn display_get_7() {
        let s = "0123中文😀1234\n".to_string();
        let start = 5;
        let end = 15;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("文😀1234\n", 6, 15)));
    }
    #[test]
    fn display_get_8() {
        let s = "0123中文😀1234\n".to_string();
        let start = 5;
        let end = 2000;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, Some(("文😀1234\n", 6, 15)));
    }
    #[test]
    fn display_get_9() {
        let s = "0123中文😀1234\n".to_string();
        let start = 100;
        let end = 2000;
        let ret = get_str_by_display_range(&s, start, end);
        assert_eq!(ret, None);
    }
    #[test]
    fn test_display_range_with_combining_characters() {
        let s = "a\u{0301}b\u{0301}c".to_string(); // 包含组合字符 "áb́c"
        assert_eq!(
            get_str_by_display_range(&s, 0, 1),
            Some(("a\u{0301}", 0, 1))
        );
        assert_eq!(
            get_str_by_display_range(&s, 1, 2),
            Some(("b\u{0301}", 1, 2))
        );
    }

    #[test]
    fn test_display_range_with_emoji() {
        let s = "😀😃😄😁".to_string();
        assert_eq!(get_str_by_display_range(&s, 0, 4), Some(("😀😃", 0, 4)));
        assert_eq!(get_str_by_display_range(&s, 4, 8), Some(("😄😁", 4, 8)));
    }

    #[test]
    fn test_display_range_with_out_of_bounds() {
        let s = "12345".to_string();
        assert_eq!(get_str_by_display_range(&s, 10, 15), None);
        assert_eq!(get_str_by_display_range(&s, 0, 10), Some(("12345", 0, 5)));
    }
}
