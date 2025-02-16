// use bytes;
use crate::mi::token::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub addr: String,
    pub func: Option<String>,
    pub fullname: Option<String>,
    pub line: Option<u64>,
}

impl TryFrom<&ResultType> for Frame {
    type Error = String;

    fn try_from(r: &ResultType) -> Result<Self, Self::Error> {
        let mut ans = Self {
            addr: "".to_string(),
            func: None,
            fullname: None,
            line: None,
        };
        if r.variable.as_str() == "frame" {
            if let ValueType::Tuple(Tuple::Results(rs)) = &r.value {
                rs.iter().for_each(|r| match r.variable.as_str() {
                    "fullname" => {
                        if let ValueType::Const(f) = &r.value {
                            ans.fullname = Some(f.clone())
                        }
                    }
                    "line" => {
                        if let ValueType::Const(l) = &r.value {
                            if let std::result::Result::Ok(l) = l.parse::<u64>() {
                                ans.line = Some(l)
                            }
                        }
                    }
                    "addr" => {
                        if let ValueType::Const(f) = &r.value {
                            ans.addr = f.clone()
                        }
                    }
                    "func" => {
                        if let ValueType::Const(f) = &r.value {
                            ans.func = Some(f.clone())
                        }
                    }
                    _ => {}
                });
                Ok(ans)
            } else {
                Err("not frame".to_string())
            }
        } else {
            Err("not frame".to_string())
        }
    }
}
