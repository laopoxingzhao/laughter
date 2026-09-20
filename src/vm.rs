//! 栈式虚拟机：解释 `Module` 中的字节码。
//!
//! 帧约定：参数已位于 `stack[base..base+arity]`；`let` 继续向栈上压局部槽。
//! `Return` 时把整帧（参数+局部+临时）truncate 到 `base`，非 void 再压回返回值。

use std::cell::RefCell;
use std::rc::Rc;

use crate::bytecode::{op_from_u8, Module, Op};
use crate::value::{ArrayHandle, StructVal, Value};

pub struct VmError {
    pub message: String,
    pub line: u32,
}

struct Frame {
    func: usize,
    ip: usize,
    base: usize,
}

pub struct Vm<'m> {
    module: &'m Module,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    max_depth: usize,
}

impl<'m> Vm<'m> {
    pub fn new(module: &'m Module) -> Self {
        Self {
            module,
            stack: Vec::with_capacity(256),
            frames: Vec::new(),
            max_depth: 1024,
        }
    }

    pub fn run(&mut self) -> Result<Vec<String>, VmError> {
        let entry = self.module.main_index.unwrap_or(self.module.toplevel_index);
        self.push_frame(entry)?;
        let mut output = Vec::new();
        self.run_loop(&mut output)
    }

    fn push_frame(&mut self, func: usize) -> Result<(), VmError> {
        if self.frames.len() >= self.max_depth {
            return Err(VmError {
                message: format!("stack overflow: recursion deeper than {}", self.max_depth),
                line: 0,
            });
        }
        let f = &self.module.functions[func];
        let argc = f.arity as usize;
        if self.stack.len() < argc {
            return Err(VmError {
                message: "internal: missing arguments".into(),
                line: 0,
            });
        }
        // 参数已在栈上；后续局部由编译后的 `let` 压入。不要在这里 padding 槽位。
        let base = self.stack.len() - argc;
        self.frames.push(Frame { func, ip: 0, base });
        Ok(())
    }

    fn current_func(&self) -> usize {
        self.frames.last().expect("frame").func
    }

    fn current_ip(&self) -> usize {
        self.frames.last().expect("frame").ip
    }

    fn current_base(&self) -> usize {
        self.frames.last().expect("frame").base
    }

    fn set_ip(&mut self, ip: usize) {
        self.frames.last_mut().expect("frame").ip = ip;
    }

    fn read_u16(&self, at: usize) -> Result<u16, VmError> {
        let code = &self.module.functions[self.current_func()].chunk.code;
        if at + 1 >= code.len() {
            return Err(VmError {
                message: "truncated instruction".into(),
                line: 0,
            });
        }
        Ok(u16::from_le_bytes([code[at], code[at + 1]]))
    }

    fn line(&self) -> u32 {
        let func = self.current_func();
        let ip = self.current_ip();
        self.module.functions[func]
            .chunk
            .lines
            .get(ip)
            .copied()
            .unwrap_or(0)
    }

    fn peek(&self, line: u32) -> Result<&Value, VmError> {
        self.stack.last().ok_or_else(|| VmError {
            message: "stack underflow".into(),
            line,
        })
    }

    fn pop(&mut self, line: u32) -> Result<Value, VmError> {
        self.stack.pop().ok_or_else(|| VmError {
            message: "stack underflow".into(),
            line,
        })
    }

    fn run_loop(&mut self, output: &mut Vec<String>) -> Result<Vec<String>, VmError> {
        // 打印内容写入 output；帧清空时用 take 取走所有权并返回。
        loop {
            if self.frames.is_empty() {
                return Ok(std::mem::take(output));
            }
            let func = self.current_func();
            let ip = self.current_ip();
            let base = self.current_base();
            let code_len = self.module.functions[func].chunk.code.len();
            if ip >= code_len {
                return Err(VmError {
                    message: "internal: ip out of range".into(),
                    line: 0,
                });
            }
            let byte = self.module.functions[func].chunk.code[ip];
            let line = self.line();
            let Some(op) = op_from_u8(byte) else {
                return Err(VmError {
                    message: format!("unknown opcode {byte}"),
                    line,
                });
            };

            match op {
                Op::Const => {
                    let idx = self.read_u16(ip + 1)? as usize;
                    let v = self.module.functions[func].chunk.constants[idx].clone();
                    self.stack.push(v);
                    self.set_ip(ip + 3);
                }
                Op::True => {
                    self.stack.push(Value::Bool(true));
                    self.set_ip(ip + 1);
                }
                Op::False => {
                    self.stack.push(Value::Bool(false));
                    self.set_ip(ip + 1);
                }
                Op::Pop => {
                    self.pop(line)?;
                    self.set_ip(ip + 1);
                }
                Op::GetLocal => {
                    let slot = self.read_u16(ip + 1)? as usize;
                    let v = self.stack[base + slot].clone();
                    self.stack.push(v);
                    self.set_ip(ip + 3);
                }
                Op::SetLocal => {
                    let slot = self.read_u16(ip + 1)? as usize;
                    let v = self.peek(line)?.clone();
                    self.stack[base + slot] = v;
                    self.set_ip(ip + 3);
                }
                Op::GetGlobal | Op::SetGlobal => {
                    return Err(VmError {
                        message: "globals not used in this MVP".into(),
                        line,
                    });
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let v = bin_num(op, a, b, line)?;
                    self.stack.push(v);
                    self.set_ip(ip + 1);
                }
                Op::Neg => {
                    let a = self.pop(line)?;
                    let v = match a {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(n) => Value::Float(-n),
                        other => {
                            return Err(VmError {
                                message: format!("cannot negate {}", other.type_name()),
                                line,
                            })
                        }
                    };
                    self.stack.push(v);
                    self.set_ip(ip + 1);
                }
                Op::Not => {
                    let a = self.pop(line)?;
                    match a {
                        Value::Bool(b) => self.stack.push(Value::Bool(!b)),
                        other => {
                            return Err(VmError {
                                message: format!("cannot apply `!` to {}", other.type_name()),
                                line,
                            })
                        }
                    }
                    self.set_ip(ip + 1);
                }
                Op::Eq | Op::Ne => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let eq = values_eq(&a, &b, line)?;
                    let r = if op == Op::Eq { eq } else { !eq };
                    self.stack.push(Value::Bool(r));
                    self.set_ip(ip + 1);
                }
                Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let ord = compare(&a, &b, line)?;
                    let r = match op {
                        Op::Lt => ord < 0,
                        Op::Le => ord <= 0,
                        Op::Gt => ord > 0,
                        Op::Ge => ord >= 0,
                        _ => unreachable!(),
                    };
                    self.stack.push(Value::Bool(r));
                    self.set_ip(ip + 1);
                }
                Op::Jump => {
                    let offset = self.read_u16(ip + 1)? as usize;
                    self.set_ip(ip + 3 + offset);
                }
                Op::JumpIfFalse | Op::JumpIfTrue => {
                    let offset = self.read_u16(ip + 1)? as usize;
                    let cond = self.peek(line)?;
                    let b = match cond {
                        Value::Bool(b) => *b,
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "condition must be bool, got {}",
                                    other.type_name()
                                ),
                                line,
                            })
                        }
                    };
                    let jump = if op == Op::JumpIfFalse { !b } else { b };
                    if jump {
                        self.set_ip(ip + 3 + offset);
                    } else {
                        self.set_ip(ip + 3);
                    }
                }
                Op::Loop => {
                    let offset = self.read_u16(ip + 1)? as usize;
                    self.set_ip(ip + 3 - offset);
                }
                Op::Call => {
                    let target = self.read_u16(ip + 1)? as usize;
                    self.set_ip(ip + 3);
                    self.push_frame(target)?;
                }
                Op::Return => {
                    let frame = self.frames.pop().expect("frame");
                    let f = &self.module.functions[frame.func];
                    // 弹掉整个调用帧；void 不压值，非 void 在 base 处留下返回值。
                    if !f.is_void {
                        if self.stack.len() <= frame.base {
                            return Err(VmError {
                                message: format!("function `{}` returned without a value", f.name),
                                line,
                            });
                        }
                        let ret = self.stack.pop().expect("return value");
                        self.stack.truncate(frame.base);
                        self.stack.push(ret);
                    } else {
                        self.stack.truncate(frame.base);
                    }
                    if self.frames.is_empty() {
                        return Ok(std::mem::take(output));
                    }
                }
                Op::NewArray => {
                    let n = self.read_u16(ip + 1)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "missing array elements".into(),
                            line,
                        });
                    }
                    let start = self.stack.len() - n;
                    let items: Vec<Value> = self.stack.split_off(start);
                    let handle: ArrayHandle = Rc::new(RefCell::new(items));
                    self.stack.push(Value::Array(handle));
                    self.set_ip(ip + 3);
                }
                Op::GetIndex => {
                    let idx = self.pop(line)?;
                    let arr = self.pop(line)?;
                    let i = as_index(&idx, line)?;
                    let Value::Array(h) = arr else {
                        return Err(VmError {
                            message: format!("cannot index {}", arr.type_name()),
                            line,
                        });
                    };
                    let borrow = h.borrow();
                    if i < 0 || i as usize >= borrow.len() {
                        return Err(VmError {
                            message: format!(
                                "array index {i} out of bounds (len {})",
                                borrow.len()
                            ),
                            line,
                        });
                    }
                    let v = borrow[i as usize].clone();
                    self.stack.push(v);
                    self.set_ip(ip + 1);
                }
                Op::SetIndex => {
                    let val = self.pop(line)?;
                    let idx = self.pop(line)?;
                    let arr = self.pop(line)?;
                    let i = as_index(&idx, line)?;
                    let Value::Array(h) = arr else {
                        return Err(VmError {
                            message: format!("cannot index {}", arr.type_name()),
                            line,
                        });
                    };
                    let mut borrow = h.borrow_mut();
                    if i < 0 || i as usize >= borrow.len() {
                        return Err(VmError {
                            message: format!(
                                "array index {i} out of bounds (len {})",
                                borrow.len()
                            ),
                            line,
                        });
                    }
                    borrow[i as usize] = val;
                    self.set_ip(ip + 1);
                }
                Op::Len => {
                    let arr = self.pop(line)?;
                    let n = match &arr {
                        Value::Array(h) => h.borrow().len() as i64,
                        Value::Str(s) => s.chars().count() as i64,
                        other => {
                            return Err(VmError {
                                message: format!(
                                    "`len` expects array or string, got {}",
                                    other.type_name()
                                ),
                                line,
                            })
                        }
                    };
                    self.stack.push(Value::Int(n));
                    self.set_ip(ip + 1);
                }
                Op::Print => {
                    let v = self.pop(line)?;
                    output.push(v.display());
                    self.set_ip(ip + 1);
                }
                Op::NewStruct => {
                    let type_idx = self.read_u16(ip + 1)? as usize;
                    let n = self.read_u16(ip + 3)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "missing struct fields".into(),
                            line,
                        });
                    }
                    let st = self
                        .module
                        .struct_types
                        .get(type_idx)
                        .ok_or_else(|| VmError {
                            message: "bad struct type index".into(),
                            line,
                        })?;
                    let start = self.stack.len() - n;
                    let items: Vec<Value> = self.stack.split_off(start);
                    let fields: Vec<(String, Value)> =
                        st.fields.iter().cloned().zip(items).collect();
                    self.stack.push(Value::Struct(StructVal {
                        name: st.name.clone(),
                        fields,
                    }));
                    self.set_ip(ip + 5);
                }
                Op::GetField => {
                    let idx = self.read_u16(ip + 1)? as usize;
                    let name = match self.module.functions[func].chunk.constants.get(idx) {
                        Some(Value::Str(s)) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "field name constant missing".into(),
                                line,
                            })
                        }
                    };
                    let obj = self.pop(line)?;
                    let Value::Struct(s) = obj else {
                        return Err(VmError {
                            message: format!("cannot get field on {}", obj.type_name()),
                            line,
                        });
                    };
                    let v = s.get(&name).cloned().ok_or_else(|| VmError {
                        message: format!("no field `{name}`"),
                        line,
                    })?;
                    self.stack.push(v);
                    self.set_ip(ip + 3);
                }
                Op::SetField => {
                    let idx = self.read_u16(ip + 1)? as usize;
                    let name = match self.module.functions[func].chunk.constants.get(idx) {
                        Some(Value::Str(s)) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "field name constant missing".into(),
                                line,
                            })
                        }
                    };
                    let val = self.pop(line)?;
                    let obj = self.pop(line)?;
                    let Value::Struct(mut s) = obj else {
                        return Err(VmError {
                            message: format!("cannot set field on {}", obj.type_name()),
                            line,
                        });
                    };
                    if !s.set(&name, val) {
                        return Err(VmError {
                            message: format!("no field `{name}`"),
                            line,
                        });
                    }
                    self.stack.push(Value::Struct(s));
                    self.set_ip(ip + 3);
                }
                Op::SetLocalField => {
                    let slot = self.read_u16(ip + 1)? as usize;
                    let idx = self.read_u16(ip + 3)? as usize;
                    let name = match self.module.functions[func].chunk.constants.get(idx) {
                        Some(Value::Str(s)) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "field name constant missing".into(),
                                line,
                            })
                        }
                    };
                    let val = self.pop(line)?;
                    let addr = base + slot;
                    if addr >= self.stack.len() {
                        return Err(VmError {
                            message: "bad local slot".into(),
                            line,
                        });
                    }
                    match &mut self.stack[addr] {
                        Value::Struct(s) => {
                            if !s.set(&name, val) {
                                return Err(VmError {
                                    message: format!("no field `{name}`"),
                                    line,
                                });
                            }
                        }
                        other => {
                            return Err(VmError {
                                message: format!("cannot set field on {}", other.type_name()),
                                line,
                            })
                        }
                    }
                    self.set_ip(ip + 5);
                }
                Op::Push => {
                    let val = self.pop(line)?;
                    let arr = self.pop(line)?;
                    let Value::Array(h) = arr else {
                        return Err(VmError {
                            message: format!("`push` expects array, got {}", arr.type_name()),
                            line,
                        });
                    };
                    h.borrow_mut().push(val);
                    self.set_ip(ip + 1);
                }
                Op::ArrayPop => {
                    let arr = self.pop(line)?;
                    let Value::Array(h) = arr else {
                        return Err(VmError {
                            message: format!("`pop` expects array, got {}", arr.type_name()),
                            line,
                        });
                    };
                    let v = h.borrow_mut().pop().ok_or_else(|| VmError {
                        message: "pop from empty array".into(),
                        line,
                    })?;
                    self.stack.push(v);
                    self.set_ip(ip + 1);
                }
                Op::Input => {
                    use std::io::Write;
                    let mut line_in = String::new();
                    std::io::stdout().flush().ok();
                    match std::io::stdin().read_line(&mut line_in) {
                        Ok(_) => {
                            while line_in.ends_with('\n') || line_in.ends_with('\r') {
                                line_in.pop();
                            }
                            self.stack.push(Value::Str(Rc::from(line_in.as_str())));
                        }
                        Err(_) => self.stack.push(Value::Str(Rc::from(""))),
                    }
                    self.set_ip(ip + 1);
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
                    self.set_ip(ip + 1);
                }
                Op::StrSub => {
                    let n = self.pop(line)?;
                    let start = self.pop(line)?;
                    let s = self.pop(line)?;
                    let (Value::Str(s), Value::Int(start), Value::Int(n)) = (&s, &start, &n) else {
                        return Err(VmError {
                            message: "str_sub expects (string, int, int)".into(),
                            line,
                        });
                    };
                    if *start < 0 || *n < 0 {
                        return Err(VmError {
                            message: "str_sub negative index".into(),
                            line,
                        });
                    }
                    let chars: Vec<char> = s.chars().collect();
                    let start = *start as usize;
                    let n = *n as usize;
                    if start + n > chars.len() {
                        return Err(VmError {
                            message: format!(
                                "str_sub out of bounds (len {}, start {start}, n {n})",
                                chars.len()
                            ),
                            line,
                        });
                    }
                    let sub: String = chars[start..start + n].iter().collect();
                    self.stack.push(Value::Str(Rc::from(sub.as_str())));
                    self.set_ip(ip + 1);
                }
                Op::ToString => {
                    let v = self.pop(line)?;
                    let s = v.display();
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.set_ip(ip + 1);
                }
                Op::CallMethod => {
                    let name_idx = self.read_u16(ip + 1)? as usize;
                    let argc = self.read_u16(ip + 3)? as usize;
                    let method_name =
                        match self.module.functions[func].chunk.constants.get(name_idx) {
                            Some(Value::Str(s)) => s.to_string(),
                            _ => {
                                return Err(VmError {
                                    message: "method name constant missing".into(),
                                    line,
                                })
                            }
                        };
                    if self.stack.len() < argc + 1 {
                        return Err(VmError {
                            message: "stack underflow in method call".into(),
                            line,
                        });
                    }
                    let recv_idx = self.stack.len() - argc - 1;
                    let type_name = match &self.stack[recv_idx] {
                        Value::Struct(s) => s.name.clone(),
                        _ => {
                            return Err(VmError {
                                message: "method receiver is not a struct".into(),
                                line,
                            })
                        }
                    };
                    let full = format!("{type_name}.{method_name}");
                    let target = self
                        .module
                        .functions
                        .iter()
                        .position(|f| f.name == full)
                        .ok_or_else(|| VmError {
                            message: format!("undefined method `{full}`"),
                            line,
                        })?;
                    self.set_ip(ip + 5);
                    self.push_frame(target)?;
                }
            }
        }
    }
}

fn as_index(v: &Value, line: u32) -> Result<i64, VmError> {
    match v {
        Value::Int(n) => Ok(*n),
        other => Err(VmError {
            message: format!("index must be int, got {}", other.type_name()),
            line,
        }),
    }
}

fn bin_num(op: Op, a: Value, b: Value, line: u32) -> Result<Value, VmError> {
    match (&a, &b) {
        (Value::Str(x), Value::Str(y)) if op == Op::Add => {
            let mut s = String::from(&**x);
            s.push_str(&y);
            Ok(Value::Str(Rc::from(s.as_str())))
        }
        (Value::Int(x), Value::Int(y)) => {
            let (x, y) = (*x, *y);
            match op {
                Op::Add => Ok(Value::Int(x.wrapping_add(y))),
                Op::Sub => Ok(Value::Int(x.wrapping_sub(y))),
                Op::Mul => Ok(Value::Int(x.wrapping_mul(y))),
                Op::Div => {
                    if y == 0 {
                        Err(VmError {
                            message: "division by zero".into(),
                            line,
                        })
                    } else {
                        Ok(Value::Int(x.wrapping_div(y)))
                    }
                }
                Op::Rem => {
                    if y == 0 {
                        Err(VmError {
                            message: "remainder by zero".into(),
                            line,
                        })
                    } else {
                        Ok(Value::Int(x.wrapping_rem(y)))
                    }
                }
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
                Op::Div => {
                    if y == 0.0 {
                        Err(VmError {
                            message: "division by zero".into(),
                            line,
                        })
                    } else {
                        Ok(Value::Float(x / y))
                    }
                }
                Op::Rem => {
                    if y == 0.0 {
                        Err(VmError {
                            message: "remainder by zero".into(),
                            line,
                        })
                    } else {
                        Ok(Value::Float(x % y))
                    }
                }
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

fn values_eq(a: &Value, b: &Value, line: u32) -> Result<bool, VmError> {
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

fn compare(a: &Value, b: &Value, line: u32) -> Result<i32, VmError> {
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

/// 编译并执行 `src`，返回 `print` 输出的各行。
/// 诊断里的 `file` 会出现在 `file:line:col: error:` 中。
pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    use crate::compiler::Compiler;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Checker;

    let tokens = Lexer::new(src).tokenize().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let program = Parser::new(tokens).parse_program().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let module = Compiler::compile(&program, consts)
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.line, e.col, e.message))?;
    let mut vm = Vm::new(&module);
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{file}: runtime error: {}", e.message)
        } else {
            format!("{file}:{}: runtime error: {}", e.line, e.message)
        }
    })
}

pub fn run_source(src: &str) -> Result<Vec<String>, String> {
    // 含 import 时必须走文件加载器（相对路径解析）
    if src.contains("import ") {
        // 粗测：真正拒绝在 parse 后
    }
    let program = {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        let tokens = Lexer::new(src).tokenize().map_err(|e| {
            format!(
                "<input>:{}:{}: error: {}",
                e.span.line, e.span.col, e.message
            )
        })?;
        Parser::new(tokens).parse_program().map_err(|e| {
            format!(
                "<input>:{}:{}: error: {}",
                e.span.line, e.span.col, e.message
            )
        })?
    };
    if program.imports().next().is_some() {
        return Err(
            "<input>: error: `import` requires a file path — use `laughter run <file.lg>`".into(),
        );
    }
    use crate::compiler::Compiler;
    use crate::resolve::Checker;
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "<input>:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let module = Compiler::compile(&program, consts)
        .map_err(|e| format!("<input>:{}:{}: error: {}", e.line, e.col, e.message))?;
    let mut vm = Vm::new(&module);
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("<input>: runtime error: {}", e.message)
        } else {
            format!("<input>:{}: runtime error: {}", e.line, e.message)
        }
    })
}

pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::compiler::Compiler;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Checker;

    let tokens = Lexer::new(src).tokenize().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let program = Parser::new(tokens).parse_program().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Compiler::compile(&program, consts)
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.line, e.col, e.message))
}

pub fn compile_source(src: &str) -> Result<Module, String> {
    compile_source_file("<input>", src)
}
