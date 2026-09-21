---
feature: ptrs-modern-syntax
status: delivered
updated: 2026-02-16
branch: feature/ptrs-modern-syntax
commits: 6acf581..6acf581
---

# 指针（安全引用）+ 现代化语法

## Report

**What was built** — 安全引用 `&T`/`&mut T`/`nil`/`*p`（可解引用读写，空指针与悬垂有运行时检查）；现代语法 `fn`、`s"..."` 插值、结构体字段简写、函数体末尾隐式 return。旧 `fun`/完整字段写法保持兼容。

**Verification** — `cargo test` 8 unit + 17 integration 全绿；`examples/pointers.lg`、`modern.lg` 输出符合预期。

**Journey log** —
1. `{ x, y }` 简写要求 parser 对 `{` 的 lookahead 认 `,` / `}`，不只 `:`。
2. `fn` 与 `fun` 等价：`parse_program` 与 `fun_decl` 都要接受 `Fn`。
3. 隐式 return：解析器在无 `;` 且见 `}` 时打标；检查器 `always_returns` 与 codegen 末尾都要识别。
4. 指针比较：检查器允许 `&` 类型互相比较；VM `val_eq` 对 `Ptr` 单独分支。

## Tasks

- [x] T1 Spec 批准 (covers: S2)
- [x] T2 语法层 (covers: S2)
- [x] T3 语义层 (covers: S2)
- [x] T4 运行时 Ptr (covers: S2)
- [x] T5 文档 + examples + 测试 (covers: S2)

## [S1] Problem

Laughter 缺少显式**指针/引用**概念；语法偏「作业本」风格，与现代语言（Rust/TypeScript/Kotlin 等）的常见优势写法不够贴合。本期加入：

1. **安全引用** `&T` / `&mut T`（可空 `nil`，解引用运行时检查）；
2. **融合现代语法**：`fn`、字符串插值、结构体字段简写、函数体末尾表达式隐式 return。

## [S2] Design

### 1. 指针 / 引用

**类型**

| 类型 | 含义 |
|------|------|
| `&T` | 指向 `T` 的**只读**引用（可 `nil`） |
| `&mut T` | 指向 `T` 的**可写**引用（可 `nil`） |

- `&mut T` 可赋给 `&T`（可放宽权限）；`&T` **不能**赋给 `&mut T`
- 原类型 `T` 可为标量、`string`、数组类型、结构体名等

**表达式与语句**

```text
let x = 1;
let r: &int = &x;        // 取只读引用
let m: &mut int = &mut x; // 取可写引用
print(*r);               // 解引用读
*m = 5;                  // 通过 &mut 写回 x
print(x);                // 5
print(m == nil);         // false

let p: &int = nil;       // 空指针
// *p                    // 运行时错误：解引用空指针
```

- `&e` / `&mut e`：`e` 必须是**左值**（变量、数组元素 `a[i]`、字段 `p.f`）
- `*p`：读出被指对象（`p` 为 `&T` 或 `&mut T`）
- `*p = v`：仅当 `p` 为 `&mut T` 时合法
- `nil`：空引用字面量，类型可与任意 `&T` / `&mut T` 比较或赋值
- 比较：`p == q` / `p != nil`（比较引用身份/是否为空）
- 通过 `&` 写入 → 编译错误；解引用 `nil` → 运行时错误
- **悬垂**：引用指向已返回函数的局部槽 → 运行时错误（教学版检测帧是否仍存在）

**实现约定（零成本向）**

- 引用为 `Value::Ptr`：内部可为「帧内槽地址」或「数组元素地址」
- 不要求 GC；局部引用随帧存在
- 结构体**字段**左值：`&p.f` 指向当前绑定内字段（写经 `&mut` 生效）

**不做**：指针算术、裸地址、完整 Rust 借用检查、引用生命周期标注。

### 2. 现代化语法（融合）

| 特性 | 写法 | 说明 |
|------|------|------|
| `fn` 关键字 | `fn add(a: int) -> int { ... }` | 与 `fun` **等价**，文档优先写 `fn` |
| 字符串插值 | `s"x={x}, y={y}"` | `{expr}` 求值后转字符串；可嵌套标识符/简单表达式 |
| 结构体简写 | `let p = Point { x, y };` | 字段名与同名变量一致时可省略 `: value` |
| 隐式返回 | `fn add(a: int, b: int) -> int { a + b }` | 函数体最后一条**无分号表达式**且返回类型非 `void` 时，等价 `return e;` |

**保持不变**：`fun` 仍可用；分号语句规则；既有类型/import/const 等。

### 3. 测试与文档

- 更新 `docs/LANGUAGE.md`（指针、fn、插值、简写、隐式 return）
- 新 examples：`examples/pointers.lg`、`examples/modern.lg`
- 集成测试：引用读写、nil 解引用、`&` 写被拒、fn/插值/简写/隐式 return
- 错误信息中文

## [S3] Out of Scope

裸指针算术、借用检查器、宏、async、完整 match/enum、操作符重载。

## Tasks

- [ ] T1: Spec 批准 + 分支 (covers: S2)
- [ ] T2: 语法层：& &mut * nil、fn、s\"\"、结构体简写、隐式 return 解析 (covers: S2)
- [ ] T3: 语义层：引用类型、左值限制、&/解引用检查 (covers: S2)
- [ ] T4: 运行时：Value::Ptr、&/*、nil、悬垂检查 (covers: S2)
- [ ] T5: 文档 + examples + 测试全绿 (covers: S2)
