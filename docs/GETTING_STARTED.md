# 快速开始

## 环境

- Rust stable（含 `cargo`）
- 可选：VS Code + rust-analyzer

## 构建与测试

```bash
cd laughter
cargo build
cargo test
```

## CLI

在仓库根目录（尚未安装到 PATH 时用 `cargo run --`）：

```bash
cargo run -- run examples/hello.lg
cargo run -- check tests/fixtures/type_error.lg
cargo run -- disasm examples/zca.lg
```

| 子命令 | 行为 |
|--------|------|
| `run <file.lg>` | 类型检查 → 编译 → 执行 |
| `check <file.lg>` | 类型检查 → 编译，不执行 |
| `disasm <file.lg>` | 打印字节码反汇编与常量表 |

## 第一个程序

`hello.lg`：

```text
fun main() -> void {
    print("hello, laughter");
}
```

```bash
cargo run -- run hello.lg
```

有 `fun main() -> void` 时从 `main` 进入；没有则执行文件顶层语句。

## 示例一览

| 文件 | 演示 |
|------|------|
| `examples/hello.lg` | 打印 |
| `examples/vars.lg` | 变量与表达式 |
| `examples/branch.lg` | if / while |
| `examples/fib.lg` / `fun.lg` | 函数与递归 |
| `examples/arrays.lg` | 数组 |
| `examples/struct.lg` | 结构体与 for-in |
| `examples/strings.lg` | 字符串内建 |
| `examples/forin.lg` | push/pop + for-in |
| `examples/methods_range.lg` | 方法、范围 for、break/continue |
| `examples/zca.lg` | 值语义结构体 + const 折叠 |
| `examples/mod_main.lg` + `mod_math.lg` | 多文件 import |

语法与语义细节以 [LANGUAGE.md](LANGUAGE.md) 为准。
