# Laughter

教学向编译语言：静态类型、C/Rust 风语法，编译为字节码并由栈式虚拟机执行。

> **文档撰写**：MiMo（小米 AI 助手）。详见 [`docs/AUTHORS.md`](docs/AUTHORS.md)。

| | |
|--|--|
| 文档入口 | [`docs/README.md`](docs/README.md) |
| 快速开始 | [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) |
| 语言手册（含指针/现代语法） | [`docs/LANGUAGE.md`](docs/LANGUAGE.md) |
| 编译原理（小白） | [`docs/COMPILE_PRIMER.md`](docs/COMPILE_PRIMER.md) |
| 术语对照 | [`docs/GLOSSARY.md`](docs/GLOSSARY.md) |
| 源文件后缀 | `.lg` |
| 仓库 | https://github.com/laopoxingzhao/laughter |
| 许可证 | [MIT](LICENSE) |

## 试一下

```bash
cargo test
cargo run -- run examples/hello.lg
cargo run -- run examples/fib.lg
cargo run -- run examples/pointers.lg
cargo run -- run examples/modern.lg
cargo run -- disasm examples/hello.lg
```

## 语言速览

- 类型：`int` `float` `bool` `string` `T[]` `struct` `void` `&T` / `&mut T`
- 指针：`&x`、`&mut x`、`*p`、`nil`
- 现代语法：`fn`、`s"..."` 插值、`Point { x, y }`、隐式 return
- `let` / `const` / 函数与方法 / 数组 / `import`
- 控制流：`if` / `while` / `for-in` / `for i in a..b` / `break` / `continue`
- 内建：`print` `len` `str_at` `str_sub` `to_string` `push` `pop` `input`
- 交付：`.lgb` 字节码、`.lgpack` 类 JAR 包（[`docs/TOOLS.md`](docs/TOOLS.md)）

细节以 **`docs/`** 为准；示例在 `examples/`。
