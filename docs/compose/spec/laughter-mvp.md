---
feature: laughter-mvp
status: delivered
updated: 2026-02-16
branch: feature/laughter-mvp
commits: 6ab806e..HEAD
---

# Laughter MVP

## Report

**What was built** — 教学语言 **Laughter**（`.lg`）的完整 MVP 工具链：Rust 实现的 lexer → parser → 静态类型检查 → 栈式字节码编译器 → VM。支持 `int`/`float`/`bool`/`string`/同构数组、变量与赋值、`if`/`while`、用户函数（可递归）、内建 `print`/`len`。CLI 为 `laughter run|check|disasm`，编译期诊断格式为 `file:line:col: error: ...`。库入口为 `laughter::vm::{run_source, compile_source}`；二进制在 `src/bin/laughter.rs`。

**Verification** —

- `cargo test` — PASS（16 unit + 16 integration）
- `cargo build` — PASS
- `laughter run examples/fib.lg` — PASS，输出 `55`
- `laughter run examples/{hello,vars,branch,fun,arrays}.lg` — PASS
- `laughter check tests/fixtures/type_error.lg` — PASS，拒绝并带 `file:line:col: error:`
- 独立复审：三项 critical（if 无 else 栈损坏、错误格式缺 file、fib.lg/fixture 缺失）均已修复，无新增 critical

**Journey log** —

1. 环境禁止 `git worktree add`（共享 worktree registry），改为在主仓库 `feature/laughter-mvp` 分支交付。
2. VM 曾对函数帧预填局部槽为 `0`，把 `let` 推入的值挤到错误 slot——改为不在 push_frame 时 padding。
3. `Return` 曾按 `base+locals` 截断，callee 局部变量残留导致递归 fib 错值——改为 truncate 到 `frame.base` 再压回返回值。
4. `if` 无 `else` 的 true 路径会落入 false-path 的 `Pop`，破坏栈；测试因 then 分支 `return` 而盲区——codegen 增加跳过 `Pop` 的 `Jump` 并补集成测试。
5. 审查指出 spec 内示例名 `fun.lg` 与 T7 验收 `fib.lg` 不一致——两者均保留。

## [S1] Problem

需要一门**教学向的编译语言**，让人能端到端走完「源码 → 词法/语法/语义 → 字节码 → VM 执行」，而不是只写解释器或只做前端分析。当前仓库为空，缺少可运行的语言工具链、示例程序与验收标准。

用户明确：不知道该有哪些要求；由本工作流把关键决策定死并落地为可实现的 MVP。

## [S2] Design

### 已定决策

| 轴 | 选择 |
|----|------|
| 定位 | 教学/玩具语言，优先端到端可跑通 |
| 名称 | 语言 **Laughter**，源文件 `.lg`，CLI `laughter` |
| 宿主 | Rust + Cargo |
| 后端 | 自研字节码 + 栈式 VM（不接 LLVM/不生成 C） |
| 类型 | **简单静态类型**，编译期检查；`int`/`float` 不隐式转换 |
| 语法 | C/Rust 风：`{}` 块，语句以 `;` 结束 |
| Workspace | 主仓库功能分支 `feature/laughter-mvp`（环境禁止 `git worktree add`） |
| 入口 | 存在 `fun main() -> void` 时从 `main` 进入；否则执行顶层语句 |

### 语言表面

**类型**

- 标量：`int`（宿主 `i64`）、`float`（`f64`）、`bool`、`string`
- 数组：同构 `T[]`；堆上句柄（`Rc<RefCell<Vec<Value>>>`），传参为引用语义
- `void`：仅作函数返回类型
- `let a: int[] = [];` 合法（空数组需标注）；无标注的 `[]` 为类型错误
- 本期不做：结构体、枚举、泛型、函数类型、多返回值

**声明与运算**

- `let x: int = 1;` / `let y = 2;`（类型取自初始化式）/ `x = expr;`
- 算术 `+ - * / %`（一元 `-`）；比较 `== != < <= > >=`；逻辑 `&& || !`（短路）
- 字符串 `+` 拼接；`print` 可打印标量与数组；`len(arr) -> int`
- 索引 `a[i]` / `a[i] = v`；越界为运行时错误；除零为运行时错误

**函数**

```text
fun add(a: int, b: int) -> int {
  return a + b;
}
fun main() -> void {
  print(add(1, 2));
}
```

- 参数与返回类型必须写全；允许递归；无闭包/嵌套函数/函数值
- `print`/`len` 为内建，不可重定义；`main` 必须零参数且返回 `void`
- 非 `void` 函数须保证所有路径返回（检查器做保守路径分析）

### 编译器与 VM 契约（交付实现）

```text
.lg → Lexer → Parser → Checker → Compiler（Chunk 字节码）→ VM
```

**实现相对设计初稿的偏差（已按实现定稿）**

- `Call` 操作码后跟 **函数索引** `u16`，arity 取自 `Function.arity`（非 `Call argc`）
- 顶层语句编译为合成函数 `$toplevel`，用局部槽而非 `GetGlobal`/`SetGlobal`（opcode 保留但未使用）
- `void` 返回不压值；`Return` 将栈截断到 `frame.base`，非 void 再压回返回值
- 代码结构：`src/lib.rs` + `src/bin/laughter.rs`（便于集成测试调用 `run_source`），而非单一 `src/main.rs`
- 诊断：编译期 `file:line:col: error: ...`；运行时 `file:line: runtime error: ...`
- `if` 无 `else`：true 路径在 then 后 `Jump` 越过 false-path 的条件 `Pop`

**字节码核心指令**

`Const/True/False`，`GetLocal/SetLocal`，`Add/Sub/Mul/Div/Rem/Neg/Not`，比较与逻辑，`Jump/JumpIfFalse/JumpIfTrue/Loop`，`Call/Return`，`NewArray/GetIndex/SetIndex/Len`，`Print/Pop`

**值**：`Value::{Int, Float, Bool, Str, Array}`；数组为共享句柄。

### 示例与测试

- `examples/hello.lg` `vars.lg` `branch.lg` `fun.lg` `fib.lg` `arrays.lg`
- `tests/fixtures/type_error.lg`：`check`/`run` 应失败
- 单元 + 集成测试见 `src/*` 内 `#[cfg(test)]` 与 `tests/programs.rs`

## [S3] Out of Scope

- 结构体/枚举/泛型/闭包/模块与 import
- 隐式数值转换、运算符重载、用户自定义类型
- 字符串完整 API、标准输入、文件/网络
- 垃圾回收器（`Rc`/`RefCell` 共享，无 tracing GC）
- LLVM/Cranelift、生成本地可执行文件、JIT
- REPL、语言服务器、包管理
- 独立 worktree（环境禁止 registry 变更）

## Tasks

- [x] T1: Cargo 工程骨架 + CLI 参数解析（run/check/disasm）— acceptance: `cargo build` 成功；`laughter` 无参打印用法 (covers: S2)
- [x] T2: Lexer + Token — acceptance: 单测覆盖关键字/标识符/数字/字符串/运算符/注释/行列号；非法字符报错 (covers: S2)
- [x] T3: AST + Parser — acceptance: 示例程序可解析为 AST；语法错误带行列；单测覆盖优先级与块结构 (covers: S2; depends: T2)
- [x] T4: 类型系统 + 作用域/类型检查 — acceptance: `check` 对合法样例通过；对未定义变量、类型不匹配、错误签名/fixtures 拒绝 (covers: S2; depends: T3)
- [x] T5: 字节码模块 + Compiler — acceptance: AST 可编译为 Chunk；`disasm` 打印可读指令；递归函数/跳转标签正确 (covers: S2; depends: T4)
- [x] T6: VM + Value + 内建 print/len — acceptance: examples 的 run 输出与设计一致；越界/除零有运行时错误 (covers: S2; depends: T5)
- [x] T7: examples + 集成测试 + README 使用说明 — acceptance: `cargo test` 全绿；`laughter run examples/fib.lg` 输出正确 (covers: S2; depends: T6)
