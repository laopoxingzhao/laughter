---
feature: zca-structs-const
status: delivered
updated: 2026-02-16
branch: feature/zca-structs-const
commits: f6fa46f..HEAD
---

# 零成本向：栈上结构体值语义 + const 折叠

## Report

**What was built** — 结构体从 `Rc<RefCell<StructVal>>` 改为 **`Value::Struct(StructVal)` 值语义**：赋值/传参/方法接收者均为字段拷贝；`p.x = v` 用 `SetLocalField` 直接改栈槽。新增顶层 **`const Name: T = expr;`**，常量表达式在检查器中折叠后由编译器 `CONST` 字面量发出。README 增加「零成本抽象」成本约定表；`examples/zca.lg` 演示值语义与折叠。`string`/数组字段仍为句柄浅拷贝。

**Verification** —

- `cargo test` — PASS（含 zca 示例、const 折叠/赋值拒绝、值拷贝、方法接收者拷贝、数组元素字段写回、disasm 含 `CONST (20)`）
- `laughter run examples/zca.lg` — `21 zca 1 50 103 1 25`

**Journey log** —

1. 去掉结构体 `Rc` 后，字段赋值必须写回栈槽（`SetLocalField`），否则 `GetLocal` 拷贝上的修改会丢失。
2. `const` 与局部变量同名时优先按常量折叠；对 `const` 赋值在检查器拒绝。
3. 多层字段赋值 `p.a.b = v` 在值语义下本期要求中间 `let`（避免隐式写回链）。

## [S1] Problem

结构体原为堆句柄引用语义，与「零成本抽象」目标不符；缺少编译期常量。

## [S2] Design

见上 Report 与 README「零成本抽象」；契约细节：

- 值语义结构体 + `SetLocalField`
- `const` 仅标量（int/float/bool/string）常量表达式
- 方法接收者按值传递

## [S3] Out of Scope

泛型单态化、借用检查、const 数组/结构体、消除字符串/数组的 Rc。

## Tasks

- [x] T1: Spec 批准 + 分支 (covers: S2)
- [x] T2: const 语法 + 检查 + 折叠 (covers: S2; depends: T1)
- [x] T3: Value::Struct 去 Rc + 值语义 (covers: S2; depends: T1)
- [x] T4: examples/zca.lg + README + 全量测试 (covers: S2; depends: T2, T3)
