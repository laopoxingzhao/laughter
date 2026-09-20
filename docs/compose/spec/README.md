# Compose 设计文档索引

本目录是 **Compose Next 工作流**留下的功能设计/交付记录，按时间从旧到新。  
**现行语言语义以 [`../../LANGUAGE.md`](../../LANGUAGE.md) 为准**；若与历史 spec 冲突，以 LANGUAGE.md 与当前代码为准。

| 文档 | 主题 | 状态 |
|------|------|------|
| [laughter-mvp.md](laughter-mvp.md) | 语言 MVP：词法/语法/类型/字节码/VM | delivered（历史） |
| [lang-mvp-plus.md](lang-mvp-plus.md) | 结构体、字符串 API、push/pop、for-in、input | delivered（历史） |
| [lang-next.md](lang-next.md) | 范围 for、break/continue、方法、import | delivered（历史） |
| [zca-structs-const.md](zca-structs-const.md) | 值语义结构体 + const 折叠 | delivered（历史） |
| [lang-rewrite.md](lang-rewrite.md) | 特性汇总 + 分层包彻底重写（契约 v2） | delivered |

新功能若再走 Compose，请在本目录新增 `docs/compose/spec/<feature>.md`，并在交付后回写本索引。
