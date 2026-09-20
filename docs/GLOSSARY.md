# 术语表（英文 ↔ 中文）

代码与反汇编里常出现英文。本表按「场景」给出中文含义。  
语言规则见 [LANGUAGE.md](LANGUAGE.md)；编译过程见 [COMPILE_PRIMER.md](COMPILE_PRIMER.md)。

---

## 1. 编译流程

| 英文 | 中文 |
|------|------|
| Lexer / Tokenizer | 词法分析器（切词） |
| Token | 词法单元（一个「词」） |
| Parser | 语法分析器（组装成语法树） |
| AST (Abstract Syntax Tree) | 抽象语法树 |
| Sema / Checker / Type checker | 语义/类型检查器 |
| Compiler / Codegen | 编译器 / 代码生成 |
| Bytecode | 字节码（给虚拟机的指令） |
| VM (Virtual Machine) | 虚拟机 |
| Interpreter | 解释器（逐步执行） |
| Frontend | 前端（词法+语法+语义） |
| Backend | 后端（生成字节码/机器码等） |
| Desugar | 脱糖（把语法糖改写成基础形式） |
| Scope | 作用域 |
| Span | 源位置（行列） |
| Diagnostic | 诊断（错误/警告信息） |

---

## 2. 语言里的类型与结构

| 英文 | 中文 | 源码写法 |
|------|------|----------|
| int | 整数 | `int` |
| float | 浮点数 | `float` |
| bool | 布尔 | `bool` |
| string | 字符串 | `string` |
| array | 数组 | `int[]` |
| struct | 结构体 | `struct Point { ... }` |
| void | 无返回值 | `-> void` |
| field | 字段 | `p.x` 里的 `x` |
| method | 方法 | `fun Point.sum(self: Point)` |
| receiver | 接收者 | 方法的第一个参数 `self` |
| literal | 字面量 | `1`、`true`、`"hi"` |
| identifier | 标识符 | 用户起的名字 |
| keyword | 关键字 | `let`、`if`、`fun` |
| builtin | 内建（自带函数） | `print`、`len`… |
| import | 导入其它文件 | `import "math.lg";` |
| alias | 别名 | `import "..." as math;` |

---

## 3. 源码里的 Rust 名（AST / 检查器）

| Rust 名 | 中文 |
|---------|------|
| `Span` | 位置（行列） |
| `Token` / `TokenKind` | 词 / 词的种类 |
| `Ident` | 标识符 |
| `Expr` | 表达式 |
| `Stmt` | 语句 |
| `Block` | 语句块 `{ }` |
| `TypeExpr` | 源码里的类型标注 |
| `Type` | 检查器使用的语义类型 |
| `Value` | 运行时值 |
| `FunDecl` | 函数声明 |
| `StructDecl` | 结构体声明 |
| `ConstDecl` | 常量声明 |
| `Program` | 整个语法树 |
| `Checker` | 类型检查器 |
| `Compiler` | 字节码编译器 |
| `Module` | 编译结果（函数集合） |
| `Chunk` | 一段字节码 |
| `Op` | 操作码 |
| `Vm` / `Frame` | 虚拟机 / 调用帧 |
| `ip` | 指令指针（下一条指令下标） |
| `base` | 帧基址（局部变量起点） |
| `Rc` | 引用计数指针（共享数据） |
| `RefCell` | 运行时可变容器 |
| `Option` | 可选：`Some` / `None` |
| `Result` | 结果：`Ok` / `Err` |
| `pub` | 公开，可被其它模块使用 |
| `crate` | 当前这个库 |
| `mut` | 可变 |
| `unwrap` | 直接取出（出错会崩，测试里常见） |

---

## 4. 字节码指令名（disasm 里会看到）

| 指令 | 中文 |
|------|------|
| Const | 压入常量 |
| True / False | 压入真 / 假 |
| Pop | 弹出栈顶 |
| GetLocal / SetLocal | 读 / 写局部槽 |
| SetLocalField | 修改槽内结构体字段 |
| Add Sub Mul Div Rem | 加减乘除取余 |
| Neg / Not | 取负 / 逻辑非 |
| Eq Ne Lt Le Gt Ge | 相等/不等/小于/≤/大于/≥ |
| Jump | 无条件跳转 |
| JumpIfFalse / JumpIfTrue | 条件跳转（只查看栈顶） |
| Loop | 跳回循环头 |
| Call / CallMethod | 调用函数 / 方法 |
| Return | 返回 |
| NewArray / GetIndex / SetIndex | 建数组 / 读下标 / 写下标 |
| NewStruct / GetField / SetField | 建结构体 / 读字段 / 写字段 |
| Len | 长度 |
| Print | 打印 |
| Push / ArrayPop | 数组追加 / 弹出末元素 |
| Input | 读一行输入 |
| StrAt / StrSub / ToString | 取字符 / 子串 / 转字符串 |

---

## 5. 栈与执行

| 英文 | 中文 |
|------|------|
| stack | 栈（后进先出） |
| stack overflow | 栈溢出（如递归太深） |
| stack underflow | 栈下溢（弹空了） |
| slot | 槽（局部变量占用的栈位） |
| frame / call frame | 调用帧（一次函数调用的现场） |
| recursion | 递归 |
| short-circuit | 短路求值（`&&`、`\|\|`） |
| value semantics | 值语义（赋值即拷贝） |
| reference semantics | 引用语义（共享同一份数据） |
| constant folding | 常量折叠（编译期算常量） |

---

## 6. 文件与工具

| 英文 | 中文 |
|------|------|
| CLI (Command Line Interface) | 命令行工具 |
| Cargo | Rust 的构建/包管理工具 |
| crate | Rust 的包/库单位 |
| package | 包 |
| manifest | 清单（包描述文件） |
| bytecode file `.lgb` | 字节码文件 |
| lgpack | 类 JAR 的程序包（ZIP） |
| disassemble / disasm | 反汇编（把字节码打成可读文本） |
| roundtrip | 往返（编码再解码后应一致） |
| magic number | 魔数（文件开头的识别标记） |
| repository / repo | 仓库 |
| commit / branch / merge | 提交 / 分支 / 合并 |

---

## 7. Rust 小抄（读源码时）

| 写法 | 中文 |
|------|------|
| `struct` | 结构体（自定义组合类型） |
| `enum` | 枚举（多选一） |
| `impl` | 给类型实现函数 |
| `fn` | 函数 |
| `let` | 绑定变量 |
| `match` | 模式匹配（按情况分支） |
| `if let` / `else` | 带模式的条件 |
| `loop` / `while` / `for` | 循环 |
| `Vec<T>` | 动态数组 |
| `String` / `&str` | 字符串 / 字符串切片 |
| `HashMap` | 键值表 |
| `Box<T>` | 堆上盒子 |
| `?` | 出错则提前返回 Err |

---

## 8. 还是看不懂某个词？

1. 在本表搜索  
2. 在源码里 `Ctrl+F` 搜该英文，看旁边的中文 `//!` / `///` 注释  
3. 对照 [COMPILE_PRIMER.md](COMPILE_PRIMER.md) 里的例子  
