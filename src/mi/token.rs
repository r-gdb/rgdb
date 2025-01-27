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
    Const(String),
    Tuple(Tuple),
    List(List),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResultType {
    pub variable: String,
    pub value: ValueType,
}

#[derive(Debug, PartialEq, Eq, Clone)]

pub enum Tuple {
    None,
    Results(Vec<ResultType>),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum List {
    None,
    ResultList(Vec<ResultType>),
    ValueList(Vec<ValueType>),
}

// pub fn apply_string_escapes(s: &str) -> String {
//     s.chars()
//         .map(|c| match c {
//             '\\' => vec!['\\', '\\'],
//             '\"' => vec!['\\', '\"'],
//             _ => vec![c],
//         })
//         .flatten()
//         .collect()
// }

pub fn vec_string_to_string(s: Vec<String>) -> String {
    let ans = s.iter().fold(String::from(""), |ans, s| ans + s);
    ans
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn f_string_char_1() {
        let s = r#"c"#;
        let a = miout::TokStringCharParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"c");
    }
    #[test]
    fn f_string_char_2() {
        let s = r#"3"#;
        let a = miout::TokStringCharParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"3");
    }

    #[test]
    fn f_id() {
        let s = r#"3asdfwerasdf"#;
        let a = miout::TokStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *s);
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
        assert!(a.unwrap() == *"3asdfwerasdf");
    }

    #[test]
    fn f_c_string_1() {
        let s = r###""\"3asdfwerasdf""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"\\\"3asdfwerasdf");
    }

    #[test]
    fn f_c_string_2() {
        let s: &str = r###""3asdfwe\\rasdf""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"3asdfwe\\\\rasdf");
    }

    #[test]
    fn f_c_string_3() {
        let s = r###""[]]""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"[]]");
    }

    #[test]
    fn f_c_string_4() {
        let s = r###""{""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"{");
    }
    #[test]
    fn f_c_string_5() {
        let s = r###""aaaa,""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"aaaa,");
    }
    #[test]
    fn f_c_string_6() {
        let s = r###""aaa=a""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *"aaa=a");
    }

    #[test]
    fn f_c_string_7() {
        let s = r###""~`!@#$%^&*()_-+=<,.>/?:;'|{[}]""###;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == *r###"~`!@#$%^&*()_-+=<,.>/?:;'|{[}]"###);
    }

    #[test]
    fn f_c_string_8() {
        let s = r###""中文""###;
        let a = miout::TokCStringParser::new().parse(s);
        if let Ok(ref b) = a {
            println!("{:?}", &b.bytes());
        }
        println!(
            "s:{:?} bytes {:?} {} parse {:?}",
            &s,
            &s.bytes(),
            s.len(),
            &a
        );
        assert!(a.unwrap() == *r###"中文"###); // 78 45 101 135
    }

    #[test]
    fn f_c_string_9() {
        let s = r###""日本語""###;
        let a = miout::TokCStringParser::new().parse(s);
        if let Ok(ref b) = a {
            println!("{:?}", &b.bytes());
        }
        println!(
            "s:{:?} bytes {:?} {} parse {:?}",
            &s,
            &s.bytes(),
            s.len(),
            &a
        );
        assert!(a.unwrap() == *r###"日本語"###);
    }
    #[test]
    fn f_c_string_10() {
        let s = r###""ελληνικά""###;
        let a = miout::TokCStringParser::new().parse(s);
        if let Ok(ref b) = a {
            println!("{:?}", &b.bytes());
        }
        println!(
            "s:{:?} bytes {:?} {} parse {:?}",
            &s,
            &s.bytes(),
            s.len(),
            &a
        );
        assert!(a.unwrap() == *r###"ελληνικά"###);
    }
    #[test]
    fn f_c_string_11() {
        let languages = [
            r##""英语 - English""##,
            r##""印地语 - हिन्दी""##,
            r##""西班牙语 - Español""##,
            r##""阿拉伯语 - العربية""##,
            r##""孟加拉语 - বাংলা""##,
            r##""法语 - Français""##,
            r##""俄语 - Русский""##,
            r##""葡萄牙语 - Português""##,
            r##""乌尔都语 - اردو""##,
            r##""印尼语 - Bahasa Indonesia""##,
            r##""德语 - Deutsch""##,
            r##""日语 - 日本語""##,
            r##""斯瓦希里语 - Kiswahili""##,
            r##""泰卢固语 - తెలుగు""##,
            r##""马拉地语 - मराठी""##,
            r##""泰米尔语 - தமிழ்""##,
            r##""土耳其语 - Türkçe""##,
            r##""越南语 - Tiếng Việt""##,
            r##""韩语 - 한국어""##,
            r##""意大利语 - Italiano""##,
            r##""泰语 - ภาษาไทย""##,
            r##""古吉拉特语 - ગુજરાતી""##,
            r##""波斯语 - فارسی""##,
            r##""波兰语 - Polski""##,
            r##""旁遮普语 - ਪੰਜਾਬੀ""##,
            r##""乌克兰语 - Українська""##,
            r##""马来语 - Bahasa Melayu""##,
            r##""荷兰语 - Nederlands""##,
            r##""菲律宾语 - Filipino""##,
            r##""缅甸语 - မြန်မာဘာသာ""##,
            r##""僧伽罗语 - සිංහල""##,
            r##""高棉语 - ភាសាខ្មែរ""##,
            r##""普什图语 - پښتو""##,
            r##""豪萨语 - Hausa""##,
            r##""约鲁巴语 - Yorùbá""##,
            r##""伊博语 - Igbo""##,
        ];
        languages.iter().for_each(|s| {
            let a = miout::TokCStringParser::new().parse(s);
            if let Ok(ref b) = a {
                println!("{:?}", &b.bytes());
            }
            println!(
                "s:{:?} bytes {:?} {} parse {:?}",
                &s,
                &s.bytes(),
                s.len(),
                &a
            );
            assert!(a.unwrap() == s[1..s.len() - 1]);
        })
    }
    #[test]
    fn f_c_string_12() {
        let s = r###""this is a 中文 in 文章""###;
        let a = miout::TokCStringParser::new().parse(s);
        assert!(a.unwrap() == *r###"this is a 中文 in 文章"###);
    }

    #[test]
    fn f_c_string_13() {
        let s = "\"\t\r\n\"";
        let a = miout::TokCStringParser::new().parse(s);
        assert!(a.unwrap() == *"\t\r\n");
    }

    #[test]
    fn f_c_string_14() {
        let s = "\"¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ\"";
        let a = miout::TokCStringParser::new().parse(s);
        assert!(a.unwrap() == s[1..s.len() - 1]);
    }

    #[test]
    fn f_tok_value_const() {
        let a = miout::TokValueParser::new().parse("\"/lib64/libexpat.so.1\"");
        assert!(a.unwrap() == ValueType::Const("/lib64/libexpat.so.1".to_string()));
    }

    #[test]
    fn f_tok_list_empty() {
        let a = miout::TokListParser::new().parse("[]");
        assert!(a.unwrap() == List::None);
    }

    #[test]
    fn f_tuple_type_empty() {
        let a = miout::TokTupleParser::new().parse("{}");
        assert!(a.unwrap() == Tuple::None);
    }
    #[test]

    fn f_tok_result_1() {
        let a = miout::TokResultParser::new().parse(r##"result={}"##);
        assert!(
            a.unwrap()
                == ResultType {
                    variable: "result".to_string(),
                    value: ValueType::Tuple(Tuple::None)
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
                    value: ValueType::List(List::None)
                }
        );
    }
    #[test]

    fn f_tok_mul_tuple() {
        let a =
            miout::TokTupleParser::new().parse(r##"{number="1",type="breakpoint",disp="del"}"##);
        assert!(
            a.unwrap()
                == Tuple::Results(vec![
                    ResultType {
                        variable: "number".to_string(),
                        value: ValueType::Const("1".to_string())
                    },
                    ResultType {
                        variable: "type".to_string(),
                        value: ValueType::Const("breakpoint".to_string())
                    },
                    ResultType {
                        variable: "disp".to_string(),
                        value: ValueType::Const("del".to_string())
                    }
                ])
        );
    }
    #[test]
    fn f_tok_list_resault() {
        let a = miout::TokListParser::new().parse(r##"[number="1",type="breakpoint",disp="del"]"##);
        assert!(
            a.unwrap()
                == List::ResultList(vec![
                    ResultType {
                        variable: "number".to_string(),
                        value: ValueType::Const("1".to_string())
                    },
                    ResultType {
                        variable: "type".to_string(),
                        value: ValueType::Const("breakpoint".to_string())
                    },
                    ResultType {
                        variable: "disp".to_string(),
                        value: ValueType::Const("del".to_string())
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
                == List::ValueList(vec![
                    ValueType::Tuple(Tuple::Results(vec![
                        ResultType {
                            variable: "from".to_string(),
                            value: ValueType::Const("0x00007ffff5106ff0".to_string())
                        },
                        ResultType {
                            variable: "to".to_string(),
                            value: ValueType::Const("0x00007ffff5107cd2".to_string())
                        }
                    ])),
                    ValueType::List(List::ResultList(vec![ResultType {
                        variable: "a".to_string(),
                        value: ValueType::Const("cccc".to_string())
                    }]))
                ])
        );
    }

    #[test]
    fn f_to_k_async_output_type() {
        let a = miout::TokOutOfBandRecordParser::new()
        .parse(r##"=stopped,reason="end-stepping-range",frame={addr="0x00000000004006ff",func="main",args=[],file="a.c",fullname="/home/shizhilvren/c++/a.c",line="27"},thread-id="1",stopped-threads="all",core="6"
"##);
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
                                    value: ValueType::Const("end-stepping-range".to_string()),
                                },
                                ResultType {
                                    variable: "frame".to_string(),
                                    value: ValueType::Tuple(Tuple::Results(vec![
                                        ResultType {
                                            variable: "addr".to_string(),
                                            value: ValueType::Const(
                                                "0x00000000004006ff".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "func".to_string(),
                                            value: ValueType::Const("main".to_string()),
                                        },
                                        ResultType {
                                            variable: "args".to_string(),
                                            value: ValueType::List(List::None),
                                        },
                                        ResultType {
                                            variable: "file".to_string(),
                                            value: ValueType::Const("a.c".to_string()),
                                        },
                                        ResultType {
                                            variable: "fullname".to_string(),
                                            value: ValueType::Const(
                                                "/home/shizhilvren/c++/a.c".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "line".to_string(),
                                            value: ValueType::Const("27".to_string()),
                                        },
                                    ]))
                                },
                                ResultType {
                                    variable: "thread-id".to_string(),
                                    value: ValueType::Const("1".to_string()),
                                },
                                ResultType {
                                    variable: "stopped-threads".to_string(),
                                    value: ValueType::Const("all".to_string()),
                                },
                                ResultType {
                                    variable: "core".to_string(),
                                    value: ValueType::Const("6".to_string()),
                                },
                            ],
                        }
                    }
                ))
        );
    }

    #[test]
    fn f_to_k_async_output_type_1() {
        let a = miout::TokOutOfBandRecordParser::new().parse("=stopped,arch=\"i386:x86-64\"\n");
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::NotifyAsyncOutput(
                    NotifyAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::Stopped,
                            resaults: vec![ResultType {
                                variable: "arch".to_string(),
                                value: ValueType::Const("i386:x86-64".to_string()),
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
                                    value: ValueType::Const("1".to_string()),
                                },
                                ResultType {
                                    variable: "frame".to_string(),
                                    value: ValueType::Tuple(Tuple::Results(vec![
                                        ResultType {
                                            variable: "level".to_string(),
                                            value: ValueType::Const("1".to_string()),
                                        },
                                        ResultType {
                                            variable: "addr".to_string(),
                                            value: ValueType::Const(
                                                "0x000000000020198c".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "func".to_string(),
                                            value: ValueType::Const("main".to_string()),
                                        },
                                        ResultType {
                                            variable: "args".to_string(),
                                            value: ValueType::List(List::None),
                                        },
                                        ResultType {
                                            variable: "file".to_string(),
                                            value: ValueType::Const("args.c".to_string()),
                                        },
                                        ResultType {
                                            variable: "fullname".to_string(),
                                            value: ValueType::Const(
                                                "/remote/x/x/code/c++/args.c".to_string()
                                            ),
                                        },
                                        ResultType {
                                            variable: "line".to_string(),
                                            value: ValueType::Const("7".to_string()),
                                        },
                                        ResultType {
                                            variable: "arch".to_string(),
                                            value: ValueType::Const("i386:x86-64".to_string()),
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
        let a = miout::TokOutOfBandRecordParser::new().parse("*running,thread-id=\"1\"\r\n");
        println!("{:?}", &a);
        assert!(
            a.unwrap()
                == OutOfBandRecordType::AsyncRecord(AsyncRecordType::ExecAsyncOutput(
                    ExecAsyncOutputType {
                        async_output: AsyncOutputType {
                            async_class: AsyncClassType::Running,
                            resaults: vec![ResultType {
                                variable: "thread-id".to_string(),
                                value: ValueType::Const("1".to_string()),
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
        //     variable: *""),
        //     value: ValueType::Const(*"")),
        // };
        // let e:ResultType;
        // let v: Vec<(ResultType, Tok)> = vec![];
        // let mut v = v.iter().map(|(r, c)| r).collect::<Vec<ResultType>>();
        // v.push(e);
        // v
    }
}
