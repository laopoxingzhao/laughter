---
feature: lang-next
status: delivered
updated: 2026-02-16
branch: feature/lang-next
commits: e6dd565..HEAD
---

# Laughter 下一期：范围 for、break/continue、结构体方法、模块 import

## Report

**What was built** — 本期交付：`for i in a..b`（半开区间）+ `break`/`continue`（范围 for、数组 for-in、while）；结构体方法 `fun Point.sum(self: Point)`，调用 `p.sum()` 或 `Point.sum(p)`（VM `CallMethod` 按接收者类型名解析）；多文件 `import "x.lg"` / `import "x.lg" as ns`（loader 合并声明，命名空间符号名为 `ns.sym`，不执行目标文件顶层语句）。`run_source` 拒绝含 import 的源；CLI `run`/`check`/`disasm` 走 `loader::run_file`/`compile_file`。

**Verification** —

- `cargo test` — PASS（16 unit + 32 integration）
- `laughter run examples/methods_range.lg` — 输出 `7 7 0 1 3 6`
- `laughter run examples/mod_main.lg` — 输出 `5 42`

**Journey log** —

1. `Point.sum(p)` 与 `p.sum()` 共用方法函数；前者在编译期按 `Type.method` 静态 Call，后者走 `CallMethod`。
2. `break`/`continue` 用向前 `Jump` + `patch_jump_to`；continue 目标必须在增量步，避免数组 for-in 死循环。
3. 结构体字面量 lookahead 只认 `{ ident :`，空 `{}` 留给控制流块（沿用 lang-mvp-plus 结论）。
4. import 只合并声明；模块内顶层语句不会在 import 时执行。

## [S1] Problem

语言已具备基础类型、结构体、数组与 for-in，但缺少数值范围循环与 `break`/`continue`、结构体方法、模块 import。本期交付这三块，验收为**实现与中文文档/注释同步**。

## [S2] Design

### 1. 范围 for + break/continue

- `for i in start..end { }` 半开 `[start, end)`，两侧为 `int`
- `break`/`continue` 可用于 for/while；循环外为编译错误
- 编译：范围 for 脱糖为 `i` + `end` 局部 + while；continue 回填到增量

### 2. 结构体方法

- `fun Type.method(self: Type, ...) -> R`；首参类型必须是该结构体
- `recv.method(args)` / `Type.method(recv, args)`
- 全局函数名与方法名冲突 → 错误；`CallMethod` 运行时用 `结构体运行时类型名.方法名` 查找

### 3. 模块 import

- `import "path.lg";` 扁平合入顶层 fun/struct；`as ns` 时符号名为 `ns.name`
- 路径相对当前文件；禁止 `..`；环 import 报错
- **只合并声明，不执行**目标文件顶层语句
- CLI 经 `loader`；`run_source` 遇 import 报错

### 文档同步

- README 特性表/示例表/设计文档链接
- 本 spec + `src/loader.rs` 等 `//!` 注释

## [S3] Out of Scope

- `impl`、方法重载、可见性、包管理、`break` 带值、闭包、REPL、GC、LLVM

## Tasks

- [x] T1: 分支 + Spec 批准 (covers: S2)
- [x] T2: Lexer/Parser：`..`、break/continue、import、`Type.method`、`recv.method()` (covers: S2; depends: T1)
- [x] T3: 类型检查：范围 for、方法表、import 合并语义 (covers: S2; depends: T2)
- [x] T4: 编译器/VM + loader `run_file` (covers: S2; depends: T3)
- [x] T5: 文档同步 + examples + 全量测试 (covers: S2; depends: T4)
