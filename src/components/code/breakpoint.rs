use crate::mi::breakpointmi::{BreakPointAction, BreakPointMultipleAction, BreakPointSignalAction};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointMultipleData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub bps: Vec<BreakPointSignalData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointSignalData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub fullname: String,
    pub line: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPointData {
    Signal(BreakPointSignalData),
    Multiple(BreakPointMultipleData),
}

impl From<&BreakPointSignalAction> for BreakPointSignalData {
    fn from(a: &BreakPointSignalAction) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            fullname: a.fullname.clone(),
            line: a.line,
        }
    }
}

impl From<&BreakPointMultipleAction> for BreakPointMultipleData {
    fn from(a: &BreakPointMultipleAction) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            bps: a.bps.iter().map(BreakPointSignalData::from).collect(),
        }
    }
}

impl From<&BreakPointAction> for BreakPointData {
    fn from(a: &BreakPointAction) -> Self {
        match a {
            BreakPointAction::Signal(p) => Self::Signal(BreakPointSignalData::from(p)),
            BreakPointAction::Multiple(p) => Self::Multiple(BreakPointMultipleData::from(p)),
        }
    }
}

impl crate::tool::HashSelf<String> for BreakPointData {
    fn get_key(&self) -> Rc<String> {
        match self {
            Self::Signal(p) => p.number.clone(),
            Self::Multiple(p) => p.number.clone(),
        }
    }
}
