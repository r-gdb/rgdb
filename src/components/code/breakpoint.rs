use crate::mi::breakpointmi::{
    BreakPointAction, BreakPointMultipleAction, BreakPointSignalAction, BreakPointSignalActionAsm,
    BreakPointSignalActionSrc,
};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointMultipleData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub bps: Vec<BreakPointSignalData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPointSignalData {
    Src(BreakPointSignalSrcData),
    Asm(BreakPointSignalAsmData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointSignalSrcData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub fullname: String,
    pub line: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakPointSignalAsmData {
    pub number: Rc<String>,
    pub enabled: bool,
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakPointData {
    Signal(BreakPointSignalData),
    Multiple(BreakPointMultipleData),
}

impl From<&BreakPointSignalAction> for BreakPointSignalData {
    fn from(a: &BreakPointSignalAction) -> Self {
        match a {
            BreakPointSignalAction::Src(p) => Self::Src(BreakPointSignalSrcData::from(p)),
            BreakPointSignalAction::Asm(p) => Self::Asm(BreakPointSignalAsmData::from(p)),
        }
    }
}

impl From<&BreakPointSignalActionSrc> for BreakPointSignalSrcData {
    fn from(a: &BreakPointSignalActionSrc) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            fullname: a.fullname.clone(),
            line: a.line,
        }
    }
}

impl From<&BreakPointSignalActionAsm> for BreakPointSignalAsmData {
    fn from(a: &BreakPointSignalActionAsm) -> Self {
        Self {
            number: Rc::new(a.number.clone()),
            enabled: a.enabled,
            addr: a.addr.clone(),
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
        match self {
            Self::Src(p) => p.get_key(),
            Self::Asm(p) => p.get_key(),
        }
    }
}

impl crate::tool::HashSelf<String> for BreakPointSignalAsmData {
    fn get_key(&self) -> Rc<String> {
        self.number.clone()
    }
}

impl crate::tool::HashSelf<String> for BreakPointSignalSrcData {
    fn get_key(&self) -> Rc<String> {
        self.number.clone()
    }
}
