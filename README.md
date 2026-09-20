# Laughter

教学向玩具**编译语言**：静态类型、C/Rust 风语法，编译为自研字节码，由栈式 VM 执行。

- 源文件后缀：`.lg`
- 宿主实现：Rust + Cargo
- 设计文档：`docs/compose/spec/laughter-mvp.md`

## 构建与测试

```bash
cargo build
cargo test
```

## CLI

```bash
laughter run <file.lg>      # 类型检查 + 编译 + 执行
laughter check <file.lg>    # 只做词法/语法/语义检查
laughter disasm <file.lg>   # 打印字节码反汇编
```

示例：

```bash
cargo run -- run examples/hello.lg
cargo run -- run examples/fun.lg
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
    print(a[0]);
    print(len(a));
}
```

| 特性 | 说明 |
|------|------|
| 类型 | `int` `float` `bool` `string` `T[]` `void`（无隐式转换） |
| 控制流 | `if` / `else if` / `else` / `while` |
| 函数 | `fun name(p: T) -> R { ... }`，允许递归；有 `main` 时从 `main` 进入 |
| 内建 | `print(x)`、`len(arr)` |
| 运算符 | `+ - * / %` `== != < <= > >=` `&& \|\| !`；字符串 `+` 拼接 |

## 编译管线

```text
.lg → Lexer → Parser → TypeChecker → Compiler（栈式字节码）→ VM
```

目录：`src/lexer.rs` `src/parser.rs` `src/resolve.rs` `src/compiler.rs` `src/vm.rs`。

## Out of Scope（本期不做）

结构体/闭包/模块、隐式数值转换、完整字符串 API、GC、LLVM/原生可执行文件、REPL。
