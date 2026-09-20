---
feature: laughter-mvp
status: in-progress
updated: 2026-02-16
branch: feature/laughter-mvp
commits: 6ab806e..6ab806e
---

# Laughter MVP

## Report

## [S1] Problem

需要一门**教学向的编译语言**，让人能端到端走完「源码 → 词法/语法/语义 → 字节码 → VM 执行」，而不是只写解释器或只做前端分析。当前仓库为空，缺少可运行的语言工具链、示例程序与验收标准。

用户明确：不知道该有哪些要求；由本工作流把关键决策定死并落地为可实现的 MVP。

## [S2] Design

### 已定决策

| 轴 | 选择 |
|----|------|
| 定位 | 教学/玩具语言，优先端到端可跑通 |
| 名称 | 语言 **Laughter**，源文件 `.lg`，CLI `laughter` |
| 宿主 | Rust + Cargo（`D:\DevelopmentKit\Rust\cargo`） |
| 后端 | 自研字节码 + 栈式 VM（不接 LLVM/不生成 C） |
| 类型 | **简单静态类型**，编译期检查 |
| 语法 | C/Rust 风：`{}` 块，语句以 `;` 结束 |
| Workspace | 主仓库功能分支 `feature/laughter-mvp`（本环境禁止 `git worktree add`） |

### 语言表面

**类型**

- 标量：`int`（宿主 `i64`）、`float`（`f64`）、`bool`、`string`
- 数组：**同构** `T[]`，如 `int[]`；堆上分配，作为句柄传递（调用时按引用生效）
- 本期不做：结构体、枚举、泛型、函数类型、`void` 多返回值

**声明与赋值**

```text
let x: int = 1;        // 显式标注
let y = 2;             // 类型取自初始化表达式
x = x + 1;             // 变量可变
```

**表达式与运算符**

- 算术：`+ - * / %`（一元 `-`）；`int` 与 `float` 之间**不隐式转换**，混用为类型错误
- 比较：`== != < <= > >=`（同类型标量；数组不可比较相等）
- 逻辑：`&& || !`，操作数为 `bool`，短路求值
- 字符串：`+` 为拼接；`print` 可打印所有标量与数组的调试表示
- 索引：`a[i]`，`i` 为 `int`；越界为**运行时**错误

**语句**

- `if cond { ... } else if cond { ... } else { ... }`，`cond: bool`
- `while cond { ... }`
- `return expr;` / `return;`（仅当函数返回类型为 `void` 时）
- 表达式语句：调用与赋值
- 块内可嵌套 `let`

**函数**

```text
fun add(a: int, b: int) -> int {
  return a + b;
}

fun main() -> void {
  print(add(1, 2));
}
```

- 参数与返回类型必须写全；返回类型可为标量、`T[]` 或 `void`
- 允许递归；**无闭包/无嵌套函数/无函数作为值**
- 可选入口：若存在 `main`（0 参数，返回 `void`），`laughter run` 从 `main` 进入；否则从顶层语句顺序执行
- 内建：`print(value)`（任意标量或数组）、`len(a) -> int`（数组长度）；二者不可被用户函数重定义

**标准库边界**

- 仅 `print`、`len`；无文件、网络、随机、时间、字符串切片 API

### 编译器与 VM 契约

```text
.lg 源码
  → Lexer（Token + 行列）
  → Parser（AST，语法错误即停）
  → Resolver + TypeChecker（作用域、函数签名、类型、未定义/重复定义）
  → Compiler（AST → Chunk 字节码 + 常量池）
  → VM（栈机执行）→ 运行时错误或正常退出
```

**字节码（栈式）核心指令（实现可增，语义需覆盖）**

- 常量：`Const idx`、`True`/`False`/`Nil`（Nil 仅用于 void，不对用户暴露）
- 局部变量：`GetLocal slot`、`SetLocal slot`
- 全局/顶层：`GetGlobal name_idx`、`SetGlobal name_idx`
- 算术/比较/逻辑：对应 opcode（逻辑在编译期或 VM 层实现短路）
- 控制流：`Jump`、`JumpIfFalse`、`Loop`；`Call argc`、`Return`
- 数组：`NewArray n`、`GetIndex`、`SetIndex`、`Len`
- 输出：`Print`
- 帧：调用时压入 CallFrame（返回地址、局部槽基址）

**值表示**

- `Value::{Int(i64), Float(f64), Bool(bool), Str(Rc<str>), Array(Rc<RefCell<Vec<Value>>>)}`（或等价的共享句柄）
- 数组元素类型在编译期已知；VM 侧可做防御性检查，但不以之替代类型检查

**错误**

| 阶段 | 行为 |
|------|------|
| 词法/语法/语义 | 打印 `file:line:col: error: ...`，退出码非 0，不进入 VM |
| 运行时 | 除零、数组越界、栈溢出保护（过深递归）→ 打印错误与大致位置（若保留调试映射）并失败退出 |
| CLI | `run`：分析+执行；`check`：只分析；`disasm`：打印指令与常量表 |

### 代码结构（单 crate）

```text
E:\Rust\laughter
  Cargo.toml          # package name = laughter, bin = laughter
  examples/*.lg
  src/
    main.rs           # CLI
    lexer.rs
    token.rs
    ast.rs
    parser.rs
    resolve.rs        # 作用域 + 类型检查（可拆 types.rs / check.rs）
    types.rs
    value.rs
    bytecode.rs
    compiler.rs
    vm.rs
  tests/              # 集成：跑 examples，断言 stdout/退出码
```

### 示例程序（验收锚点）

- `examples/hello.lg`：print 字面量
- `examples/vars.lg`：let/赋值/表达式
- `examples/branch.lg`：if/else + while
- `examples/fun.lg`：自定义函数 + 递归（如 fib）
- `examples/arrays.lg`：数组字面量、索引、写入、len
- `examples/type_error.lg`：应 `check` 失败的样例（放 `tests/fixtures/` 更合适）

### 测试边界

- 单元：词法分词、解析优先级/错误、类型检查拒绝混用 `int`+`float`、未定义变量、参数个数/返回类型
- 集成：`cargo test` 内嵌或调用二进制跑 examples，比对 stdout
- 不测：性能、GC、并发、跨平台发布

## [S3] Out of Scope

- 结构体/枚举/泛型/闭包/模块与 import
- 隐式数值转换、运算符重载、用户自定义类型
- 字符串完整 API、标准输入、文件/网络
- 垃圾回收器（本期用 `Rc`/`RefCell` 共享，不实现 tracing GC）
- LLVM/Cranelift、生成可执行文件、JIT
- REPL、语言服务器、包管理
- 独立 worktree（环境禁止 registry 变更；本功能在 `feature/laughter-mvp` 分支交付）

## Tasks

- [ ] T1: Cargo 工程骨架 + CLI 参数解析（run/check/disasm）— acceptance: `cargo build` 成功；`laughter` 无参打印用法 (covers: S2)
- [ ] T2: Lexer + Token — acceptance: 单测覆盖关键字/标识符/数字/字符串/运算符/注释/行列号；非法字符报错 (covers: S2)
- [ ] T3: AST + Parser — acceptance: 示例程序可解析为 AST；语法错误带行列；单测覆盖优先级与块结构 (covers: S2; depends: T2)
- [ ] T4: 类型系统 + 作用域/类型检查 — acceptance: `check` 对合法样例通过；对未定义变量、类型不匹配、错误签名/fixtures 拒绝 (covers: S2; depends: T3)
- [ ] T5: 字节码模块 + Compiler — acceptance: AST 可编译为 Chunk；`disasm` 打印可读指令；递归函数/跳转标签正确 (covers: S2; depends: T4)
- [ ] T6: VM + Value + 内建 print/len — acceptance: examples 的 run 输出与设计一致；越界/除零有运行时错误 (covers: S2; depends: T5)
- [ ] T7: examples + 集成测试 + README 使用说明 — acceptance: `cargo test` 全绿；`laughter run examples/fib.lg` 输出正确 (covers: S2; depends: T6)
