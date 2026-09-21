# 架构与源码地图

> **本文档由 MiMo（小米 AI 助手）撰写**，说明源码如何对应编译各阶段。署名见 [AUTHORS.md](AUTHORS.md)。

本文说明：**磁盘上的代码文件**如何对应编译器的各个阶段。  
若你还不清楚「词法/语法/字节码」是什么，请先读 [COMPILE_PRIMER.md](COMPILE_PRIMER.md)。

---

## 1. 总览

```text
.lg 源文件
    │
    ▼
syntax::lexer          字符 → Token
    │
    ▼
syntax::parser         Token → AST（语法树）
    │
    ▼
sema::check            类型检查 + const 折叠
    │
    ▼
codegen::compile       AST → 字节码 Module（内存）
    │
    ├──────────────► codegen::lgb     写入/读取 .lgb 文件
    ├──────────────► codegen::lgpack  打包/解包 .lgpack（ZIP）
    │
    ▼
runtime::vm            栈机执行 Module → 输出
```

多文件：`module_loader` 先解析 `import`，把多个文件的**声明**合并成一个 `Program`，再走上图。

---

## 2. 目录树（每个文件干什么）

```text
src/
  lib.rs                    库入口；导出常用 API
  module_loader.rs          读文件、解析 import、合并声明、编译

  syntax/                   ── 语法层 ──
    token.rs                Token 种类、Span（行列）
    lexer.rs                词法：切词
    ast.rs                  AST 节点定义
    parser/
      mod.rs                Parser 状态与工具（expect/peek）
      decl.rs               import / struct / const / fun
      stmt.rs               let / if / while / for / 赋值…
      expr.rs               表达式优先级

  sema/                     ── 语义层 ──
    types.rs                语义类型 Type
    check/
      mod.rs                Checker 入口、声明、作用域
      fold.rs               const 常量折叠
      stmt.rs               语句检查
      expr.rs               表达式与调用检查

  codegen/                  ── 代码生成 ──
    op.rs                   操作码列表（Add/Jump/Call…）
    chunk.rs                Chunk / Function / Module / 反汇编
    compile/
      mod.rs                Compiler 总控，生成 Module
      stmt.rs               语句、循环、赋值编译
      expr.rs               表达式与调用编译
    lgb.rs                  .lgb 二进制编解码
    lgpack.rs               .lgpack（类 JAR）打包

  runtime/                  ── 运行时 ──
    value.rs                Value（整数/字符串/数组/结构体…）
    vm/
      mod.rs                Vm、调用帧 Frame
      exec.rs               主解释循环（每条指令）
      source.rs             run_source：字符串源码 API

  bin/laughter.rs           命令行工具

examples/                   示例 .lg
tests/programs.rs           集成测试
docs/                       文档（见 docs/README.md）
```

---

## 3. 库 API（其它 Rust 代码怎么用）

| 函数 | 用途 |
|------|------|
| `run_file("app.lg")` | 按路径执行（支持 import） |
| `compile_file("app.lg")` | 按路径编译成 Module |
| `run_source("print(1);")` | 单文件字符串执行（**不支持 import**） |
| `codegen::lgb::encode_module` 等 | .lgb 读写 |
| `codegen::lgpack::pack_program` 等 | .lgpack 打包 |

CLI（`bin/laughter.rs`）是对上述 API 的薄封装，命令见 [TOOLS.md](TOOLS.md)。

---

## 4. 关键实现约定（和语言手册一致）

| 主题 | 约定 |
|------|------|
| 局部变量 | 函数帧内栈槽；`let` 往栈上压一格并登记槽号 |
| `const` | sema 折叠后，codegen 只发字面量 |
| 结构体 | `Value::Struct` 值拷贝；`string`/数组字段仍是句柄 |
| 数组 | `Rc<RefCell<Vec<Value>>>` 句柄，多变量可共享 |
| 方法 | 编译为 `Type.name`；`p.m()` 由 VM 按运行时类型名查找 |
| `for` | 脱糖为下标 + while，不生成堆上迭代器对象 |
| `if`/条件跳转 | JumpIf* 只 peek 栈顶；生成代码负责 Pop |
| 空块 vs 结构体字面量 | 仅 `{ 字段名:` 时当作结构体字面量 |
| 诊断 | 编译期 `file:行:列: 错误:…`；运行时 `file:行: 运行时错误:…` |

---

## 5. 测试在哪里

| 位置 | 测什么 |
|------|--------|
| `src/syntax/lexer.rs` 内 `#[cfg(test)]` | 分词 |
| `src/syntax/parser/` 内 | 解析结构 |
| `src/codegen/lgb.rs`、`lgpack.rs` 内 | 打包往返 |
| `tests/programs.rs` | 语言行为 + 全部 examples |

跑测试：

```bash
cargo test
```

---

## 6. 修改代码时的建议路径

1. 先改或写 **失败测试**（`tests/programs.rs`）  
2. 再改实现  
3. `cargo test` 直到全绿  
4. 同步文档（若行为或命令变了）  

改词法看 `lexer.rs`；改语法看 `parser/`；改类型规则看 `sema/check/`；改生成指令看 `codegen/compile/`；改运行行为看 `runtime/vm/`。

---

## 7. 文档导航

- 语言：[LANGUAGE.md](LANGUAGE.md)  
- 编译原理：[COMPILE_PRIMER.md](COMPILE_PRIMER.md)  
- 工具与文件格式：[TOOLS.md](TOOLS.md)  
- 术语：[GLOSSARY.md](GLOSSARY.md)  
