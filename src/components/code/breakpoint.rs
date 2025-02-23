use crate::mi::breakpointmi::{
    BreakPointAction, BreakPointMultipleAction, BreakPointSignalAction, BreakPointSignalActionSrc,
};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPointData {
    Signal(BreakPointSignalData),
    Multiple(BreakPointMultipleData),
}

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
    pub src: Option<BreakPointSignalSrcData>,
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointSignalSrcData {
    pub fullname: String,
    pub line: u64,
}

impl From<&BreakPointSignalAction> for BreakPointSignalData {
    fn from(a: &BreakPointSignalAction) -> Self {
        a.src.as_ref().map_or(
            BreakPointSignalData {
                number: Rc::new(a.number.clone()),
                enabled: a.enabled,
                src: None,
                addr: a.addr.clone(),
            },
            |src| BreakPointSignalData {
                number: Rc::new(a.number.clone()),
                enabled: a.enabled,
                src: Some(BreakPointSignalSrcData::from(src)),
                addr: a.addr.clone(),
            },
        )
    }
}

impl From<&BreakPointSignalActionSrc> for BreakPointSignalSrcData {
    fn from(a: &BreakPointSignalActionSrc) -> Self {
        Self {
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
            Self::Signal(p) => p.get_key(),
            Self::Multiple(p) => p.get_key(),
        }
    }
}

impl crate::tool::HashSelf<String> for BreakPointMultipleData {
    fn get_key(&self) -> Rc<String> {
        self.number.clone()
    }
}

impl crate::tool::HashSelf<String> for BreakPointSignalData {
    fn get_key(&self) -> Rc<String> {
        self.number.clone()
    }
}
