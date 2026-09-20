use std::cell::RefCell;
use std::rc::Rc;

use crate::bytecode::{op_from_u8, Module, Op};
use crate::value::{ArrayHandle, Value};

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
        // Args already sit on the stack at base..base+arity; further locals are
        // pushed by compiled `let` instructions. Do not pre-pad slots.
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
        // transfer ownership of output at end — use &mut throughout instead
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
                    // Discard the callee frame (args + locals + temps) and, for
                    // non-void functions, leave the return value on the stack.
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
                    let Value::Array(h) = arr else {
                        return Err(VmError {
                            message: format!("`len` expects array, got {}", arr.type_name()),
                            line,
                        });
                    };
                    let n = h.borrow().len() as i64;
                    self.stack.push(Value::Int(n));
                    self.set_ip(ip + 1);
                }
                Op::Print => {
                    let v = self.pop(line)?;
                    output.push(v.display());
                    self.set_ip(ip + 1);
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
        (Value::Float(x), Value::Float(y)) => x
            .partial_cmp(y)
            .map(|o| o as i32)
            .ok_or_else(|| VmError {
                message: "cannot compare NaN".into(),
                line,
            })?,
        _ => {
            return Err(VmError {
                message: format!("cannot order {} and {}", a.type_name(), b.type_name()),
                line,
            })
        }
    })
}

/// Compile + run source, returning printed lines. Used by tests and CLI.
/// `file` is used in diagnostics as `file:line:col: error: ...`.
pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    use crate::compiler::Compiler;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Checker;

    let tokens = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    Checker::new(&program)
        .check()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    let module = Compiler::compile(&program)
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
    run_source_file("<input>", src)
}

pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::compiler::Compiler;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Checker;

    let tokens = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    Checker::new(&program)
        .check()
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.span.line, e.span.col, e.message))?;
    Compiler::compile(&program)
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.line, e.col, e.message))
}

pub fn compile_source(src: &str) -> Result<Module, String> {
    compile_source_file("<input>", src)
}
