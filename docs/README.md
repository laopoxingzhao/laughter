# Laughter 文档索引

> **撰写说明**：本文档树由 **MiMo（小米 AI 助手）** 整理与撰写，面向编译原理初学者。  
> 完整署名见 [AUTHORS.md](AUTHORS.md)。语言规则以 [LANGUAGE.md](LANGUAGE.md) 与代码测试为准。

Laughter 是一门**教学向编译语言**（`.lg`）。建议按下列顺序阅读。

| 顺序 | 文档 | 你将学会 |
|------|------|----------|
| 0 | [AUTHORS.md](AUTHORS.md) | 文档由谁写的、如何维护 |
| 1 | [GETTING_STARTED.md](GETTING_STARTED.md) | 安装、跑通第一个程序、常用命令 |
| 2 | [LANGUAGE.md](LANGUAGE.md) | 语言：类型、指针、函数、结构体、import… |
| 3 | [COMPILE_PRIMER.md](COMPILE_PRIMER.md) | **编译原理从零**：源码如何变成可执行步骤 |
| 4 | [ARCHITECTURE.md](ARCHITECTURE.md) | 源码目录 ↔ 编译阶段 |
| 5 | [TOOLS.md](TOOLS.md) | CLI、`.lgb` 字节码、`.lgpack` 包 |
| — | [GLOSSARY.md](GLOSSARY.md) | 英文标识符 / 术语 ↔ 中文 |

根目录 [README.md](../README.md) 只是仓库门面。

## 当前语言能力一览（便于对照）

- 标量：`int` `float` `bool` `string`
- 数组 `T[]`、结构体（值语义）、**指针** `&T` / `&mut T` / `nil` / `*p`
- `let` / `const`（编译期折叠）/ `fn`（与 `fun` 等价）
- 控制流：`if` `while` `for-in` `for i in a..b` `break` `continue`
- 现代写法：`s"..."` 插值、`Point { x, y }` 简写、函数体末尾隐式 return
- `import` 多文件；内建：`print` `len` `str_at` `str_sub` `to_string` `push` `pop` `input`
- CLI：`run` / `check` / `compile` / `exec` / `disasm` / `pack` / `list`
