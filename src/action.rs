use serde::{Deserialize, Serialize};
use strum::Display;

use crate::app;
use crate::components::code;
use crate::components::gdbmi;
use crate::components::gdbtty;
use crate::components::home;
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    SwapHV,
    ClearScreen,
    Error(String),
    Help,
    Home(home::Action),
    Gdbtty(gdbtty::Action),
    Gdbmi(gdbmi::Action),
    Code(code::Action),
    Mode(app::Mode),
    CopyStr(String),
}
