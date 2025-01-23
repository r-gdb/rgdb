use serde::{Deserialize, Serialize};
use strum::Display;

use crate::components::gdbmi;
use crate::components::gdbtty;
use crate::components::code;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    Up,
    Down,
    Gdbtty(gdbtty::Action),
    Gdbmi(gdbmi::Action),
    Code(code::Action),
}
