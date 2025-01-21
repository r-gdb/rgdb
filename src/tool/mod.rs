use color_eyre::{eyre::Ok, Result};
use libc::ptsname;
use ratatui::layout::{Constraint, Layout, Rect};
use std::ffi::CStr;
use lalrpop_util::lalrpop_mod;


pub fn get_pty_name(fd: i32) -> Result<String> {
    let name = unsafe { ptsname(fd) };
    let c_str = unsafe { CStr::from_ptr(name) }.to_str()?;
    Ok(c_str.to_string())
}

pub fn get_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Min(1),
        Constraint::Percentage(75),
        Constraint::Percentage(25),
    ])
    .areas(area)
}

 macro_rules!  lalrpop_mod_doc {
    ($vis:vis $name:ident) => {
        lalrpop_util::lalrpop_mod!(
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::vec_box)]
            $vis $name);
    }
}
pub(crate) use lalrpop_mod_doc;
