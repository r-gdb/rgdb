// use bytes;
use lalrpop_util::lalrpop_mod;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::vec_box)]
    miout,
    "/mi/miout.rs"
);
use crate::mi::token::*;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct Bkpt {
    pub number: u64,
    pub enabled: bool,
    pub fullnmae: String,
    pub line: u64,
}

impl Hash for Bkpt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.number.hash(state);
    }
}

impl PartialEq for Bkpt {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
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

pub fn show_bkpt(a: &OutOfBandRecordType) -> Option<Bkpt> {
    let get_from_bkpt = |r: &ResultType| -> Option<Bkpt> {
        let mut file = None;
        let mut line = None;
        let mut number = None;
        let mut enabled = None;
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
                            if let std::result::Result::Ok(l) = l.parse::<u64>() {
                                number = Some(l)
                            }
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
                    _ => {}
                });
            }
        }
        match (file, line, number, enabled) {
            (Some(file), Some(line), Some(number), Some(enabled)) => Some(Bkpt {
                number: number,
                enabled: enabled,
                fullnmae: file,
                line: line,
            }),
            _ => None,
        }
    };

    let mut ret = None;
    let OutOfBandRecordType::AsyncRecord(a) = a;
    match a {
        AsyncRecordType::NotifyAsyncOutput(a) => match a.async_output.async_class {
            AsyncClassType::BreakpointCreated => a.async_output.resaults.iter().for_each(|r| {
                ret = get_from_bkpt(r);
            }),
            AsyncClassType::BreakpointModified => a.async_output.resaults.iter().for_each(|r| {
                ret = get_from_bkpt(r);
            }),
            _ => {}
        },
        _ => {}
    };
    ret
}

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
                                    value: ValueType::List(List::ValueList(vec![
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
        bkpt == Some(Bkpt {
            number: 1_u64,
            enabled: true,
            fullnmae: "/home/shizhilvren/tmux/tmux.c".to_string(),
            line: 355_u64,
        })
    );
}

#[test]
fn f_breakpoint_modified_2() {
    let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-modified,bkpt={number=\"2\",type=\"breakpoint\",disp=\"keep\",enabled=\"n\",addr=\"0x0000000000404570\",func=\"main\",file=\"tmux.c\",fullname=\"/home/shizhilvren/tmux/tmux.c\",line=\"355\",thread-groups=[\"i1\"],cond=\"1==2\",times=\"0\",original-location=\"main\"}\n"  );
    let bkpt = show_bkpt(&a.unwrap());
    assert!(
        bkpt == Some(Bkpt {
            number: 2_u64,
            enabled: false,
            fullnmae: "/home/shizhilvren/tmux/tmux.c".to_string(),
            line: 355_u64,
        })
    );
}

#[test]
fn f_breakpoint_deleted_2() {
    let a = miout::TokOutOfBandRecordParser::new().parse("=breakpoint-deleted,id=\"11\"\n");
    let bkpt = show_breakpoint_deleted(&a.unwrap());
    assert!(bkpt == Some(11_u64));
}
