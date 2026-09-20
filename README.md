# Laughter

教学向玩具**编译语言**：静态类型、C/Rust 风语法，编译为自研字节码，由栈式 VM 执行。

| 项 | 内容 |
|----|------|
| 源文件后缀 | `.lg` |
| 宿主实现 | Rust + Cargo |
| 仓库 | https://github.com/laopoxingzhao/laughter |
| 设计文档 | [`docs/compose/spec/laughter-mvp.md`](docs/compose/spec/laughter-mvp.md) |

## 构建与测试

```bash
cargo build
cargo test
```

## CLI

```bash
laughter run <file.lg>      # 类型检查 + 编译 + 执行
laughter check <file.lg>    # 词法/语法/语义 + 编译到字节码，不执行
laughter disasm <file.lg>   # 打印字节码反汇编与常量表
```

在仓库根目录用 `cargo run --` 代替安装后的 `laughter`：

```bash
cargo run -- run examples/hello.lg
cargo run -- run examples/fib.lg
cargo run -- check tests/fixtures/type_error.lg
cargo run -- disasm examples/arrays.lg
```

## 语言速览

```text
fun fib(n: int) -> int {
    if n < 2 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fun main() -> void {
    let a: int[] = [1, 2, 3];
    a[0] = fib(10);
    print(a[0]);       // 55
    print(len(a));     // 3
}
```

| 特性 | 说明 |
|------|------|
| 类型 | `int` `float` `bool` `string` `T[]` `void`（`int`/`float` 不隐式转换） |
| 变量 | `let x: int = 1;` 或 `let y = 2;`（类型取自初始化式）；`x = ...;` 赋值 |
| 控制流 | `if` / `else if` / `else` / `while` |
| 函数 | `fun name(p: T) -> R { ... }`，允许递归；存在 `main() -> void` 时从 `main` 进入，否则跑顶层语句 |
| 数组 | 同构 `T[]`；`let a: int[] = [];` 合法；索引越界为运行时错误 |
| 内建 | `print(x)`、`len(arr)`（不可重定义） |
| 运算符 | `+ - * / %` `== != < <= > >=` `&& \|\| !`；字符串 `+` 拼接 |
| 诊断 | 编译期 `file:line:col: error: ...`；运行时 `file:line: runtime error: ...` |

## 示例程序

| 文件 | 内容 |
|------|------|
| `examples/hello.lg` | 打印字面量 |
| `examples/vars.lg` | 变量、表达式、字符串 |
| `examples/branch.lg` | `if`/`while` |
| `examples/fun.lg` / `fib.lg` | 用户函数与递归 |
| `examples/arrays.lg` | 数组字面量、索引、`len` |
| `tests/fixtures/type_error.lg` | 应被拒绝的类型错误样例 |

## 编译管线与源码地图

```text
.lg → Lexer → Parser → Checker → Compiler（栈式字节码）→ VM
```

| 模块 | 职责 |
|------|------|
| `src/token.rs` / `lexer.rs` | Token、Span、词法 |
| `src/ast.rs` / `parser.rs` | AST、递归下降语法分析 |
| `src/types.rs` / `resolve.rs` | 语义类型、作用域与类型检查 |
| `src/bytecode.rs` / `compiler.rs` | 操作码、Chunk、AST→字节码 |
| `src/value.rs` / `vm.rs` | 运行时值、栈式 VM、`run_source` |
| `src/lib.rs` / `src/bin/laughter.rs` | 库入口与 CLI |

## 本期不做

结构体/闭包/模块 import、隐式数值转换、完整字符串 API、GC、LLVM/原生可执行文件、REPL、语言服务器。
