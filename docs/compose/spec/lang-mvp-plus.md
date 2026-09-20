---
feature: lang-mvp-plus
status: delivered
updated: 2026-02-16
branch: feature/lang-mvp-plus
commits: 24efecd..HEAD
---

# Laughter 语言特性 MVP++

## Report

**What was built** — 在 MVP 之上加入：`struct` 声明/字面量/字段读写（含 `T[]` 与 `for-in`）、字符串内建 `len`/`str_at`/`str_sub`/`to_string`、数组 `push`/`pop`、`for x in arr`（编译期脱糖为下标 while）、`input()`。解析器对 `Ident {` 使用 lookahead，避免与 `for-in` 的块语法冲突。诊断格式与模块注释保持中文约定。

**Verification** —

- `cargo test` — PASS（20 unit + 23 integration）
- `laughter run examples/struct.lg` — PASS
- `laughter run examples/strings.lg` — PASS
- `laughter run examples/forin.lg` — PASS

**Journey log** —

1. `for x in arr { }` 若把 `arr {` 当结构体字面量会解析失败——lookahead **仅**在 `{ ident :` 时视为字面量；`{ }` 一律留给控制流空块。
2. 结构体句柄与数组一样用 `Rc<RefCell<...>>`，字段赋值对别名可见。
3. `for-in` 不新增 opcode，脱糖后复用 `Len`/`GetIndex`/`while` 跳转。
4. 空结构体字面量 `P { }` 不作特殊支持（与空块消歧冲突）；结构体至少要有一个字段才能方便地构造。

## [S1] Problem

Laughter MVP 已能写变量/函数/数组小程序，但缺少**记录/结构体**、可用的**字符串操作**、**数组追加**与**for-in**、以及**标准输入**，示例规模停在教学玩具层，难以演示稍完整的程序。用户选定本期补齐这些特性，验收为「示例 + 测试全绿」。

## [S2] Design

### 本期范围（已定）

| 特性 | 设计要点 |
|------|----------|
| 结构体 | `struct Name { field: Type, ... }`；字面量 `Name { f: e, ... }`；字段读 `p.x` / 写 `p.x = e`；同构数组 `Point[]` |
| 字符串 API | `len(s)` 对 string 与数组均有效；`str_at(s,i) -> string`；`str_sub(s,start,n) -> string`；`to_string(v) -> string` |
| 数组增强 | `push(a, v) -> void`、`pop(a) -> T`（空数组运行时错误）；`for x in arr { ... }`（编译期脱糖） |
| 标准输入 | `input() -> string`：读一行，去掉行尾换行；EOF 得空串 |

明确不做（本期）：模块 import、闭包、GC、用户自定义运算符、结构体方法、字节码文件序列化、`for i in 0..n`。

### 语法与类型

**结构体声明（仅顶层）**

```text
struct Point {
    x: int,
    y: int,
}
```

- 字段名唯一；类型非 void；字面量必须覆盖全部字段（可乱序）；`arr[i].x` 与 `p.a.b` 支持
- `for ident in arrayExpr { ... }`：`ident` 作用域在块内

**内建签名**

| 签名 | 语义 |
|------|------|
| `len(string) -> int` / `len(T[]) -> int` | 长度 |
| `str_at` / `str_sub` / `to_string` | 字符串操作 |
| `push` / `pop` | 数组变更 |
| `input() -> string` | stdin 一行 |

### 运行时与字节码

- `Value::Struct(Rc<RefCell<StructVal>>)`；`Module.struct_types` 存字段声明顺序
- `NewStruct` / `GetField` / `SetField` / `Push` / `ArrayPop` / `Input` / `StrAt` / `StrSub` / `ToString`
- `for-in`：隐藏局部 `for_arr`/`for_i` + while

## [S3] Out of Scope

- 模块/import、REPL、LLVM、GC 追踪
- 结构体方法/构造函数重载、可见性
- `for i in 0..n` 范围语法
- 隐式数值转换、字节码磁盘格式

## Tasks

- [x] T1: Spec 固化 + 分支 feature/lang-mvp-plus — acceptance: 本文档 status=designed 且用户批准 (covers: S2)
- [x] T2: Lexer/Parser：struct 声明、字段访问、结构体字面量、for-in — acceptance: 新语法单测通过；非法结构体字面量报错 (covers: S2; depends: T1)
- [x] T3: 类型检查：struct 类型环境、字段/构造/for-in/新内建签名 — acceptance: 缺字段/错类型/for-in 非数组被拒 (covers: S2; depends: T2)
- [x] T4: Value + 字节码 + Compiler/VM：结构体表示、GetField/SetField/NewStruct、内建实现、for-in 脱糖 — acceptance: disasm 含新指令；struct 示例 run 正确 (covers: S2; depends: T3)
- [x] T5: examples + 集成测试 + README — acceptance: `cargo test` 全绿；examples/{struct,strings,forin}.lg 输出正确 (covers: S2; depends: T4)
