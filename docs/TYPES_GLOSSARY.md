# 类型对照表（英文标识符 → 中文）

代码里类型、枚举、字段名多为英文。此表按模块列出常见英文名的中文含义，方便对照源码阅读。

更完整的语言规则见 [LANGUAGE.md](LANGUAGE.md)；阅读顺序见 [CODE_TOUR.md](CODE_TOUR.md)。

---

## syntax/token.rs — 词法

| 英文 | 中文 | 说明 |
|------|------|------|
| `Span` | 源位置 | 行号 `line` + 列号 `col` |
| `Token` | 词法单元 | 一个词：`kind`（是什么词）+ `span`（在哪） |
| `TokenKind` | 词的种类 | 见下表 |
| `Ident` | 标识符 | 用户起的名字，如 `x`、`Point` |
| `Int` / `Float` / `Str` | 整数 / 浮点 / 字符串字面量 | `1` / `1.5` / `"hi"` |
| `Fun` / `Struct` / `Const` / `Let` | 关键字 | `fun` `struct` `const` `let` |
| `If` / `Else` / `While` / `For` / `In` | 关键字 | 条件与循环 |
| `Break` / `Continue` / `Return` | 关键字 | 跳出 / 继续 / 返回 |
| `TyInt` `TyFloat` `TyBool` `TyString` `TyVoid` | **类型关键字** | 源码里写类型时用的 `int` 等 |
| `LParen` `RParen` | 左右圆括号 | `(` `)` |
| `LBrace` `RBrace` | 左右花括号 | `{` `}` |
| `LBracket` `RBracket` | 左右方括号 | `[` `]` |
| `Comma` `Colon` `Semi` | 逗号 / 冒号 / 分号 | `,` `:` `;` |
| `Dot` / `DotDot` | 点 / 双点 | `.`（字段） / `..`（范围） |
| `Arrow` | 箭头 | `->` 函数返回类型 |
| `Plus` `Minus` `Star` `Slash` `Percent` | 加 减 乘 除 取余 | |
| `EqEq` `BangEq` | 等于 / 不等于 | `==` `!=` |
| `Lt` `LtEq` `Gt` `GtEq` | 小于/≤/大于/≥ | `<` `<=` `>` `>=` |
| `AndAnd` `OrOr` `Bang` | 逻辑与/或/非 | `&&` `\|\|` `!` |
| `Assign` | 赋值号 | `=` |
| `Eof` | 文件结束 | 词法流末尾的哨兵 |

---

## syntax/ast.rs — 语法树（解析结果）

| 英文 | 中文 | 说明 |
|------|------|------|
| `TypeExpr` | 类型标注 | 源码里写的类型，尚未检查 |
| `Named(String)` | 具名类型 | 结构体名，如 `Point` |
| `Array(Box<TypeExpr>)` | 数组类型 | `T[]`，元素类型在 Box 里 |
| `BinOp` | 二元运算符 | `Add` 加 `Sub` 减 `Mul` 乘 `Div` 除 `Rem` 余… |
| `UnOp` | 一元运算符 | `Neg` 取负、`Not` 逻辑非 |
| `Ident` | 名字 | `name` 文本 + `span` 位置 |
| `Expr` | 表达式节点 | 见下方「Expr 变体」 |
| `Stmt` | 语句节点 | let/赋值/if/while/for/return… |
| `Block` | 语句块 | `{ ... }`，带自己的 `stmts` 列表 |
| `FunDecl` | 函数声明 | 名、参数、返回类型、函数体 |
| `on_type` | 所属结构体 | 方法 `Point.sum` 时为 `Some(Point)` |
| `Param` | 函数参数 | 名字 + 类型 |
| `StructDecl` | 结构体声明 | 名 + 字段列表 |
| `FieldDecl` | 字段声明 | 字段名 + 类型 |
| `ConstDecl` | 常量声明 | 名 + 类型 + 初始表达式 |
| `ImportItem` | import 项 | 路径 + 可选别名 `as ns` |
| `Program` | 整个程序 | `items`：顶层的声明/语句列表 |
| `Item` | 顶层一项 | Struct / Const / Fun / Import / Stmt |

### `Expr` 变体（表达式）

| 英文 | 中文 | 例 |
|------|------|-----|
| `Int` `Float` `Bool` `Str` | 字面量 | `1` `1.5` `true` `"hi"` |
| `Var` | 变量引用 | `x` |
| `Unary` | 一元运算 | `-x` `!flag` |
| `Binary` | 二元运算 | `a + b` |
| `Call` | 函数调用 | `f(1, 2)` |
| `MethodCall` | 方法调用 | `p.sum()` |
| `Index` | 下标 | `a[0]` |
| `Field` | 字段访问 | `p.x` |
| `Array` | 数组字面量 | `[1, 2]` |
| `StructLit` | 结构体字面量 | `Point { x: 1, y: 2 }` |
| `Range` | 范围 | `0..n`（仅 for 使用） |

### `Stmt` 变体（语句）

| 英文 | 中文 | 例 |
|------|------|-----|
| `Let` | 声明并初始化 | `let x = 1;` |
| `Assign` | 赋值 | `x = 2;` `a[0] = 1;` `p.x = 1;` |
| `If` | 条件分支 | `if c { } else { }` |
| `While` | 当型循环 | `while c { }` |
| `For` | 遍历/范围循环 | `for x in a` / `for i in 0..n` |
| `Break` / `Continue` | 中断 / 继续下一轮 | 仅循环内 |
| `Return` | 返回 | `return 1;` |
| `Expr` | 表达式语句 | `print(1);` |
| `Block` | 独立块 | `{ let t = 1; }` |

`AssignStmt` 字段：`name` 变量名；`index` 数组下标（可无）；`fields` 字段路径（可多层）；`value` 右值。

---

## sema/types.rs — 语义类型（检查器使用）

| 英文 | 中文 | 对应源码类型 |
|------|------|----------------|
| `Type::Int` | 整数类型 | `int` |
| `Type::Float` | 浮点类型 | `float` |
| `Type::Bool` | 布尔类型 | `bool` |
| `Type::Str` | 字符串类型 | `string` |
| `Type::Void` | 无返回值 | `void` |
| `Type::Array(Box<Type>)` | 数组类型 | `int[]` 等 |
| `Type::Struct(String)` | 结构体类型 | `Point` |

`TypeExpr`（语法）→ 检查时 → `Type`（语义）。

---

## runtime/value.rs — 运行时的值

| 英文 | 中文 | 说明 |
|------|------|------|
| `Value` | 运行时值 | 栈上放的就是它 |
| `Value::Int(i64)` | 64 位整数 | |
| `Value::Float(f64)` | 64 位浮点 | |
| `Value::Bool(bool)` | 真/假 | |
| `Value::Str(Rc<str>)` | 字符串（句柄） | `Rc`＝引用计数，共享一份数据 |
| `Value::Array(ArrayHandle)` | 数组句柄 | 见下 |
| `Value::Struct(StructVal)` | 结构体值 | 整份拷贝，值语义 |
| `ArrayHandle` | 数组句柄 | `Rc<RefCell<Vec<Value>>>`：可共享、可改内部 |
| `StructVal` | 结构体实例 | `name` 类型名 + `fields` 字段表 |
| `RefCell` | 可变借用容器 | 运行时允许改内部数据（教学实现用） |
| `Rc` | 引用计数指针 | 多个变量可指向同一份数据 |

**记忆**：数组像「遥控器」（引用）；结构体像「复印件」（值）。

---

## sema/check.rs — 检查器

| 英文 | 中文 | 说明 |
|------|------|------|
| `CheckError` | 语义错误 | `message` 信息 + `span` 位置 |
| `Checker` | 检查器 | 持有函数表、结构体表、const 表、作用域栈 |
| `FunInfo` | 函数签名信息 | `params` 参数类型列表 + `ret` 返回类型 |
| `BUILTINS` | 内建函数名表 | print、len 等，不可重定义 |
| `scopes` | 作用域栈 | 每层一个「名字 → 类型」表，块结束弹出 |
| `fold` | 常量折叠 | 编译期把 `N*2+1` 算成数字 |

---

## codegen — 字节码

| 英文 | 中文 | 说明 |
|------|------|------|
| `Op` | 操作码 | 指令种类，如 `Add`、`Jump` |
| `Chunk` | 字节码块 | `code` 指令字节 + `constants` 常量池 + `lines` 行号 |
| `Function` | 已编译函数 | 参数个数、局部槽数、是否 void、字节码 |
| `Module` | 编译产物 | 所有函数 + 入口 + 结构体布局 |
| `StructType` | 结构体布局 | 名 + 字段名顺序 |
| `Compiler` | 编译器 | AST → 字节码 |
| `CompileError` | 编译错误 | 带行列 |
| `GetLocal` / `SetLocal` | 读/写局部槽 | 槽＝函数在栈上的一格 |
| `SetLocalField` | 改槽内结构体字段 | 值语义赋值 |
| `Call` / `CallMethod` | 调用函数/方法 | |
| `Jump` `JumpIfFalse` `JumpIfTrue` | 跳转 | 条件跳转只查看栈顶 |
| `Loop` | 循环回跳 | 跳回循环开头 |
| `NewArray` / `GetIndex` / `SetIndex` | 建数组/读下标/写下标 | |
| `NewStruct` / `GetField` / `SetField` | 建结构体/读字段/写字段 | |

### 常见 Op 名对照

| Op | 中文 |
|----|------|
| `Const` | 压入常量 |
| `True` / `False` | 压入真/假 |
| `Pop` | 弹出栈顶 |
| `Add` `Sub` `Mul` `Div` `Rem` | 加减乘除取余 |
| `Neg` / `Not` | 取负 / 逻辑非 |
| `Eq` `Ne` `Lt` `Le` `Gt` `Ge` | 相等/不等/小于/≤/大于/≥ |
| `Len` | 求长度 |
| `Print` | 打印栈顶 |
| `Push` / `ArrayPop` | 数组追加 / 弹出末元素 |
| `Input` | 读一行输入 |
| `StrAt` / `StrSub` / `ToString` | 取字符 / 子串 / 转字符串 |
| `Return` | 从函数返回 |

---

## runtime/vm.rs — 虚拟机

| 英文 | 中文 | 说明 |
|------|------|------|
| `Vm` | 虚拟机 | 解释执行字节码 |
| `VmError` | 运行时错误 | 消息 + 行号 |
| `Frame` | 调用帧 | `func` 函数下标、`ip` 指令位置、`base` 局部槽起点 |
| `stack` | 操作数栈 | 指令压入/弹出的值 |
| `ip` | 指令指针 | 下一条指令下标（instruction pointer） |
| `base` | 帧基址 | 本函数局部变量在栈上的起点 |
| `peek` | 偷看栈顶 | 不弹出 |
| `pop` | 弹出栈顶 | 取走并去掉 |
| `push_frame` | 压入调用帧 | 进入函数 |
| `CallMethod` | 按方法名调用 | 根据接收者类型名找 `Type.method` |

---

## module_loader.rs

| 英文 | 中文 |
|------|------|
| `module_loader` | 模块加载器 |
| `compile_path` / `run_path` | 按路径编译 / 按路径运行 |
| `alias` | import 的 `as 名字` 别名 |
| `cyclic import` | 循环导入（A→B→A） |

---

## Rust 语法小抄（读注释/代码时）

| 英文/记号 | 中文 |
|-----------|------|
| `struct` | 结构体（自定义组合类型） |
| `enum` | 枚举（多选一的类型） |
| `impl` | 给类型写函数（方法） |
| `fn` | 函数 |
| `let` | 绑定一个变量 |
| `mut` | 可变（能修改） |
| `&` | 借用（引用，不拿走所有权） |
| `&mut` | 可变借用 |
| `Box<T>` | 堆上的盒子（装递归/大对象） |
| `Rc<T>` | 引用计数共享指针 |
| `RefCell<T>` | 运行时可变借用 |
| `Option` | 可选：`Some(x)` 或 `None` |
| `Result` | 结果：`Ok(x)` 或 `Err(e)` |
| `Vec<T>` | 动态数组 |
| `HashMap` | 哈希表（键→值） |
| `unwrap` | 直接取出（出错会崩溃，测试里常见） |
| `clone` | 复制一份 |
| `pub` | 公开，可被其它模块使用 |
| `crate` | 当前这个库/包 |
