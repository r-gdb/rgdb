// use bytes;
use crate::mi::token::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisassembleFunctionLine {
    pub address: String,
    pub offset: u64,
    pub inst: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisassembleFunction {
    pub func: String,
    pub insts: Vec<DisassembleFunctionLine>,
}

pub fn get_disassemble_function(r: ResultRecordType) -> Option<DisassembleFunction> {
    let mut same = true;
    let mut func = None;
    let mut insts = vec![];
    if r.result_class == ResultClassType::Done {
        if let Some(v) = r.results.into_iter().next() {
            if v.variable == "asm_insns" {
                if let ValueType::List(List::Values(l)) = v.value {
                    l.into_iter().for_each(|v| {
                        if let Some((f, dfl)) = get_disassemble_function_line(v) {
                            insts.push(dfl);
                            match &func {
                                Some(func) => {
                                    same = same && (*func == f);
                                }
                                None => {
                                    func = Some(f);
                                }
                            }
                        }
                    });
                }
            }
        };
    }

    match (func, same) {
        (Some(func), true) => Some(DisassembleFunction { func, insts }),
        _ => None,
    }
}

fn get_disassemble_function_line(tuple: ValueType) -> Option<(String, DisassembleFunctionLine)> {
    let mut addr = None;
    let mut func = None;
    let mut offset = None;
    let mut inst = None;
    match tuple {
        ValueType::Tuple(Tuple::Results(r)) => {
            r.into_iter().for_each(|r| match r.variable.as_str() {
                "address" => {
                    if let ValueType::Const(s) = r.value {
                        addr = Some(s);
                    }
                }
                "func-name" => {
                    if let ValueType::Const(s) = r.value {
                        func = Some(s);
                    }
                }
                "inst" => {
                    if let ValueType::Const(s) = r.value {
                        inst = Some(s);
                    }
                }
                "offset" => {
                    if let ValueType::Const(s) = r.value {
                        offset = s.parse::<u64>().ok();
                    }
                }
                _ => {}
            });
        }
        _ => {}
    }
    match (addr, func, offset, inst) {
        (Some(addr), Some(func), Some(offset), Some(inst)) => {
            let dfl = DisassembleFunctionLine {
                address: addr,
                offset,
                inst,
            };
            Some((func, dfl))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn f_get_disassemble_function_line() {
        use crate::mi::disassemble::*;
        let tuple = ValueType::Tuple(Tuple::Results(vec![
            ResultType {
                variable: "address".to_string(),
                value: ValueType::Const("0x0000000000400b10".to_string()),
            },
            ResultType {
                variable: "func-name".to_string(),
                value: ValueType::Const("main".to_string()),
            },
            ResultType {
                variable: "inst".to_string(),
                value: ValueType::Const("mov eax, 0x0".to_string()),
            },
            ResultType {
                variable: "offset".to_string(),
                value: ValueType::Const("0".to_string()),
            },
        ]));
        let (func, dfl) = get_disassemble_function_line(tuple).unwrap();
        assert_eq!(func, "main");
        assert_eq!(dfl.address, "0x0000000000400b10");
        assert_eq!(dfl.offset, 0);
        assert_eq!(dfl.inst, "mov eax, 0x0");
    }

    #[test]
    fn f_get_disassemble_function() {
        use crate::mi::disassemble::*;
        use crate::mi::miout;
        let a = miout::TokOutputOnelineParser::new().parse("^done,asm_insns=[{address=\"0x00005555555865f0\",func-name=\"main\",offset=\"0\",inst=\"endbr64\"},{address=\"0x00005555555865f4\",func-name=\"main\",offset=\"4\",inst=\"push   %rbp\"},{address=\"0x000055555558834c\",func-name=\"main\",offset=\"7516\",inst=\"mov    %r15,%rcx\"},{address=\"0x000055555558834f\",func-name=\"main\",offset=\"7519\",inst=\"jmp    0x555555587c99 <main+5801>\"}]\n" );
        println!("{:?}", &a);
        let a = a.unwrap();
        match a {
            OutputOneline::ResultRecord(r) => {
                let df = get_disassemble_function(r).unwrap();
                println!("df is {:?}", &df);
                assert!(
                    df == DisassembleFunction {
                        func: "main".to_string(),
                        insts: vec![
                            DisassembleFunctionLine {
                                address: "0x00005555555865f0".to_string(),
                                offset: 0,
                                inst: "endbr64".to_string()
                            },
                            DisassembleFunctionLine {
                                address: "0x00005555555865f4".to_string(),
                                offset: 4,
                                inst: "push   %rbp".to_string()
                            },
                            DisassembleFunctionLine {
                                address: "0x000055555558834c".to_string(),
                                offset: 7516,
                                inst: "mov    %r15,%rcx".to_string()
                            },
                            DisassembleFunctionLine {
                                address: "0x000055555558834f".to_string(),
                                offset: 7519,
                                inst: "jmp    0x555555587c99 <main+5801>".to_string()
                            }
                        ]
                    }
                );
            }
            _ => panic!(),
        }
    }
}
