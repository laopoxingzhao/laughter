# 字节码文件 `.lgb`

Laughter 编译后会在内存里生成 `Module`。用 CLI 的 `compile` 可以把它写成磁盘上的 **`.lgb`** 文件，再用 `exec` 执行，不必每次重新解析源码。

## 命令

```bash
# 源码 → 字节码文件（默认 fib.lg → fib.lgb）
cargo run -- compile examples/fib.lg
cargo run -- compile examples/fib.lg -o out/fib.lgb

# 执行字节码
cargo run -- exec examples/fib.lgb
# 或
cargo run -- exec out/fib.lgb

# 反汇编：源码或 .lgb 都可以
cargo run -- disasm examples/fib.lg
cargo run -- disasm examples/fib.lgb
```

注意：

- `run` 只接受 `.lg` 源码  
- `exec` 只接受 `.lgb`  
- `.lgb` 是**本教学实现的格式**，版本号写在文件头；换版本号会导致无法加载  

## 文件里有什么（小白版）

可以把 `.lgb` 想成「打包好的菜谱」：

| 内容 | 说明 |
|------|------|
| 魔数 `LGB1` | 标识「这是 Laughter 字节码」，防止读错文件 |
| 版本号 | 当前为 `1`；程序会检查版本是否匹配 |
| 入口信息 | 有没有 `main`、顶层函数在哪 |
| 结构体表 | 类型名 + 字段名顺序（运行时建结构体用） |
| 函数表 | 每个函数的：名字、参数个数、是否 void、局部槽数、常量池、指令字节、行号表 |

常量池里目前只存 **整数 / 浮点 / 布尔 / 字符串**。  
数组和结构体不在文件里预先摆好，而是运行时由 `NewArray` / `NewStruct` 指令构造。

## 格式概要（版本 1）

小端（little-endian）二进制：

```text
"LGB1"                  4 字节魔数
u16 version             当前 1
u16 reserved            填 0
u16 main_index          0xFFFF 表示无 main
u16 toplevel_index

u16 struct_type_count
  每个结构体:
    u16 name_len + name
    u16 field_count
    每个字段: u16 len + name

u16 function_count
  每个函数:
    u16 name_len + name
    u8 arity
    u8 is_void (0/1)
    u16 locals
    u16 const_count
      每个常量: u8 标签
        0: i64   1: f64   2: bool(u8)   3: u16 长度 + UTF-8
    u32 code_len + code 字节
    u32 lines_len + 每行 u32
```

实现代码：[`src/codegen/lgb.rs`](../src/codegen/lgb.rs)（文件头注释里也有同样说明）。

## 和「真正编译器」的对比

| | Laughter `.lgb` | 例如 C 的 `.o` |
|--|------------------|----------------|
| 谁来执行 | 自带的栈式 VM | 操作系统 CPU |
| 是否机器码 | 否，是教学用指令 | 是 |
| 能否直接 `./a.out` | 否，需 `laughter exec` | 是 |

这正是教学语言的价值：字节码可读、可反汇编，每一步都能对上源码。

## 相关测试

- `src/codegen/lgb.rs` 内单元测试：`fib` 的 encode → decode → exec 应输出 `55`  
- `tests/programs.rs`：`compile_exec_roundtrip` 等  

详细设计若与本文不一致，以 **`src/codegen/lgb.rs` 文件头注释 + 代码** 为准。
