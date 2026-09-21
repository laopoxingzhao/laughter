# 快速开始

> **本文档由 MiMo（小米 AI 助手）撰写**，面向未接触过编译器的同学。署名见 [AUTHORS.md](AUTHORS.md)。

目标：10 分钟内在你的电脑上跑起来 Laughter。

## 1. 你需要什么

- **Windows / macOS / Linux** 均可
- **Rust**（自带 `cargo`）  
  - 安装：<https://rustup.rs>（一直下一步即可）  
  - 验证：终端执行 `cargo -V`，能看到版本号即可  
- 可选：**VS Code** + 扩展 **rust-analyzer**（看 Rust 代码有补全）

## 2. 取得代码并编译

在仓库根目录（有 `Cargo.toml` 的那一层）执行：

```bash
cargo build
cargo test
```

- `cargo build`：编译本项目（生成命令行工具 `laughter`）  
- `cargo test`：跑测试，确认一切正常  
- 若 `cargo test` 显示 `test result: ok`，说明环境没问题  

## 3. 运行第一个程序

仓库里已有示例，可直接跑：

```bash
cargo run -- run examples/hello.lg
```

应输出：

```text
hello, laughter
```

再试一个稍微复杂一点的（递归求斐波那契）：

```bash
cargo run -- run examples/fib.lg
```

输出应为 `55`（fib(10) = 55）。

### 自己写一个文件

新建文本文件 `hello_main.lg`，内容：

```text
fun main() -> void {
    print("你好，Laughter");
}
```

执行：

```bash
cargo run -- run hello_main.lg
```

> **`cargo run --` 是什么意思？**  
> `cargo run` 会编译并运行本项目里的命令行工具；`--` 后面的参数原样传给该工具。  
> 若你已把 `laughter` 装进系统 PATH，也可以直接写 `laughter run hello_main.lg`。

## 4. 五个最常用的命令

| 命令 | 作用 |
|------|------|
| `cargo run -- run 文件.lg` | 检查 + 编译 + 执行源码 |
| `cargo run -- check 文件.lg` | 只检查和编译，不执行（看有没有类型错误） |
| `cargo run -- disasm 文件.lg` | 打印「字节码」——给虚拟机看的指令清单 |
| `cargo run -- compile 文件.lg` | 把字节码写到磁盘（生成 `.lgb`） |
| `cargo run -- pack 文件.lg` | 打成类似 Java JAR 的包（`.lgpack`） |

更多命令见 [TOOLS.md](TOOLS.md)。

## 5. 示例一览

| 文件 | 演示内容 |
|------|----------|
| `examples/hello.lg` | 打印一句话 |
| `examples/vars.lg` | 变量与算术 |
| `examples/branch.lg` | if / while |
| `examples/fib.lg` | 函数与递归 |
| `examples/arrays.lg` | 数组 |
| `examples/struct.lg` | 结构体 |
| `examples/strings.lg` | 字符串操作 |
| `examples/forin.lg` | 遍历数组 |
| `examples/methods_range.lg` | 方法、范围循环、break/continue |
| `examples/zca.lg` | 常量折叠与「值语义」结构体 |
| `examples/mod_main.lg` + `mod_math.lg` | 多文件 import |
| `examples/pointers.lg` | 指针 `&` / `&mut` / `nil` |
| `examples/modern.lg` | `fn`、字符串插值、结构体简写、隐式 return |

## 6. 出错时怎么看

编译期错误示例：

```text
foo.lg:2:13: 错误: 不能将 `int` 与 `float` 相加
```

含义：文件 `foo.lg`，第 2 行第 13 列附近，类型不能混用 `int` 和 `float`。

运行时错误示例：

```text
x.lg:1: 运行时错误: 除数不能为零
```

## 7. 接下来读什么

- 语言怎么写 → [LANGUAGE.md](LANGUAGE.md)  
- 「编译」到底是什么 → [COMPILE_PRIMER.md](COMPILE_PRIMER.md)（强烈建议）  
- 术语英文对照 → [GLOSSARY.md](GLOSSARY.md)  

若某一步卡住，把**完整命令 + 完整报错**发出来，比只说「不行了」更有帮助。
