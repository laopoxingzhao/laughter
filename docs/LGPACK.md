# `.lgpack` — 类似 JAR 的程序包

`.lgpack` 是一个 **ZIP 文件**，用来把「编译好的字节码 + 清单 + 可选资源」打成**一个文件**，方便分发和执行——概念上对齐 Java 的 **JAR**。

## 和 JAR 怎么对照

| Java JAR | Laughter `.lgpack` |
|----------|-------------------|
| ZIP 容器 | ZIP 容器（可用 `unzip -l` 查看） |
| `META-INF/MANIFEST.MF` | 同路径清单 |
| `.class` | `.lgb` 字节码 |
| `Main-Class: com.foo.App` | `Main-Bytecode: app.lgb` |
| 应用资源 | `resources/...`（`pack --resource` 打入） |
| `jar tf` / `jar xf` | `laughter list`（查看条目） |
| `java -jar app.jar` | `laughter exec app.lgpack` |

## 命令

```bash
# 打包：编译 examples/fib.lg，生成 examples/fib.lgpack
cargo run -- pack examples/fib.lg
cargo run -- pack examples/fib.lg -o target/fib.lgpack

# 带资源打包
cargo run -- pack examples/fib.lg -o target/app.lgpack --resource README.md

# 执行包
cargo run -- exec examples/fib.lgpack

# 查看包内文件列表
cargo run -- list examples/fib.lgpack

# 反汇编包内入口字节码
cargo run -- disasm examples/fib.lgpack
```

系统里若有 `unzip`：

```bash
unzip -l examples/fib.lgpack
```

## 包内结构（示例）

```text
fib.lgpack  (ZIP)
├── META-INF/MANIFEST.MF
├── app.lgb                 ← 入口字节码（清单里 Main-Bytecode 指向它）
└── resources/
    └── ...                 ← 可选资源
```

清单内容示例：

```text
Manifest-Version: 1.0
Created-By: laughter
Laughter-Package-Version: 1
Main-Bytecode: app.lgb
Bytecode-Format-Version: 1
```

## 注意

1. `pack` 时会**重新编译**入口 `.lg`（含 `import` 合并），包内通常只有**一份** `app.lgb`。  
2. `.lgpack` 仍需 `laughter exec` 执行（不是操作系统可直接加载的本地程序）。  
3. 实现依赖 crate `zip`；格式细节见 `src/codegen/lgpack.rs` 顶部注释。

## 相关测试

- `src/codegen/lgpack.rs`：清单解析、pack→exec 往返  
- `tests/programs.rs`：`lgpack_exec_roundtrip`  

与本文冲突时，以 **`src/codegen/lgpack.rs`** 为准。
