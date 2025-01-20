use color_eyre::{eyre::Ok, Result};
use libc::ptsname;
use std::ffi::CStr;

pub fn get_pty_name(fd: i32) -> Result<String> {
    let name = unsafe { ptsname(fd) };
    let c_str = unsafe { CStr::from_ptr(name) }.to_str()?;
    Ok(c_str.to_string())
}
