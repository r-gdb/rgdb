use lalrpop_util::lalrpop_mod;
lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::vec_box)]
    miout,
    "/mi/miout.rs"
);
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Tok {
    Eq,
    DoubleQuotes,
    Comma,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OutOfBandRecordType {
    AsyncRecord(AsyncRecordType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AsyncRecordType {
    NotifyAsyncOutput(NotifyAsyncOutputType),
    ExecAsyncOutput(ExecAsyncOutputType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExecAsyncOutputType {
    pub async_output: AsyncOutputType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NotifyAsyncOutputType {
    pub async_output: AsyncOutputType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AsyncOutputType {
    pub async_class: AsyncClassType,
    pub resaults: Vec<ResultType>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AsyncClassType {
    Stopped,
    Running,
    ThreadSelected,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NewLineType {
    Linux,
    Windows,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueType {
    ConstType(String),
    TupleType(TupleType),
    ListType(ListType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResultType {
    pub variable: String,
    pub value: ValueType,
}

#[derive(Debug, PartialEq, Eq, Clone)]

pub enum TupleType {
    None,
    Results(Vec<ResultType>),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ListType {
    None,
    ResultList(Vec<ResultType>),
    ValueList(Vec<ValueType>),
}

pub fn apply_string_escapes(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '\"' => vec!['\\', '\"'],
            _ => vec![c],
        })
        .flatten()
        .collect()
}
pub fn vec_string_to_string(s: Vec<String>) -> String {
    s.iter()
        .map(|s| s.bytes())
        .flatten()
        .map(|c| char::from(c))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn f_string_char_1() {
        let s = r#"c"#;
        let a = miout::TokStringCharParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("c"));
    }
    #[test]
    fn f_string_char_2() {
        let s = r#"3"#;
        let a = miout::TokStringCharParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("3"));
    }

    #[test]
    fn f_id() {
        let s = r#"3asdfwerasdf"#;
        let a = miout::TokStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from(s));
    }

    #[test]
    fn f_double_quotes() {
        let s = r##"""##;
        let a = miout::TokDoubleQuotesParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == Tok::DoubleQuotes);
    }

    #[test]
    fn f_c_string() {
        let s = r##""3asdfwerasdf""##;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("3asdfwerasdf"));
    }

    #[test]
    fn f_c_string_1() {
        let s = r###""\"3asdfwerasdf""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("\\\"3asdfwerasdf"));
    }

    #[test]
    fn f_c_string_2() {
        let s: &str = r###""3asdfwe\\rasdf""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("3asdfwe\\\\rasdf"));
    }

    #[test]
    fn f_c_string_3() {
        let s = r###""[]]""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("[]]"));
    }

    #[test]
    fn f_c_string_4() {
        let s = r###""{""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("{"));
    }
    #[test]
    fn f_c_string_5() {
        let s = r###""aaaa,""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("aaaa,"));
    }
    #[test]
    fn f_c_string_6() {
        let s = r###""aaa=a""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("aaa=a"));
    }

    #[test]
    fn f_c_string_7() {
        let s = r###""~`!@#$%^&*()_-+=<,.>/?:;'|{[}]""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from(r###"~`!@#$%^&*()_-+=<,.>/?:;'|{[}]"###));
    }

    // #[test]
    // fn f_c_string_8() {
    //     let s = r###""中文""###;
    //     let a = miout::TokCStringParser::new().parse(s);
    //     println!("s:{:?} {} {:?}", &s, s.len(), &a);
    //     assert!(a.unwrap() == String::from(r###"中文"###));
    // }
    #[test]
    fn f_tok_value_const() {
        let a = miout::TokValueParser::new().parse("\"/lib64/libexpat.so.1\"");
        assert!(a.unwrap() == ValueType::ConstType("/lib64/libexpat.so.1".to_string()));
    }

    #[test]
    fn f_tok_list_empty() {
        let a = miout::TokListParser::new().parse("[]");
        assert!(a.unwrap() == ListType::None);
    }

    #[test]
    fn f_tuple_type_empty() {
        let a = miout::TokTupleParser::new().parse("{}");
        assert!(a.unwrap() == TupleType::None);
    }
    #[test]

    fn f_tok_result_1() {
        let a = miout::TokResultParser::new().parse(r##"result={}"##);
        assert!(
            a.unwrap()
                == ResultType {
                    variable: "result".to_string(),
                    value: ValueType::TupleType(TupleType::None)
                }
        );
    }
    #[test]

    fn f_tok_result_2() {
        let a = miout::TokResultParser::new().parse(r##"res-ult=[]"##);
        assert!(
            a.unwrap()
                == ResultType {
                    variable: "res-ult".to_string(),
                    value: ValueType::ListType(ListType::None)
                }
        );
    }
    #[test]

    fn f_tok_mul_tuple() {
        let a =
            miout::TokTupleParser::new().parse(r##"{number="1",type="breakpoint",disp="del"}"##);
        assert!(
            a.unwrap()
                == TupleType::Results(vec![
                    ResultType {
                        variable: "number".to_string(),
                        value: ValueType::ConstType("1".to_string())
                    },
                    ResultType {
                        variable: "type".to_string(),
                        value: ValueType::ConstType("breakpoint".to_string())
                    },
                    ResultType {
                        variable: "disp".to_string(),
                        value: ValueType::ConstType("del".to_string())
                    }
                ])
        );
    }
    #[test]
    fn f_tok_list_resault() {
        let a = miout::TokListParser::new().parse(r##"[number="1",type="breakpoint",disp="del"]"##);
        assert!(
            a.unwrap()
                == ListType::ResultList(vec![
                    ResultType {
                        variable: "number".to_string(),
                        value: ValueType::ConstType("1".to_string())
                    },
                    ResultType {
                        variable: "type".to_string(),
                        value: ValueType::ConstType("breakpoint".to_string())
                    },
                    ResultType {
                        variable: "disp".to_string(),
                        value: ValueType::ConstType("del".to_string())
                    }
                ])
        );
    }

    #[test]
    fn f_tok_list_value() {
        let a = miout::TokListParser::new()
            .parse(r##"[{from="0x00007ffff5106ff0",to="0x00007ffff5107cd2"},[a="cccc"]]"##);
        assert!(
            a.unwrap()
                == ListType::ValueList(vec![
                    ValueType::TupleType(TupleType::Results(vec![
                        ResultType {
                            variable: "from".to_string(),
                            value: ValueType::ConstType("0x00007ffff5106ff0".to_string())
                        },
                        ResultType {
                            variable: "to".to_string(),
                            value: ValueType::ConstType("0x00007ffff5107cd2".to_string())
                        }
                    ])),
                    ValueType::ListType(ListType::ResultList(vec![ResultType {
                        variable: "a".to_string(),
                        value: ValueType::ConstType("cccc".to_string())
                    }]))
                ])
        );
    }

    #[test]
    fn f_to_k_async_output_type() {
        let a = miout::TokOutOfBandRecordParser::new()
        .parse(r##"=stopped,reason="end-stepping-range",frame={addr="0x00000000004006ff",func="main",args=[],file="a.c",fullname="/home/shizhilvren/c++/a.c",line="27"},thread-id="1",stopped-threads="all",core="6""##);
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::Stopped,
                            resaults: vec![
                                ResultType {
                                    variable: "reason".to_string(),
                                    value: ValueType::ConstType("end-stepping-range".to_string()),
                                },
                                ResultType {
                                    variable: "frame".to_string(),
                                    value: ValueType::TupleType(TupleType::Results(vec![
                                        ResultType {
                                            variable: "addr".to_string(),
                                            value: ValueType::ConstType(
                                                "0x00000000004006ff".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "func".to_string(),
                                            value: ValueType::ConstType("main".to_string()),
                                        },
                                        ResultType {
                                            variable: "args".to_string(),
                                            value: ValueType::ListType(ListType::None),
                                        },
                                        ResultType {
                                            variable: "file".to_string(),
                                            value: ValueType::ConstType("a.c".to_string()),
                                        },
                                        ResultType {
                                            variable: "fullname".to_string(),
                                            value: ValueType::ConstType(
                                                "/home/shizhilvren/c++/a.c".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "line".to_string(),
                                            value: ValueType::ConstType("27".to_string()),
                                        },
                                    ]))
                                },
                                ResultType {
                                    variable: "thread-id".to_string(),
                                    value: ValueType::ConstType("1".to_string()),
                                },
                                ResultType {
                                    variable: "stopped-threads".to_string(),
                                    value: ValueType::ConstType("all".to_string()),
                                },
                                ResultType {
                                    variable: "core".to_string(),
                                    value: ValueType::ConstType("6".to_string()),
                                },
                            ],
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_to_k_async_output_type_1() {
        let a = miout::TokOutOfBandRecordParser::new().parse(r##"=stopped,arch="i386:x86-64""##);
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::Stopped,
                            resaults: vec![ResultType {
                                variable: "arch".to_string(),
                                value: ValueType::ConstType("i386:x86-64".to_string()),
                            },],
                        }
                    }
                ))
        );
    }
    #[test]
    fn f_to_k_async_output_type_2() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=thread-selected,id=\"1\",frame={level=\"1\",addr=\"0x000000000020198c\",func=\"main\",args=[],file=\"args.c\",fullname=\"/remote/x/x/code/c++/args.c\",line=\"7\",arch=\"i386:x86-64\"}\n");
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::ThreadSelected,
                            resaults: vec![
                                ResultType {
                                    variable: "id".to_string(),
                                    value: ValueType::ConstType("1".to_string()),
                                },
                                ResultType {
                                    variable: "frame".to_string(),
                                    value: ValueType::TupleType(TupleType::Results(vec![
                                        ResultType {
                                            variable: "level".to_string(),
                                            value: ValueType::ConstType("1".to_string()),
                                        },
                                        ResultType {
                                            variable: "addr".to_string(),
                                            value: ValueType::ConstType(
                                                "0x000000000020198c".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "func".to_string(),
                                            value: ValueType::ConstType("main".to_string()),
                                        },
                                        ResultType {
                                            variable: "args".to_string(),
                                            value: ValueType::ListType(ListType::None),
                                        },
                                        ResultType {
                                            variable: "file".to_string(),
                                            value: ValueType::ConstType("args.c".to_string()),
                                        },
                                        ResultType {
                                            variable: "fullname".to_string(),
                                            value: ValueType::ConstType(
                                                "/remote/x/x/code/c++/args.c"
                                                    .to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "line".to_string(),
                                            value: ValueType::ConstType("7".to_string()),
                                        },
                                        ResultType {
                                            variable: "arch".to_string(),
                                            value: ValueType::ConstType("i386:x86-64".to_string()),
                                        },
                                    ])),
                                },
                            ],
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_tok_exec_async_output_type() {
        let a = miout::TokOutOfBandRecordParser::new().parse(r##"*running,thread-id="1""##);
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::ExecAsyncOutput(
                    ExecAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::Running,
                            resaults: vec![ResultType {
                                variable: "thread-id".to_string(),
                                value: ValueType::ConstType("1".to_string()),
                            }]
                        }
                    }
                ))
        );
    }

    #[test]
    fn f0() {
        // let s: Vec<String> = vec![];
        // let s = s
        //     .iter()
        //     .map(|s| s.bytes())
        //     .flatten()
        //     .map(|c| char::from(c))
        //     .collect::<String>();
        // ResultType {
        //     variable: String::from(""),
        //     value: ValueType::ConstType(String::from("")),
        // };
        // let e:ResultType;
        // let v: Vec<(ResultType, Tok)> = vec![];
        // let mut v = v.iter().map(|(r, c)| r).collect::<Vec<ResultType>>();
        // v.push(e);
        // v
    }
}
