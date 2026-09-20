# Laughter 文档索引

Laughter 是一门**教学向编译语言**（`.lg`）。本文档树按「先学会用 → 再懂语言 → 再懂编译器」排列。

**适合谁**：想学编译原理、但英文和基础都不厚的同学。每篇都尽量少用行话；出现的英文词在 [GLOSSARY.md](GLOSSARY.md) 都有中文对照。

| 顺序 | 文档 | 你将学会 |
|------|------|----------|
| 1 | [GETTING_STARTED.md](GETTING_STARTED.md) | 安装、跑通第一个程序、常用命令 |
| 2 | [LANGUAGE.md](LANGUAGE.md) | 语言怎么写：类型、变量、循环、函数、结构体… |
| 3 | [COMPILE_PRIMER.md](COMPILE_PRIMER.md) | **编译原理从零讲**：程序如何变成可执行步骤 |
| 4 | [ARCHITECTURE.md](ARCHITECTURE.md) | 源码目录怎么对应编译各阶段 |
| 5 | [TOOLS.md](TOOLS.md) | 字节码 `.lgb`、类 JAR 包 `.lgpack`、全部 CLI |
| — | [GLOSSARY.md](GLOSSARY.md) | 英文标识符 / 编译术语 ↔ 中文 |

根目录 [README.md](../README.md) 只是仓库门面，细节以本文档树为准。

> 语言规则的「权威定义」在 [LANGUAGE.md](LANGUAGE.md)；若文档与代码不一致，以代码 + 测试为准，并欢迎改文档。
