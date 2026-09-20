# 工具与交付格式（CLI / .lgb / .lgpack）

本文汇总命令行用法，以及两种「编译产物」文件。  
跑通示例请先看 [GETTING_STARTED.md](GETTING_STARTED.md)。

---

## 1. 命令行总表

在仓库根目录使用 `cargo run --`，或安装后直接用 `laughter`：

| 命令 | 含义 |
|------|------|
| `run <file.lg>` | 检查 + 编译 + 执行源码 |
| `check <file.lg>` | 检查 + 编译，不执行 |
| `compile <file.lg> [-o out.lgb]` | 生成字节码文件 `.lgb` |
| `exec <file>` | 执行 `.lg` / `.lgb` / `.lgpack` |
| `disasm <file>` | 反汇编（源码或字节码均可） |
| `pack <file.lg> [-o out.lgpack] [--resource f]...` | 打包类 JAR 包 |
| `list <file.lgpack>` | 列出包内文件（类似 `jar tf`） |
| `help` | 显示用法 |

示例：

```bash
cargo run -- run examples/hello.lg
cargo run -- compile examples/fib.lg
cargo run -- exec examples/fib.lgb
cargo run -- pack examples/fib.lg
cargo run -- list examples/fib.lgpack
cargo run -- exec examples/fib.lgpack
```

---

## 2. 什么时候用哪条命令？

| 场景 | 命令 |
|------|------|
| 日常写作业、跑示例 | `run` |
| 只想看有没有类型错误 | `check` |
| 学习字节码长什么样 | `disasm` |
| 想保存编译结果、反复执行 | `compile` + `exec` |
| 像分发 JAR 一样分发 | `pack` + `exec` |

---

## 3. `.lgb` 字节码文件

### 是什么？

把内存里的「函数表 + 指令 + 常量」写到磁盘，方便保存和再次执行。  
**不是**操作系统可直接双击运行的 exe，需要 `laughter exec`。

### 怎么生成与运行？

```bash
cargo run -- compile examples/fib.lg      # 生成 examples/fib.lgb
cargo run -- exec examples/fib.lgb
cargo run -- disasm examples/fib.lgb
```

### 文件里有什么？（白话）

| 部分 | 作用 |
|------|------|
| 魔数 `LGB1` | 标明「这是 Laughter 字节码」 |
| 版本号 | 当前为 1；版本不符会拒绝加载 |
| 入口信息 | 有没有 `main` 等 |
| 结构体布局 | 类型名与字段名顺序 |
| 函数表 | 每个函数的指令字节、常量池、行号表 |

更细的二进制布局见源码注释：`src/codegen/lgb.rs` 文件头。

### `.gitignore`

仓库已忽略 `*.lgb` / `*.lgpack`，避免把生成物提交进 git。

---

## 4. `.lgpack` 类 JAR 包

### 和 Java JAR 的对应

| JAR | Laughter |
|-----|----------|
| ZIP 容器 | ZIP 容器 |
| `META-INF/MANIFEST.MF` | 同路径清单 |
| `.class` | 包内的 `.lgb` |
| `Main-Class` | 清单里的 `Main-Bytecode` |
| `java -jar x.jar` | `laughter exec x.lgpack` |
| `jar tf` | `laughter list` |

### 打包与执行

```bash
cargo run -- pack examples/mod_main.lg -o target/demo.lgpack
cargo run -- list target/demo.lgpack
# 应能看到：
#   META-INF/MANIFEST.MF
#   app.lgb

cargo run -- exec target/demo.lgpack
```

带资源文件：

```bash
cargo run -- pack examples/fib.lg -o target/app.lgpack --resource README.md
```

资源会放在包内 `resources/` 目录下（当前教学版**不会**自动把资源读进语言运行时，主要用于演示「包里可以放东西」）。

### 清单示例

```text
Manifest-Version: 1.0
Created-By: laughter
Laughter-Package-Version: 1
Main-Bytecode: app.lgb
Bytecode-Format-Version: 1
```

实现与注释：`src/codegen/lgpack.rs`。

---

## 5. 反汇编怎么看？

```bash
cargo run -- disasm examples/hello.lg
```

输出形如：

```text
== <toplevel> ==
0000 L1    Const 0 (hello, laughter)
0003 L1    Print
0004 L0    Return
constants:
  [0] hello, laughter
```

| 字段 | 含义 |
|------|------|
| `0000` | 指令在字节码里的地址 |
| `L1` | 对应源码大致行号 |
| `Const 0` | 常量表下标 0 |
| `(...)` | 该常量的显示内容 |

指令名与中文对照见 [GLOSSARY.md](GLOSSARY.md)；栈上如何变化见 [COMPILE_PRIMER.md](COMPILE_PRIMER.md)。

---

## 6. 库函数 vs CLI

| | CLI | 库 API |
|--|-----|--------|
| 多文件 import | 支持 | 用 `run_file` / `compile_file` |
| 字符串里写源码 | 不支持 | `run_source`（无 import） |
| 生成 .lgb / .lgpack | `compile` / `pack` | `codegen::lgb` / `lgpack` |

---

## 7. 相关文档

- [LANGUAGE.md](LANGUAGE.md) — 语言规则  
- [COMPILE_PRIMER.md](COMPILE_PRIMER.md) — 编译过程  
- [ARCHITECTURE.md](ARCHITECTURE.md) — 源码结构  
