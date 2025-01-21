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
        let s = apply_string_escapes(r##""3asdfwerasdf""##);
        let s = r##""3asdfwerasdf""##;
        let a = miout::TokCStringParser::new().parse(s);
        println!("s:{:?} {} {:?}", &s, s.len(), &a);
        assert!(a.unwrap() == String::from("3asdfwerasdf"));
    }
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
