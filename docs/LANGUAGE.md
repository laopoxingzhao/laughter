# Laughter 语言契约（v2）

> **权威文档**：本文件定义 Laughter 的类型、语法与语义。实现与测试须与之一致。  
> 入门见 [GETTING_STARTED.md](GETTING_STARTED.md)，实现见 [ARCHITECTURE.md](ARCHITECTURE.md)。

教学向编译语言。宿主 Rust；源文件 `.lg`；管线：**Lexer → Parser → Checker → Bytecode → VM**。

## 1. 类型

| 类型 | 说明 |
|------|------|
| `int` | 64 位有符号整数 |
| `float` | 64 位浮点 |
| `bool` | `true` / `false` |
| `string` | 不可变字节/字符串；`+` 拼接 |
| `T[]` | 同构数组；**引用语义**（共享句柄） |
| `struct` | 用户结构体；**值语义**（赋值/传参字段拷贝） |
| `void` | 仅作函数返回类型 |

- 无隐式 `int`/`float` 转换。
- 结构体字段可为标量、`string`、`T[]` 或其它结构体；`string`/数组字段在结构体拷贝时为**浅拷贝**。

## 2. 程序结构

```text
program := item*
item    := struct_decl | const_decl | import | fun_decl | stmt
```

- **入口**：若存在 `fun main() -> void`，`run` 从 `main` 进入（先完成模块声明合并）；否则执行顶层语句。
- **import**（仅顶层；通过文件路径运行，如 `laughter run main.lg`）：
  - `import "rel/path.lg";` — 将目标文件中顶层 `struct`/`const`/`fun` **扁平合入**（不执行目标顶层语句）。
  - `import "rel/path.lg" as ns;` — 合入并把符号命名为 `ns.name`（结构体类型名亦为 `ns.Name`）。
  - 路径相对当前文件；**禁止** `..`；环状 import 报错。
  - 同名冲突报错。
  - 库 API `run_source`/`compile_source` **不支持** import（单文件字符串）。
- 顶层语句在 `main` 存在时**不执行**（仅声明合并进模块）；无 `main` 时执行顶层语句。

## 3. 声明

### struct

```text
struct Point {
    x: int,
    y: int,
}
```

字段名唯一；类型非 `void`。

### const

```text
const N: int = 10;
const M: int = N * 2 + 1;
```

- 仅顶层；必须标注类型。
- 初始化式为**常量表达式**：字面量、已定义 `const`、一元 `-`/`!`、二元算术/比较/逻辑、字符串 `+`。
- 不允许函数调用、变量、数组/结构体字面量。
- 使用处编译为字面量；不可赋值。
- 标量类型：`int` `float` `bool` `string`。

### fun / 方法

```text
fun add(a: int, b: int) -> int { return a + b; }

fun Point.sum(self: Point) -> int {
    return self.x + self.y;
}
```

- 参数与返回类型必须写全；允许递归；无嵌套函数/函数值。
- 方法：`fun Type.name(self: Type, ...)`；首参类型必须是 `Type`。
- 调用：`p.sum()` 或 `Point.sum(p)`；接收者**按值**传递。
- 方法名与全局函数名不得冲突。
- `main` 必须为零参数且返回 `void`。
- 非 `void` 函数须保证所有路径 `return`（保守分析：if/else 双支 return 等）。

## 4. 语句

```text
let x: int = 1;
let y = 2;              // 类型取自初始化式
x = y + 1;
a[i] = v;
p.x = v;
p.a.b = v;              // 多层字段：沿值路径写回
a[i].x = v;

if cond { } else if cond { } else { }
while cond { }
for e in arr { }
for i in start..end { } // int，半开区间 [start, end)
break;
continue;
return expr; | return;
{ /* 块作用域 */ }
expr;                   // 表达式语句
```

- `if`/`while` 条件必须为 `bool`。
- `break`/`continue` 仅循环内。
- 空数组：`let a: int[] = [];`；无标注的 `[]` 为错误。

## 5. 表达式

| 优先级（低→高） | 运算 |
|----------------|------|
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `==` `!=` |
| 4 | `<` `<=` `>` `>=` |
| 5 | `+` `-` |
| 6 | `*` `/` `%` |
| 7 | 一元 `-` `!` |
| 8 | 后缀 `a[i]`、`p.f`、`f(...)`、`recv.m(...)` |

- 字面量：`123`、`1.5`、`true`/`false`、`"str"`（`\n \t \\ \"`）。
- 数组：`[e, ...]`；结构体：`Point { x: 1, y: 2 }`（字段须齐全，顺序任意）。
- `&&`/`||` 短路。
- 范围 `a..b` **仅**作为 `for-in` 迭代式。
- 整数除法向零截断；除零为运行时错误。

## 6. 内建函数（不可重定义）

| 函数 | 说明 |
|------|------|
| `print(v)` | 打印显示形式并换行到输出列表（CLI 打印一行） |
| `len(s_or_arr)` | 字符串字符数 / 数组长度 |
| `str_at(s, i)` | 第 i 个字符（从 0）；越界运行时错 |
| `str_sub(s, start, n)` | 子串；越界运行时错 |
| `to_string(v)` | 与 `print` 相同的显示字符串 |
| `push(arr, v)` | 追加 |
| `pop(arr)` | 弹出末元素；空数组运行时错 |
| `input()` | 读一行 stdin，去掉行尾 `\n`/`\r` |

显示形式：int/float（整数值浮点显示一位小数）/bool/string 原文；数组 `[a, b]`；结构体 `Name { f: v, ... }`。

## 7. 零成本约定

| 机制 | 约定 |
|------|------|
| `const` | 编译期折叠为字面量指令 |
| 结构体 | 栈上值，无 Rc 壳；拷贝赋值 |
| 字段写 | 槽内修改或拷贝-改-写回 |
| 方法 | 静态直呼 / 按类型名查表 |
| `for` | 脱糖为 while，无堆上迭代器 |

## 8. 诊断

- 编译期：`file:line:col: error: message`
- 运行时：`file:line: runtime error: message`（无行号时省略行号段）

## 9. CLI

```text
laughter run <file.lg>     # 检查 + 编译 + 执行
laughter check <file.lg>   # 检查 + 编译，不执行
laughter disasm <file.lg>  # 反汇编 + 常量表
```

## 10. 非目标

闭包、泛型、enum/match、impl 块、方法重载、包管理、GC、LLVM/JIT、REPL、隐式数值转换、`break` 带值、标签循环。
