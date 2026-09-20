# Laughter

教学向玩具**编译语言**：静态类型、C/Rust 风语法，编译为自研字节码，由栈式 VM 执行。

| 项 | 内容 |
|----|------|
| 源文件后缀 | `.lg` |
| 宿主实现 | Rust + Cargo |
| 仓库 | https://github.com/laopoxingzhao/laughter |
| 许可证 | [MIT](LICENSE) |
| CI | GitHub Actions：`cargo fmt --check` + `cargo test` |
| 设计文档 | [`laughter-mvp.md`](docs/compose/spec/laughter-mvp.md)、[`lang-mvp-plus.md`](docs/compose/spec/lang-mvp-plus.md)、[`lang-next.md`](docs/compose/spec/lang-next.md)、[`zca-structs-const.md`](docs/compose/spec/zca-structs-const.md) |

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
| 类型 | `int` `float` `bool` `string` `T[]` `void` 以及 `struct`（无隐式转换） |
| 变量 | `let x: int = 1;`；`const N: int = 10;`（编译期折叠为字面量）；`x = ...;` |
| 控制流 | `if` / `else if` / `else` / `while` / `for x in arr` / `for i in a..b` / `break` / `continue` |
| 函数 | `fun name(p: T) -> R { ... }`，允许递归；有 `main` 时从 `main` 进入 |
| 结构体 | **值语义**拷贝；`struct Point { x: int, y: int }`；方法 `fun Point.sum(self: Point) -> int`（接收者按值） |
| 模块 | `import "math.lg";` 或 `import "math.lg" as math;`（`math.add`）；只合并声明，不执行目标文件顶层语句 |
| 数组 | 同构 `T[]`；`push`/`pop`；索引越界与空 `pop` 为运行时错误 |
| 内建 | `print` `len` `str_at` `str_sub` `to_string` `push` `pop` `input` |
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
| `examples/struct.lg` | 结构体字段、`for-in` |
| `examples/strings.lg` | 字符串 API |
| `examples/forin.lg` | `push`/`pop`/`for-in` |
| `examples/methods_range.lg` | 结构体方法、`for i in 0..n`、`break`/`continue` |
| `examples/zca.lg` | 值语义结构体 + `const` 折叠 |
| `examples/mod_main.lg` + `mod_math.lg` | 多文件 `import` |
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

## 零成本抽象（当前约定）

目标是让常用抽象**接近手写栈机指令**的成本，而不是运行时包装：

| 机制 | 成本约定 |
|------|----------|
| `const` | 初始化式在编译期求值；使用处是 `CONST 字面量`，无局部槽、无运行时运算 |
| 结构体 | `Value::Struct` 直接在栈/槽中；`let b = a` 为字段拷贝，**无 `Rc` 堆分配** |
| 字段写 `p.x = v` | `SetLocalField` 改栈槽内字段，不分配、不影响其它拷贝 |
| 方法 `p.m()` | 编译为直接 `Call`/`CallMethod`（按类型名查表），无虚表 |
| `for i in a..b` | 脱糖为下标 `while`，无迭代器对象 |
| `string` / 数组字段 | 仍是指针语义（`Rc` 句柄浅拷贝共享）——见 `docs/compose/spec/zca-structs-const.md` |

用 `laughter disasm examples/zca.lg` 可看到 `CONST 21` 等折叠结果。

## 本期不做

方法重载/`impl` 块、import 包管理、`break` 带值、标签循环、闭包/函数值、范围 `for` 以外的迭代器、GC、LLVM/原生可执行文件、REPL、语言服务器。

模块 `import` 需通过文件路径运行：`laughter run examples/mod_main.lg`（`run_source` 不解析 import）。
