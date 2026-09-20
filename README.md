# Laughter

教学向编译语言：静态类型、C/Rust 风语法，编译为栈式字节码并由 VM 执行。

| | |
|--|--|
| 语言契约 | [`docs/LANGUAGE.md`](docs/LANGUAGE.md) |
| 快速开始 | [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) |
| 架构 | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| 文档索引 | [`docs/README.md`](docs/README.md) |
| 源文件后缀 | `.lg` |
| 仓库 | https://github.com/laopoxingzhao/laughter |
| 许可证 | [MIT](LICENSE) |
| CI | GitHub Actions：`cargo fmt --check` + `cargo test` + `cargo build --release` |

## 试用

```bash
cargo test
cargo run -- run examples/hello.lg
cargo run -- run examples/fib.lg
cargo run -- disasm examples/zca.lg
```

## 一分钟了解

- 类型：`int` `float` `bool` `string` `T[]` `struct` `void`
- `let` / `const`（编译期折叠）/ 值语义结构体 / 方法
- `if` / `while` / `for-in` / `for i in a..b` / `break` / `continue`
- `import "f.lg"` 或 `import "f.lg" as ns`
- 内建：`print` `len` `str_at` `str_sub` `to_string` `push` `pop` `input`
- 零成本约定：常量折叠、栈上结构体、循环脱糖、方法直呼

示例在 `examples/`，细节以 **LANGUAGE.md** 为准。
