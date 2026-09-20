//! 主解释循环。
use super::*;

impl<'m> Vm<'m> {
    pub(crate) fn loop_run(&mut self, out: &mut Vec<String>) -> Result<Vec<String>, VmError> {
        loop {
            let Some(fr) = self.frames.last() else {
                return Ok(std::mem::take(out));
            };
            let func = fr.func;
            let ip = fr.ip;
            let base = fr.base;
            let code_len = self.module.functions[func].chunk.code.len();
            if ip >= code_len {
                return Err(VmError {
                    message: "ip out of range".into(),
                    line: 0,
                });
            }
            let byte = self.module.functions[func].chunk.code[ip];
            let line = self.line();
            let Some(op) = Op::from_u8(byte) else {
                return Err(VmError {
                    message: format!("bad opcode {byte}"),
                    line,
                });
            };
            match op {
                Op::Const => {
                    let i = self.u16(ip + 1)? as usize;
                    let v = self.module.functions[func].chunk.constants[i].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::True => {
                    self.stack.push(Value::Bool(true));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::False => {
                    self.stack.push(Value::Bool(false));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Pop => {
                    self.pop(line)?;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::GetLocal => {
                    let s = self.u16(ip + 1)? as usize;
                    let v = self.stack[base + s].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::SetLocal => {
                    let s = self.u16(ip + 1)? as usize;
                    let v = self.peek(line)?.clone();
                    self.stack[base + s] = v;
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::SetLocalField => {
                    let slot = self.u16(ip + 1)? as usize;
                    let ci = self.u16(ip + 3)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "bad field constant".into(),
                                line,
                            })
                        }
                    };
                    let val = self.pop(line)?;
                    let addr = base + slot;
                    match self.stack.get_mut(addr) {
                        Some(Value::Struct(s)) => {
                            if !s.set(&fname, val) {
                                return Err(VmError {
                                    message: format!("no field `{fname}`"),
                                    line,
                                });
                            }
                        }
                        Some(other) => {
                            return Err(VmError {
                                message: format!("cannot set field on {}", other.type_name()),
                                line,
                            })
                        }
                        None => {
                            return Err(VmError {
                                message: "bad local slot".into(),
                                line,
                            })
                        }
                    }
                    self.frames.last_mut().unwrap().ip = ip + 5;
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.stack.push(arith(op, a, b, line)?);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Neg => {
                    let a = self.pop(line)?;
                    self.stack.push(match a {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(n) => Value::Float(-n),
                        o => {
                            return Err(VmError {
                                message: format!("cannot negate {}", o.type_name()),
                                line,
                            })
                        }
                    });
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Not => {
                    let a = self.pop(line)?;
                    match a {
                        Value::Bool(b) => self.stack.push(Value::Bool(!b)),
                        o => {
                            return Err(VmError {
                                message: format!("cannot apply `!` to {}", o.type_name()),
                                line,
                            })
                        }
                    }
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Eq | Op::Ne => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let eq = val_eq(&a, &b, line)?;
                    let r = if op == Op::Eq { eq } else { !eq };
                    self.stack.push(Value::Bool(r));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let c = cmp(&a, &b, line)?;
                    let r = match op {
                        Op::Lt => c < 0,
                        Op::Le => c <= 0,
                        Op::Gt => c > 0,
                        _ => c >= 0,
                    };
                    self.stack.push(Value::Bool(r));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Jump => {
                    let o = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3 + o;
                }
                Op::JumpIfFalse | Op::JumpIfTrue => {
                    let o = self.u16(ip + 1)? as usize;
                    let b = match self.peek(line)? {
                        Value::Bool(b) => *b,
                        o => {
                            return Err(VmError {
                                message: format!("condition must be bool, got {}", o.type_name()),
                                line,
                            })
                        }
                    };
                    let jump = if op == Op::JumpIfFalse { !b } else { b };
                    self.frames.last_mut().unwrap().ip = if jump { ip + 3 + o } else { ip + 3 };
                }
                Op::Loop => {
                    let o = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3 - o;
                }
                Op::Call => {
                    let t = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3;
                    self.push_frame(t)?;
                }
                Op::CallMethod => {
                    let ni = self.u16(ip + 1)? as usize;
                    let argc = self.u16(ip + 3)? as usize;
                    let mname = match &self.module.functions[func].chunk.constants[ni] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "bad method name".into(),
                                line,
                            })
                        }
                    };
                    if self.stack.len() < argc + 1 {
                        return Err(VmError {
                            message: "method call underflow".into(),
                            line,
                        });
                    }
                    let recv_i = self.stack.len() - argc - 1;
                    let tname = match &self.stack[recv_i] {
                        Value::Struct(s) => s.name.clone(),
                        _ => {
                            return Err(VmError {
                                message: "receiver is not a struct".into(),
                                line,
                            })
                        }
                    };
                    let full = format!("{tname}.{mname}");
                    let t = self
                        .module
                        .functions
                        .iter()
                        .position(|f| f.name == full)
                        .ok_or_else(|| VmError {
                            message: format!("undefined method `{full}`"),
                            line,
                        })?;
                    self.frames.last_mut().unwrap().ip = ip + 5;
                    self.push_frame(t)?;
                }
                Op::Return => {
                    let fr = self.frames.pop().unwrap();
                    let f = &self.module.functions[fr.func];
                    if !f.is_void {
                        if self.stack.len() <= fr.base {
                            return Err(VmError {
                                message: format!("`{}` returned without value", f.name),
                                line,
                            });
                        }
                        let r = self.stack.pop().unwrap();
                        self.stack.truncate(fr.base);
                        self.stack.push(r);
                    } else {
                        self.stack.truncate(fr.base);
                    }
                    if self.frames.is_empty() {
                        return Ok(std::mem::take(out));
                    }
                }
                Op::NewArray => {
                    let n = self.u16(ip + 1)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "missing array elements".into(),
                            line,
                        });
                    }
                    let st = self.stack.len() - n;
                    let items = self.stack.split_off(st);
                    let h: ArrayHandle = Rc::new(RefCell::new(items));
                    self.stack.push(Value::Array(h));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::GetIndex => {
                    let i = self.pop(line)?;
                    let a = self.pop(line)?;
                    let idx = as_idx(&i, line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("cannot index {}", a.type_name()),
                            line,
                        });
                    };
                    let b = h.borrow();
                    if idx < 0 || idx as usize >= b.len() {
                        return Err(VmError {
                            message: format!("array index {idx} out of bounds (len {})", b.len()),
                            line,
                        });
                    }
                    let v = b[idx as usize].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::SetIndex => {
                    let v = self.pop(line)?;
                    let i = self.pop(line)?;
                    let a = self.pop(line)?;
                    let idx = as_idx(&i, line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("cannot index {}", a.type_name()),
                            line,
                        });
                    };
                    let mut b = h.borrow_mut();
                    if idx < 0 || idx as usize >= b.len() {
                        return Err(VmError {
                            message: format!("array index {idx} out of bounds (len {})", b.len()),
                            line,
                        });
                    }
                    b[idx as usize] = v;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::NewStruct => {
                    let ti = self.u16(ip + 1)? as usize;
                    let n = self.u16(ip + 3)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "missing struct fields".into(),
                            line,
                        });
                    }
                    let st = self.module.struct_types[ti].clone();
                    let start = self.stack.len() - n;
                    let items = self.stack.split_off(start);
                    let fields = st.fields.iter().cloned().zip(items).collect();
                    self.stack.push(Value::Struct(StructVal {
                        name: st.name,
                        fields,
                    }));
                    self.frames.last_mut().unwrap().ip = ip + 5;
                }
                Op::GetField => {
                    let ci = self.u16(ip + 1)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "bad field constant".into(),
                                line,
                            })
                        }
                    };
                    let o = self.pop(line)?;
                    let Value::Struct(s) = o else {
                        return Err(VmError {
                            message: format!("cannot get field on {}", o.type_name()),
                            line,
                        });
                    };
                    let v = s.get(&fname).cloned().ok_or_else(|| VmError {
                        message: format!("no field `{fname}`"),
                        line,
                    })?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::SetField => {
                    let ci = self.u16(ip + 1)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "bad field constant".into(),
                                line,
                            })
                        }
                    };
                    let val = self.pop(line)?;
                    let o = self.pop(line)?;
                    let Value::Struct(mut s) = o else {
                        return Err(VmError {
                            message: format!("cannot set field on {}", o.type_name()),
                            line,
                        });
                    };
                    if !s.set(&fname, val) {
                        return Err(VmError {
                            message: format!("no field `{fname}`"),
                            line,
                        });
                    }
                    self.stack.push(Value::Struct(s));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::Len => {
                    let a = self.pop(line)?;
                    let n = match &a {
                        Value::Array(h) => h.borrow().len() as i64,
                        Value::Str(s) => s.chars().count() as i64,
                        o => {
                            return Err(VmError {
                                message: format!(
                                    "`len` expects array/string, got {}",
                                    o.type_name()
                                ),
                                line,
                            })
                        }
                    };
                    self.stack.push(Value::Int(n));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Print => {
                    let v = self.pop(line)?;
                    out.push(v.display());
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Push => {
                    let v = self.pop(line)?;
                    let a = self.pop(line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("`push` expects array, got {}", a.type_name()),
                            line,
                        });
                    };
                    h.borrow_mut().push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::ArrayPop => {
                    let a = self.pop(line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("`pop` expects array, got {}", a.type_name()),
                            line,
                        });
                    };
                    let v = h.borrow_mut().pop().ok_or_else(|| VmError {
                        message: "pop from empty array".into(),
                        line,
                    })?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Input => {
                    use std::io::Write;
                    let mut s = String::new();
                    std::io::stdout().flush().ok();
                    let _ = std::io::stdin().read_line(&mut s);
                    while s.ends_with('\n') || s.ends_with('\r') {
                        s.pop();
                    }
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::StrAt => {
                    let i = self.pop(line)?;
                    let s = self.pop(line)?;
                    let (Value::Str(s), Value::Int(i)) = (&s, &i) else {
                        return Err(VmError {
                            message: "str_at expects (string, int)".into(),
                            line,
                        });
                    };
                    let ch = s.chars().nth(*i as usize).ok_or_else(|| VmError {
                        message: format!("str_at index {i} out of bounds"),
                        line,
                    })?;
                    self.stack
                        .push(Value::Str(Rc::from(ch.to_string().as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::StrSub => {
                    let n = self.pop(line)?;
                    let st = self.pop(line)?;
                    let s = self.pop(line)?;
                    let (Value::Str(s), Value::Int(st), Value::Int(n)) = (&s, &st, &n) else {
                        return Err(VmError {
                            message: "str_sub expects (string, int, int)".into(),
                            line,
                        });
                    };
                    if *st < 0 || *n < 0 {
                        return Err(VmError {
                            message: "str_sub negative index".into(),
                            line,
                        });
                    }
                    let cs: Vec<char> = s.chars().collect();
                    let (st, n) = (*st as usize, *n as usize);
                    if st + n > cs.len() {
                        return Err(VmError {
                            message: format!(
                                "str_sub out of bounds (len {}, start {st}, n {n})",
                                cs.len()
                            ),
                            line,
                        });
                    }
                    let sub: String = cs[st..st + n].iter().collect();
                    self.stack.push(Value::Str(Rc::from(sub.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::ToString => {
                    let v = self.pop(line)?;
                    let s = v.display();
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
            }
        }
    }
}

fn as_idx(v: &Value, line: u32) -> Result<i64, VmError> {
    match v {
        Value::Int(n) => Ok(*n),
        o => Err(VmError {
            message: format!("index must be int, got {}", o.type_name()),
            line,
        }),
    }
}

fn arith(op: Op, a: Value, b: Value, line: u32) -> Result<Value, VmError> {
    match (&a, &b) {
        (Value::Str(x), Value::Str(y)) if op == Op::Add => {
            let mut s = x.to_string();
            s.push_str(y);
            Ok(Value::Str(Rc::from(s.as_str())))
        }
        (Value::Int(x), Value::Int(y)) => {
            let (x, y) = (*x, *y);
            match op {
                Op::Add => Ok(Value::Int(x.wrapping_add(y))),
                Op::Sub => Ok(Value::Int(x.wrapping_sub(y))),
                Op::Mul => Ok(Value::Int(x.wrapping_mul(y))),
                Op::Div if y != 0 => Ok(Value::Int(x.wrapping_div(y))),
                Op::Rem if y != 0 => Ok(Value::Int(x.wrapping_rem(y))),
                Op::Div | Op::Rem => Err(VmError {
                    message: "division by zero".into(),
                    line,
                }),
                _ => Err(VmError {
                    message: "bad int op".into(),
                    line,
                }),
            }
        }
        (Value::Float(x), Value::Float(y)) => {
            let (x, y) = (*x, *y);
            match op {
                Op::Add => Ok(Value::Float(x + y)),
                Op::Sub => Ok(Value::Float(x - y)),
                Op::Mul => Ok(Value::Float(x * y)),
                Op::Div if y != 0.0 => Ok(Value::Float(x / y)),
                Op::Rem if y != 0.0 => Ok(Value::Float(x % y)),
                Op::Div | Op::Rem => Err(VmError {
                    message: "division by zero".into(),
                    line,
                }),
                _ => Err(VmError {
                    message: "bad float op".into(),
                    line,
                }),
            }
        }
        _ => Err(VmError {
            message: format!(
                "cannot apply arithmetic to {} and {}",
                a.type_name(),
                b.type_name()
            ),
            line,
        }),
    }
}

fn val_eq(a: &Value, b: &Value, line: u32) -> Result<bool, VmError> {
    Ok(match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => {
            return Err(VmError {
                message: format!("cannot compare {} and {}", a.type_name(), b.type_name()),
                line,
            })
        }
    })
}

fn cmp(a: &Value, b: &Value, line: u32) -> Result<i32, VmError> {
    Ok(match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y) as i32,
        (Value::Float(x), Value::Float(y)) => {
            x.partial_cmp(y).map(|o| o as i32).ok_or_else(|| VmError {
                message: "cannot compare NaN".into(),
                line,
            })?
        }
        _ => {
            return Err(VmError {
                message: format!("cannot order {} and {}", a.type_name(), b.type_name()),
                line,
            })
        }
    })
}
