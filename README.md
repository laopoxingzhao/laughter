# Laughter

教学向编译语言：静态类型、C/Rust 风语法，编译为栈式字节码 VM。

| 项 | 内容 |
|----|------|
| 源文件 | `.lg` |
| 语言契约 | **[`docs/LANGUAGE.md`](docs/LANGUAGE.md)**（v2 重写定稿） |
| 实现 | Rust，分层：`syntax` / `sema` / `codegen` / `runtime` / `module_loader` |
| 仓库 | https://github.com/laopoxingzhao/laughter |
| 许可证 | [MIT](LICENSE) |
| CI | GitHub Actions：`cargo fmt --check` + `cargo test` |

## 构建与测试

```bash
cargo test
cargo run -- run examples/fib.lg
cargo run -- disasm examples/zca.lg
```

## CLI

```text
laughter run <file.lg>     # 检查 + 编译 + 执行
laughter check <file.lg>   # 检查 + 编译，不执行
laughter disasm <file.lg>  # 反汇编 + 常量表
```

## 语言一览（详见 LANGUAGE.md）

- 类型：`int` `float` `bool` `string` `T[]` `struct` `void`
- `let` / `const`（编译期折叠）/ 值语义结构体 / 方法
- `if` / `while` / `for-in` / `for i in a..b` / `break` / `continue`
- `import "f.lg"` 或 `as ns`
- 内建：`print` `len` `str_at` `str_sub` `to_string` `push` `pop` `input`
- 零成本约定：const 折叠、结构体栈上值、for 脱糖、方法直呼

## 示例

`examples/*.lg`（hello、fib、struct、zca、mod_main 等）均可 `laughter run`。

## 源码结构

```text
src/
  syntax/    token lexer ast parser
  sema/      types check（含 const 折叠）
  codegen/   op chunk compile
  runtime/   value vm
  module_loader.rs
  bin/laughter.rs
```

设计演进文档在 `docs/compose/spec/`。
