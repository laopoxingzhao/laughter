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
cargo run -- compile examples/fib.lg          # 生成 examples/fib.lgb
cargo run -- exec examples/fib.lgb
cargo run -- disasm examples/zca.lg
cargo run -- disasm examples/fib.lgb
```

| 子命令 | 行为 |
|--------|------|
| `run <file.lg>` | 类型检查 → 编译 → 执行源码 |
| `check <file.lg>` | 类型检查 → 编译，不执行 |
| `compile <file.lg> [-o out.lgb]` | 编译并**写入字节码文件** `.lgb` |
| `exec <file.lgb>` | 执行字节码文件 |
| `disasm <file.lg\|file.lgb>` | 打印字节码反汇编与常量表 |

## 第一个程序

新建 `hello_main.lg`（或打开 `examples/hello.lg`，后者是**顶层** `print`，无 `main`）：

```text
fun main() -> void {
    print("hello, laughter");
}
```

```bash
cargo run -- run hello_main.lg
# 或
cargo run -- run examples/hello.lg
```

有 `fun main() -> void` 时从 `main` 进入；没有则执行文件顶层语句。

**模块注意**：CLI / `module_loader::run_file` 可处理 `import`；库函数 `run_source` / `compile_source` 只接受无 import 的单文件源码（含 import 会报错，请改用文件路径）。

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
若你不熟悉编译器，请接着读 [CODE_TOUR.md](CODE_TOUR.md)（栈、字节码、值语义的白话说明）。
