---
feature: lang-rewrite
status: delivered
updated: 2026-02-16
branch: feature/lang-rewrite
commits: b0b9343..HEAD
---

# Laughter 语言能力汇总 + 彻底重写

## Report

**What was built** — 将演进中的实现彻底重写为分层结构：`syntax` / `sema` / `codegen` / `runtime` / `module_loader`。语言契约收敛到 **`docs/LANGUAGE.md`（v2）**，允许破坏旧兼容。相对旧实现：支持多层字段赋值 `p.a.b = v`；`const` 折叠与值语义结构体保留；import 仅合并声明；有 `main` 时顶层不执行。examples 在 v2 下行为与原先一致并全部通过集成测试。

**Verification** —

- `cargo test` — PASS（4 unit + 10 integration）
- `all_examples` 覆盖 hello/vars/branch/fun/fib/arrays/struct/strings/forin/methods_range/mod_main/zca
- 多层字段赋值、const 折叠、值语义、break/continue、类型/运行时错误均有测试

**Journey log** —

1. 重写以 `docs/LANGUAGE.md` 为唯一契约，旧 spec 仅作历史。
2. `struct_lit_ahead` 只认 `{ ident :`，避免 `for x in a { }` 被吞。
3. 多层字段写回：栈上保留 root 与父链，`SetField` 逆序落到 `SetLocal`。
4. `Op::from_u8` 用连续 `repr(u8)` 枚举，VM 分发更干净。

## 特性汇总（已固化于 LANGUAGE.md）

类型、声明（struct/const/fun/import）、语句与循环、表达式优先级、内建、零成本约定、诊断与 CLI —— 见 `docs/LANGUAGE.md` §1–§10。

## [S2] Design

分层包 + 契约文档；破坏性定稿见 LANGUAGE.md 与 git 历史讨论。

## [S3] Out of Scope

enum/泛型/闭包、LLVM、GC、REPL 等（LANGUAGE.md §10）。

## Tasks

- [x] T1: LANGUAGE.md + Spec (covers: S2)
- [x] T2: syntax 层重写 (covers: S2)
- [x] T3: sema + codegen + runtime (covers: S2)
- [x] T4: 测试/CLI/文档 (covers: S2)
- [x] T5: 全量验证 examples + cargo test (covers: S2)
