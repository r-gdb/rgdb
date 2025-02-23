// use bytes;
use crate::mi::token::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakPointAction {
    Signal(BreakPointSignalAction),
    Multiple(BreakPointMultipleAction),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakPointMultipleAction {
    pub number: String,
    pub enabled: bool,
    pub bps: Vec<BreakPointSignalAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakPointSignalAction {
    pub number: String,
    pub enabled: bool,
    pub src: Option<BreakPointSignalActionSrc>,
    pub addr: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakPointSignalActionSrc {
    pub fullname: String,
    pub line: u64,
}

pub fn show_breakpoint_deleted(a: &OutOfBandRecordType) -> Option<u64> {
    let get_breakpoint_deleted = |r: &ResultType| -> Option<u64> {
        let mut id = None;
        if r.variable.as_str() == "id" {
            if let ValueType::Const(v) = &r.value {
                if let std::result::Result::Ok(l) = v.parse::<u64>() {
                    id = Some(l)
                }
            }
        }
        id
    };
    let mut ret = None;
    let OutOfBandRecordType::AsyncRecord(a) = a;
    match a {
        AsyncRecordType::NotifyAsyncOutput(a) => match a.async_output.async_class {
            AsyncClassType::BreakpointDeleted => a.async_output.resaults.iter().for_each(|r| {
                ret = get_breakpoint_deleted(r);
            }),
            _ => {}
        },
        _ => {}
    };
    ret
}

fn get_from_signal_point(v: &ValueType) -> Option<BreakPointSignalAction> {
    let mut file = None;
    let mut line = None;
    let mut number = None;
    let mut enabled = None;
    let mut addr = None;
    if let ValueType::Tuple(Tuple::Results(rs)) = v {
        rs.iter().for_each(|r| match r.variable.as_str() {
            "fullname" => {
                if let ValueType::Const(f) = &r.value {
                    file = Some(f.clone())
                }
            }
            "line" => {
                if let ValueType::Const(l) = &r.value {
                    if let std::result::Result::Ok(l) = l.parse::<u64>() {
                        line = Some(l)
                    }
                }
            }
            "number" => {
                if let ValueType::Const(l) = &r.value {
                    number = Some(l)
                }
            }
            "enabled" => {
                enabled = match &r.value {
                    ValueType::Const(v) => match v.as_str() {
                        "y" => Some(true),
                        "n" => Some(false),
                        _ => None,
                    },
                    _ => None,
                };
            }
            "addr" => {
                if let ValueType::Const(l) = &r.value {
                    addr = Some(l.clone());
                }
            }
            _ => {}
        });
    }
    match (addr, file, line, number, enabled) {
        (Some(addr), Some(file), Some(line), Some(number), Some(enabled)) => {
            Some(BreakPointSignalAction {
                number: number.clone(),
                enabled,
                src: Some(BreakPointSignalActionSrc {
                    fullname: file,
                    line,
                }),
                addr,
            })
        }
        (Some(addr), _, _, Some(number), Some(enabled)) => Some(BreakPointSignalAction {
            number: number.clone(),
            enabled,
            addr,
            src: None,
        }),
        _ => None,
    }
}

fn get_from_bkpt(r: &ResultType) -> Option<BreakPointAction> {
    let mut file = None;
    let mut line = None;
    let mut number = None;
    let mut enabled = None;
    let mut multiple = false;
    let mut addr = None;
    let mut bps = vec![];
    if r.variable.as_str() == "bkpt" {
        if let ValueType::Tuple(Tuple::Results(rs)) = &r.value {
            rs.iter().for_each(|r| match r.variable.as_str() {
                "fullname" => {
                    if let ValueType::Const(f) = &r.value {
                        file = Some(f.clone())
                    }
                }
                "line" => {
                    if let ValueType::Const(l) = &r.value {
                        if let std::result::Result::Ok(l) = l.parse::<u64>() {
                            line = Some(l)
                        }
                    }
                }
                "number" => {
                    if let ValueType::Const(l) = &r.value {
                        number = Some(l)
                    }
                }
                "enabled" => {
                    enabled = match &r.value {
                        ValueType::Const(v) => match v.as_str() {
                            "y" => Some(true),
                            "n" => Some(false),
                            _ => None,
                        },
                        _ => None,
                    };
                }
                "addr" => {
                    if let ValueType::Const(l) = &r.value {
                        match l.as_str() {
                            "<MULTIPLE>" => multiple = true,
                            "<PENDING>" => {}
                            _ => addr = Some(l.clone()),
                        }
                    }
                }
                // for mi3 and upper
                "locations" => {
                    if let ValueType::List(List::Values(list)) = &r.value {
                        list.iter().for_each(|v| {
                            if let Some(p) = get_from_signal_point(v) {
                                bps.push(p);
                            }
                        })
                    }
                }
                _ => {}
            });
        }
    }
    match (addr, file, line, number, enabled, multiple) {
        (Some(addr), Some(file), Some(line), Some(number), Some(enabled), false) => {
            Some(BreakPointAction::Signal(BreakPointSignalAction {
                number: number.clone(),
                enabled,
                src: Some(BreakPointSignalActionSrc {
                    fullname: file,
                    line,
                }),
                addr,
            }))
        }

        (Some(addr), _, _, Some(number), Some(enabled), false) => {
            Some(BreakPointAction::Signal(BreakPointSignalAction {
                number: number.clone(),
                enabled,
                addr,
                src: None,
            }))
        }
        (_, None, None, Some(number), Some(enabled), true) => {
            Some(BreakPointAction::Multiple(BreakPointMultipleAction {
                number: number.clone(),
                enabled,
                bps,
            }))
        }
        _ => None,
    }
}

pub fn show_bkpt(a: &OutOfBandRecordType) -> Option<BreakPointAction> {
    let mut ret = None;
    let OutOfBandRecordType::AsyncRecord(a) = a;
    match a {
        AsyncRecordType::NotifyAsyncOutput(a) => match a.async_output.async_class {
            AsyncClassType::BreakpointCreated | AsyncClassType::BreakpointModified => {
                a.async_output.resaults.iter().for_each(|r| {
                    ret = get_from_bkpt(r);
                });

                if let Some(BreakPointAction::Multiple(bp)) = ret.as_mut() {
                    // for mi2
                    a.async_output.values.iter().for_each(|v| {
                        if let Some(p) = get_from_signal_point(v) {
                            bp.bps.push(p);
                        }
                    });
                };
            }
            _ => {}
        },
        _ => {}
    };
    ret
}

#[cfg(test)]
mod tests {
    use crate::mi::breakpointmi::*;
    use crate::mi::miout;
    #[test]
    fn f_breakpoint_created() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-created,bkpt={number=\"1\",type=\"breakpoint\",disp=\"del\",enabled=\"y\",addr=\"0x0000000000404570\",func=\"main\",file=\"tmux.c\",fullname=\"/home/shizhilvren/tmux/tmux.c\",line=\"355\",thread-groups=[\"i1\"],times=\"0\",original-location=\"main\"}\n" );
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::BreakpointCreated,
                            values: vec![],
                            resaults: vec![ResultType {
                                variable: "bkpt".to_string(),
                                value: ValueType::Tuple(Tuple::Results(vec![
                                    ResultType {
                                        variable: "number".to_string(),
                                        value: ValueType::Const("1".to_string()),
                                    },
                                    ResultType {
                                        variable: "type".to_string(),
                                        value: ValueType::Const("breakpoint".to_string()),
                                    },
                                    ResultType {
                                        variable: "disp".to_string(),
                                        value: ValueType::Const("del".to_string()),
                                    },
                                    ResultType {
                                        variable: "enabled".to_string(),
                                        value: ValueType::Const("y".to_string()),
                                    },
                                    ResultType {
                                        variable: "addr".to_string(),
                                        value: ValueType::Const("0x0000000000404570".to_string()),
                                    },
                                    ResultType {
                                        variable: "func".to_string(),
                                        value: ValueType::Const("main".to_string()),
                                    },
                                    ResultType {
                                        variable: "file".to_string(),
                                        value: ValueType::Const("tmux.c".to_string()),
                                    },
                                    ResultType {
                                        variable: "fullname".to_string(),
                                        value: ValueType::Const(
                                            "/home/shizhilvren/tmux/tmux.c".to_string()
                                        ),
                                    },
                                    ResultType {
                                        variable: "line".to_string(),
                                        value: ValueType::Const("355".to_string()),
                                    },
                                    ResultType {
                                        variable: "thread-groups".to_string(),
                                        value: ValueType::List(List::Values(vec![
                                            ValueType::Const("i1".to_string())
                                        ])),
                                    },
                                    ResultType {
                                        variable: "times".to_string(),
                                        value: ValueType::Const("0".to_string()),
                                    },
                                    ResultType {
                                        variable: "original-location".to_string(),
                                        value: ValueType::Const("main".to_string()),
                                    },
                                ])),
                            }]
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_breakpoint_modified() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-modified,bkpt={}\n");
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::BreakpointModified,
                            values: vec![],
                            resaults: vec![ResultType {
                                variable: "bkpt".to_string(),
                                value: ValueType::Tuple(Tuple::None),
                            }]
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_breakpoint_deleted() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-deleted,id=\"1\"\n");
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::BreakpointDeleted,
                            values: vec![],
                            resaults: vec![ResultType {
                                variable: "id".to_string(),
                                value: ValueType::Const("1".to_string()),
                            }]
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_breakpoint_created_2() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-created,bkpt={number=\"1\",type=\"breakpoint\",disp=\"del\",enabled=\"y\",addr=\"0x0000000000404570\",func=\"main\",file=\"tmux.c\",fullname=\"/home/shizhilvren/tmux/tmux.c\",line=\"355\",thread-groups=[\"i1\"],times=\"0\",original-location=\"main\"}\n" );
        let bkpt = show_bkpt(&a.unwrap());
        assert!(
            bkpt == Some(BreakPointAction::Signal(BreakPointSignalAction {
                number: "1".to_string(),
                enabled: true,
                src: Some(BreakPointSignalActionSrc {
                    fullname: "/home/shizhilvren/tmux/tmux.c".to_string(),
                    line: 355_u64,
                }),
                addr: "0x0000000000404570".to_string(),
            }))
        );
    }

    #[test]
    fn f_breakpoint_modified_2() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-modified,bkpt={number=\"2\",type=\"breakpoint\",disp=\"keep\",enabled=\"n\",addr=\"0x0000000000404570\",func=\"main\",file=\"tmux.c\",fullname=\"/home/shizhilvren/tmux/tmux.c\",line=\"355\",thread-groups=[\"i1\"],cond=\"1==2\",times=\"0\",original-location=\"main\"}\n"  );
        let bkpt = show_bkpt(&a.unwrap());
        assert!(
            bkpt == Some(BreakPointAction::Signal(BreakPointSignalAction {
                number: "2".to_string(),
                enabled: false,
                src: Some(BreakPointSignalActionSrc {
                    fullname: "/home/shizhilvren/tmux/tmux.c".to_string(),
                    line: 355_u64,
                }),
                addr: "0x0000000000404570".to_string(),
            }))
        );
    }

    #[test]
    fn f_breakpoint_deleted_2() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-deleted,id=\"11\"\n");
        let bkpt = show_breakpoint_deleted(&a.unwrap());
        assert!(bkpt == Some(11_u64));
    }

    #[test]
    fn f_breakpoint_modified_3() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-modified,bkpt={number=\"5\",type=\"breakpoint\",disp=\"keep\",enabled=\"n\",addr=\"<MULTIPLE>\",times=\"3\",original-location=\"/home/shizhilvren/tmux/environ.c:1\"},\
{number=\"5.1\",enabled=\"y\",addr=\"0x0000000000426d70\",func=\"environ_RB_INSERT\",file=\"environ.c\",fullname=\"/home/shizhilvren/tmux/environ.c\",line=\"34\",thread-groups=[\"i1\"]},\
{number=\"5.2\",enabled=\"n\",addr=\"0x0000000000427c61\",func=\"environ_RB_MINMAX\",file=\"environ.c\",fullname=\"/home/shizhilvren/tmux/environ.c\",line=\"34\",thread-groups=[\"i1\"]}\n"  );
        let bkpt = show_bkpt(&a.unwrap());
        println!("{:?}", &bkpt);

        assert!(
            bkpt == Some(BreakPointAction::Multiple(BreakPointMultipleAction {
                number: "5".to_string(),
                enabled: false,
                bps: vec![
                    BreakPointSignalAction {
                        number: "5.1".to_string(),
                        enabled: true,
                        src: Some(BreakPointSignalActionSrc {
                            line: 34_u64,
                            fullname: "/home/shizhilvren/tmux/environ.c".to_string()
                        }),
                        addr: "0x0000000000426d70".to_string(),
                    },
                    BreakPointSignalAction {
                        number: "5.2".to_string(),
                        enabled: false,
                        src: Some(BreakPointSignalActionSrc {
                            line: 34_u64,
                            fullname: "/home/shizhilvren/tmux/environ.c".to_string()
                        }),
                        addr: "0x0000000000427c61".to_string()
                    },
                ]
            }))
        );
    }

    #[test]
    fn f_breakpoint_modified_4() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-modified,\
    bkpt={number=\"2\",type=\"breakpoint\",disp=\"keep\",enabled=\"n\",addr=\"<MULTIPLE>\",times=\"2\",original-location=\"environ.c:34\",\
    locations=[\
    {number=\"2.1\",enabled=\"y\",addr=\"0x0000000000426d70\",func=\"environ_RB_INSERT\",file=\"environ.c\",fullname=\"/home/shizhilvren/tmux/environ.c\",line=\"34\",thread-groups=[\"i1\"]},\
    {number=\"2.8\",enabled=\"n\",addr=\"0x0000000000427c61\",func=\"environ_RB_MINMAX\",file=\"environ.c\",fullname=\"/home/shizhilvren/tmux/environ.c\",line=\"34\",thread-groups=[\"i1\"]}]}\n"  );
        println!("{:?}", &a);
        let bkpt = show_bkpt(&a.unwrap());
        println!("{:?}", &bkpt);
        assert!(
            bkpt == Some(BreakPointAction::Multiple(BreakPointMultipleAction {
                number: "2".to_string(),
                enabled: false,
                bps: vec![
                    BreakPointSignalAction {
                        number: "2.1".to_string(),
                        enabled: true,
                        src: Some(BreakPointSignalActionSrc {
                            line: 34_u64,
                            fullname: "/home/shizhilvren/tmux/environ.c".to_string()
                        }),
                        addr: "0x0000000000426d70".to_string(),
                    },
                    BreakPointSignalAction {
                        number: "2.8".to_string(),
                        enabled: false,
                        src: Some(BreakPointSignalActionSrc {
                            line: 34_u64,
                            fullname: "/home/shizhilvren/tmux/environ.c".to_string()
                        }),
                        addr: "0x0000000000427c61".to_string(),
                    },
                ]
            }))
        );
    }

    #[test]
    fn f_breakpoint() {
        let a = BreakPointAction::Multiple(BreakPointMultipleAction {
            number: "5".to_string(),
            enabled: false,
            bps: vec![
                BreakPointSignalAction {
                    number: "5.1".to_string(),
                    enabled: true,
                    src: Some(BreakPointSignalActionSrc {
                        line: 34_u64,
                        fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                    }),
                    addr: "0x1234".to_string(),
                },
                BreakPointSignalAction {
                    number: "5.2".to_string(),
                    enabled: false,
                    src: Some(BreakPointSignalActionSrc {
                        line: 34_u64,
                        fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
                    }),
                    addr: "0x1234".to_string(),
                },
            ],
        });
        let b = BreakPointAction::Signal(BreakPointSignalAction {
            number: "5".to_string(),
            enabled: true,
            src: Some(BreakPointSignalActionSrc {
                line: 34_u64,
                fullname: "/home/shizhilvren/tmux/environ.c".to_string(),
            }),
            addr: "0x1234".to_string(),
        });
        assert!(a != b);
    }
}
