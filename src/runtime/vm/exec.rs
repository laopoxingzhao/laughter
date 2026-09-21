//! 主解释循环：一步一步执行字节码。
//!
//! 每次循环做四件事：
//! 1. 取当前帧（正在跑的函数 + 指令位置 ip + 局部槽基址 base）
//! 2. 读 `code[ip]` 这个字节，翻译成操作码 `Op`
//! 3. 按操作码改栈 / 改帧（见各分支中文注释）
//! 4. 把 `ip` 推进到下一条指令
//!
//! 帧栈空了 → 程序结束，返回 `print` 收集到的行。

use super::*;

impl<'m> Vm<'m> {
    /// 通过指针读取：nil 报错；Local 帧仍在则读栈槽；ArrayEl 读数组。
    fn deref_read(&self, p: &Value, line: u32) -> Result<Value, VmError> {
        use crate::runtime::value::Ptr;
        let Value::Ptr(p) = p else {
            return Err(VmError {
                message: format!("`*` 需要指针，实际是 {}", p.type_name()),
                line,
            });
        };
        match p {
            Ptr::Nil => Err(VmError {
                message: "解引用空指针".into(),
                line,
            }),
            Ptr::Local { frame, slot } => {
                if *frame >= self.frames.len() {
                    return Err(VmError {
                        message: "悬垂指针：函数帧已结束".into(),
                        line,
                    });
                }
                let base = self.frames[*frame].base;
                self.stack.get(base + slot).cloned().ok_or_else(|| VmError {
                    message: "指针槽无效".into(),
                    line,
                })
            }
            Ptr::ArrayEl { arr, index } => {
                let b = arr.borrow();
                if *index < 0 || *index as usize >= b.len() {
                    return Err(VmError {
                        message: format!("指针下标 {index} 越界"),
                        line,
                    });
                }
                Ok(b[*index as usize].clone())
            }
        }
    }

    /// 通过指针写入。
    fn deref_write(&mut self, p: &Value, val: Value, line: u32) -> Result<(), VmError> {
        use crate::runtime::value::Ptr;
        let Value::Ptr(p) = p else {
            return Err(VmError {
                message: format!("解引用赋值需要指针，实际是 {}", p.type_name()),
                line,
            });
        };
        match p {
            Ptr::Nil => Err(VmError {
                message: "解引用空指针（写）".into(),
                line,
            }),
            Ptr::Local { frame, slot } => {
                if *frame >= self.frames.len() {
                    return Err(VmError {
                        message: "悬垂指针：函数帧已结束".into(),
                        line,
                    });
                }
                let base = self.frames[*frame].base;
                let addr = base + slot;
                if addr >= self.stack.len() {
                    return Err(VmError {
                        message: "指针槽无效".into(),
                        line,
                    });
                }
                self.stack[addr] = val;
                Ok(())
            }
            Ptr::ArrayEl { arr, index } => {
                let mut b = arr.borrow_mut();
                if *index < 0 || *index as usize >= b.len() {
                    return Err(VmError {
                        message: format!("指针下标 {index} 越界"),
                        line,
                    });
                }
                b[*index as usize] = val;
                Ok(())
            }
        }
    }
    pub(crate) fn loop_run(&mut self, out: &mut Vec<String>) -> Result<Vec<String>, VmError> {
        loop {
            // 步骤1：没有帧 = 主程序已返回
            let Some(fr) = self.frames.last() else {
                return Ok(std::mem::take(out));
            };
            // 步骤2：读出当前执行位置（函数编号、指令指针、局部基址）
            let func = fr.func;
            let ip = fr.ip;
            let base = fr.base;
            let code_len = self.module.functions[func].chunk.code.len();
            if ip >= code_len {
                return Err(VmError {
                    message: "指令指针越界".into(),
                    line: 0,
                });
            }
            // 步骤3：取出操作码字节，并查源码行号（报错用）
            let byte = self.module.functions[func].chunk.code[ip];
            let line = self.line();
            let Some(op) = Op::from_u8(byte) else {
                return Err(VmError {
                    message: format!("未知操作码 {byte}"),
                    line,
                });
            };
            // 步骤4：按操作码分支执行（下面每个分支就是「这一步该做什么」）
            match op {
                // Const <u16>：从常量池取出第 i 项，压入栈
                Op::Const => {
                    let i = self.u16(ip + 1)? as usize;
                    let v = self.module.functions[func].chunk.constants[i].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // True：把布尔 true 压栈
                Op::True => {
                    self.stack.push(Value::Bool(true));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // False：把布尔 false 压栈
                Op::False => {
                    self.stack.push(Value::Bool(false));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // Pop：丢弃栈顶（表达式算完后清掉临时值）
                Op::Pop => {
                    self.pop(line)?;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // GetLocal <槽号>：复制局部变量到栈顶（槽里原值还在）
                Op::GetLocal => {
                    let s = self.u16(ip + 1)? as usize;
                    let v = self.stack[base + s].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // SetLocal <槽号>：用栈顶的值覆盖局部槽（不弹栈，常与 Pop 连用）
                Op::SetLocal => {
                    let s = self.u16(ip + 1)? as usize;
                    let v = self.peek(line)?.clone();
                    self.stack[base + s] = v;
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // SetLocalField <槽号> <字段名常量>：弹出值，写入栈槽里结构体的该字段
                Op::SetLocalField => {
                    let slot = self.u16(ip + 1)? as usize;
                    let ci = self.u16(ip + 3)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "字段名常量无效".into(),
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
                                    message: format!("没有字段 `{fname}`"),
                                    line,
                                });
                            }
                        }
                        Some(other) => {
                            return Err(VmError {
                                message: format!("无法在 {} 上设置字段", other.type_name()),
                                line,
                            })
                        }
                        None => {
                            return Err(VmError {
                                message: "局部槽下标无效".into(),
                                line,
                            })
                        }
                    }
                    self.frames.last_mut().unwrap().ip = ip + 5;
                }
                // 算术：先弹右操作数 b，再弹左操作数 a，算完把结果压回
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.stack.push(arith(op, a, b, line)?);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // Neg：栈顶取负（-x）
                Op::Neg => {
                    let a = self.pop(line)?;
                    self.stack.push(match a {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(n) => Value::Float(-n),
                        o => {
                            return Err(VmError {
                                message: format!("无法对 {} 取负", o.type_name()),
                                line,
                            })
                        }
                    });
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // Not：栈顶逻辑非（!flag）
                Op::Not => {
                    let a = self.pop(line)?;
                    match a {
                        Value::Bool(b) => self.stack.push(Value::Bool(!b)),
                        o => {
                            return Err(VmError {
                                message: format!("无法对 {} 使用 `!`", o.type_name()),
                                line,
                            })
                        }
                    }
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // == / !=：比较两个栈顶值，结果为 bool
                Op::Eq | Op::Ne => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let eq = val_eq(&a, &b, line)?;
                    let r = if op == Op::Eq { eq } else { !eq };
                    self.stack.push(Value::Bool(r));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // < <= > >=：两个数比较，结果为 bool
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
                // Jump：无条件向前跳转（偏移量在操作数里）
                Op::Jump => {
                    let o = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3 + o;
                }
                // 条件跳转：只「看」栈顶 bool，不弹出；代码生成负责随后 Pop
                Op::JumpIfFalse | Op::JumpIfTrue => {
                    let o = self.u16(ip + 1)? as usize;
                    let b = match self.peek(line)? {
                        Value::Bool(b) => *b,
                        o => {
                            return Err(VmError {
                                message: format!("条件必须是 bool，实际是 {}", o.type_name()),
                                line,
                            })
                        }
                    };
                    let jump = if op == Op::JumpIfFalse { !b } else { b };
                    self.frames.last_mut().unwrap().ip = if jump { ip + 3 + o } else { ip + 3 };
                }
                // Loop：向后跳回循环头（while / for 脱糖后的回边）
                Op::Loop => {
                    let o = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3 - o;
                }
                // Call <函数下标>：进入被调函数（新压一帧）
                Op::Call => {
                    let t = self.u16(ip + 1)? as usize;
                    self.frames.last_mut().unwrap().ip = ip + 3;
                    self.push_frame(t)?;
                }
                // CallMethod <方法名> <argc>：按接收者运行时类型名查找 Type.method 并调用
                Op::CallMethod => {
                    let ni = self.u16(ip + 1)? as usize;
                    let argc = self.u16(ip + 3)? as usize;
                    let mname = match &self.module.functions[func].chunk.constants[ni] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "方法名常量无效".into(),
                                line,
                            })
                        }
                    };
                    if self.stack.len() < argc + 1 {
                        return Err(VmError {
                            message: "方法调用栈下溢".into(),
                            line,
                        });
                    }
                    let recv_i = self.stack.len() - argc - 1;
                    let tname = match &self.stack[recv_i] {
                        Value::Struct(s) => s.name.clone(),
                        _ => {
                            return Err(VmError {
                                message: "方法接收者不是结构体".into(),
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
                            message: format!("未定义方法 `{full}`"),
                            line,
                        })?;
                    self.frames.last_mut().unwrap().ip = ip + 5;
                    self.push_frame(t)?;
                }
                // Return：弹出当前帧
                //   void：截断栈到 base（清掉参数/局部）
                //   非void：先拿返回值，截断后再压回去（供调用方使用）
                Op::Return => {
                    let fr = self.frames.pop().unwrap();
                    let f = &self.module.functions[fr.func];
                    if !f.is_void {
                        if self.stack.len() <= fr.base {
                            return Err(VmError {
                                message: format!("函数 `{}` 缺少返回值", f.name),
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
                // NewArray <n>：弹出 n 个元素，包成数组句柄压栈
                Op::NewArray => {
                    let n = self.u16(ip + 1)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "数组元素缺失".into(),
                            line,
                        });
                    }
                    let st = self.stack.len() - n;
                    let items = self.stack.split_off(st);
                    let h: ArrayHandle = Rc::new(RefCell::new(items));
                    self.stack.push(Value::Array(h));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // GetIndex：栈 [数组, 下标] → 弹出后把元素压栈（越界报错）
                Op::GetIndex => {
                    let i = self.pop(line)?;
                    let a = self.pop(line)?;
                    let idx = as_idx(&i, line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("无法对 {} 做下标访问", a.type_name()),
                            line,
                        });
                    };
                    let b = h.borrow();
                    if idx < 0 || idx as usize >= b.len() {
                        return Err(VmError {
                            message: format!("数组下标 {idx} 越界（长度 {}）", b.len()),
                            line,
                        });
                    }
                    let v = b[idx as usize].clone();
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // SetIndex：栈 [数组, 下标, 新值] → 写回数组（引用语义，就地改）
                Op::SetIndex => {
                    let v = self.pop(line)?;
                    let i = self.pop(line)?;
                    let a = self.pop(line)?;
                    let idx = as_idx(&i, line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("无法对 {} 做下标访问", a.type_name()),
                            line,
                        });
                    };
                    let mut b = h.borrow_mut();
                    if idx < 0 || idx as usize >= b.len() {
                        return Err(VmError {
                            message: format!("数组下标 {idx} 越界（长度 {}）", b.len()),
                            line,
                        });
                    }
                    b[idx as usize] = v;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // NewStruct <类型下标> <字段数>：按声明顺序组装字段，压入结构体值
                Op::NewStruct => {
                    let ti = self.u16(ip + 1)? as usize;
                    let n = self.u16(ip + 3)? as usize;
                    if self.stack.len() < n {
                        return Err(VmError {
                            message: "结构体字段缺失".into(),
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
                // GetField <字段名>：弹出结构体，把字段值拷贝压栈
                Op::GetField => {
                    let ci = self.u16(ip + 1)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "字段名常量无效".into(),
                                line,
                            })
                        }
                    };
                    let o = self.pop(line)?;
                    let Value::Struct(s) = o else {
                        return Err(VmError {
                            message: format!("无法在 {} 上读取字段", o.type_name()),
                            line,
                        });
                    };
                    let v = s.get(&fname).cloned().ok_or_else(|| VmError {
                        message: format!("没有字段 `{fname}`"),
                        line,
                    })?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // SetField <字段名>：栈 [结构体, 新值] → 改字段后的结构体压回（值语义）
                Op::SetField => {
                    let ci = self.u16(ip + 1)? as usize;
                    let fname = match &self.module.functions[func].chunk.constants[ci] {
                        Value::Str(s) => s.to_string(),
                        _ => {
                            return Err(VmError {
                                message: "字段名常量无效".into(),
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
                            message: format!("没有字段 `{fname}`"),
                            line,
                        });
                    }
                    self.stack.push(Value::Struct(s));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                // Len：数组长度或字符串字符数
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
                // Print：弹出栈顶，把显示文本记入 out（CLI 再打印）
                Op::Print => {
                    let v = self.pop(line)?;
                    out.push(v.display());
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // push(arr, v)：弹出值和数组，追加到数组末尾
                Op::Push => {
                    let v = self.pop(line)?;
                    let a = self.pop(line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("`push` 需要数组，实际是 {}", a.type_name()),
                            line,
                        });
                    };
                    h.borrow_mut().push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // pop(arr)：弹出数组，取出末元素压栈；空数组报错
                Op::ArrayPop => {
                    let a = self.pop(line)?;
                    let Value::Array(h) = a else {
                        return Err(VmError {
                            message: format!("`pop` 需要数组，实际是 {}", a.type_name()),
                            line,
                        });
                    };
                    let v = h.borrow_mut().pop().ok_or_else(|| VmError {
                        message: "不能对空数组执行 pop".into(),
                        line,
                    })?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // input()：从标准输入读一行，压入 string（去掉换行）
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
                // str_at(s, i)：取第 i 个字符；越界报错
                Op::StrAt => {
                    let i = self.pop(line)?;
                    let s = self.pop(line)?;
                    let (Value::Str(s), Value::Int(i)) = (&s, &i) else {
                        return Err(VmError {
                            message: "`str_at` 需要 (string, int)".into(),
                            line,
                        });
                    };
                    let ch = s.chars().nth(*i as usize).ok_or_else(|| VmError {
                        message: format!("`str_at` 下标 {i} 越界"),
                        line,
                    })?;
                    self.stack
                        .push(Value::Str(Rc::from(ch.to_string().as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                // str_sub(s, start, n)：子串；越界报错
                Op::StrSub => {
                    let n = self.pop(line)?;
                    let st = self.pop(line)?;
                    let s = self.pop(line)?;
                    let (Value::Str(s), Value::Int(st), Value::Int(n)) = (&s, &st, &n) else {
                        return Err(VmError {
                            message: "`str_sub` 需要 (string, int, int)".into(),
                            line,
                        });
                    };
                    if *st < 0 || *n < 0 {
                        return Err(VmError {
                            message: "`str_sub` 下标不能为负".into(),
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
                // to_string(v)：把任意可打印值转成字符串显示
                Op::ToString => {
                    let v = self.pop(line)?;
                    let s = v.display();
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Nil => {
                    self.stack.push(Value::Ptr(crate::runtime::value::Ptr::Nil));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::RefLocal | Op::RefMutLocal => {
                    let slot = self.u16(ip + 1)? as usize;
                    let frame_idx = self.frames.len() - 1;
                    self.stack
                        .push(Value::Ptr(crate::runtime::value::Ptr::Local {
                            frame: frame_idx,
                            slot,
                        }));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::DerefRead => {
                    let p = self.pop(line)?;
                    let v = self.deref_read(&p, line)?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::DerefWrite => {
                    let val = self.pop(line)?;
                    let p = self.pop(line)?;
                    self.deref_write(&p, val, line)?;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::PtrEq => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let eq = match (&a, &b) {
                        (Value::Ptr(p), Value::Ptr(q)) => match (p, q) {
                            (crate::runtime::value::Ptr::Nil, crate::runtime::value::Ptr::Nil) => {
                                true
                            }
                            (
                                crate::runtime::value::Ptr::Local {
                                    frame: f1,
                                    slot: s1,
                                },
                                crate::runtime::value::Ptr::Local {
                                    frame: f2,
                                    slot: s2,
                                },
                            ) => f1 == f2 && s1 == s2,
                            (
                                crate::runtime::value::Ptr::ArrayEl { index: i1, .. },
                                crate::runtime::value::Ptr::ArrayEl { index: i2, .. },
                            ) => i1 == i2,
                            _ => false,
                        },
                        _ => false,
                    };
                    self.stack.push(Value::Bool(eq));
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
            message: format!("下标必须是 int，实际是 {}", o.type_name()),
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
                    message: "除数不能为零".into(),
                    line,
                }),
                _ => Err(VmError {
                    message: "非法的整数运算".into(),
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
                    message: "除数不能为零".into(),
                    line,
                }),
                _ => Err(VmError {
                    message: "非法的浮点运算".into(),
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
    use crate::runtime::value::Ptr;
    Ok(match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Ptr(p), Value::Ptr(q)) => match (p, q) {
            (Ptr::Nil, Ptr::Nil) => true,
            (
                Ptr::Local {
                    frame: f1,
                    slot: s1,
                },
                Ptr::Local {
                    frame: f2,
                    slot: s2,
                },
            ) => f1 == f2 && s1 == s2,
            (Ptr::ArrayEl { index: i1, .. }, Ptr::ArrayEl { index: i2, .. }) => i1 == i2,
            _ => false,
        },
        _ => {
            return Err(VmError {
                message: format!("无法比较 {} 与 {}", a.type_name(), b.type_name()),
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
                message: "无法比较 NaN".into(),
                line,
            })?
        }
        _ => {
            return Err(VmError {
                message: format!("无法比较大小 {} 与 {}", a.type_name(), b.type_name()),
                line,
            })
        }
    })
}
