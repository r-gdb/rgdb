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
