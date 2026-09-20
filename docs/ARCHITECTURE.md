# 架构说明

## 编译管线

```text
.lg 源文件
  → syntax::lexer     Token 流（带行列）
  → syntax::parser    AST
  → sema::check       作用域 / 类型 / const 折叠
  → codegen::compile  栈式字节码 Chunk → Module（内存）
  → codegen::lgb      （可选）Module ↔ .lgb 磁盘文件
  → runtime::vm       执行 → print 输出或运行时错误

module_loader：解析 import，合并顶层声明后再走上述管线
CLI: run/check/disasm 可读源码；compile/exec 使用 .lgb
```

多文件时：`import` 在 loader 阶段展开为**单一 Program**（只合声明，不执行目标文件顶层语句），再检查与编译。

## 源码分层

```text
src/
  lib.rs              库入口：run_file/compile_file、run_path/compile_path、
                      run_source/compile_source、run_source_file 等
  syntax/
    mod.rs
    token.rs          TokenKind、Span
    lexer.rs          词法
    ast.rs            抽象语法树
    parser/
      mod.rs          Parser 状态与工具
      decl.rs         import / struct / const / fun
      stmt.rs         语句
      expr.rs         表达式（优先级）
  sema/
    mod.rs
    types.rs          语义类型 Type
    check/
      mod.rs          Checker 入口、声明与作用域
      fold.rs         const 折叠
      stmt.rs         语句/函数检查
      expr.rs         表达式与调用
  codegen/
    mod.rs
    op.rs             操作码
    chunk.rs          Chunk / Function / Module / 反汇编
    compile/
      mod.rs          Compiler 总控
      stmt.rs         语句/循环/赋值
      expr.rs         表达式与调用
    lgb.rs            .lgb 文件编解码
    lgpack.rs         .lgpack 类 JAR 包
  runtime/
    mod.rs
    value.rs          Value（含值语义 StructVal）
    vm/
      mod.rs          Vm、帧、错误
      exec.rs         主解释循环
      source.rs       run_source 单文件 API
  module_loader.rs    import 加载与符号合并
  bin/laughter.rs     CLI
```

`run_source` / `compile_source` **拒绝**含 `import` 的源码；多文件请用 `run_file` / CLI。

## 关键契约（与 LANGUAGE.md 一致）

| 主题 | 实现约定 |
|------|----------|
| 局部变量 | 函数帧内栈槽；`let` 压栈后登记槽位 |
| `const` | sema 折叠后 codegen 直接 `Const` 字面量 |
| 结构体 | `Value::Struct(StructVal)` 值拷贝；字段写 `SetField` / `SetLocalField` |
| 方法 | 编译为 `Type.method` 直呼，或 `CallMethod` 按接收者运行时类型名查表 |
| `for` | 脱糖为下标 + while；`break`/`continue` 为向前 Jump 回填 |
| 数组 | `Rc<RefCell<Vec<Value>>>` 句柄，引用语义 |
| 空块 vs 字面量 | 仅 `{ ident :` 视为结构体字面量 |
| 诊断 | 编译期 `file:line:col: error:`；运行时 `file:line: runtime error:` |

## 零成本方向

- 常量不占运行时槽、不重复计算  
- 结构体无堆壳；赋值/传参为字段拷贝  
- 循环与方法无对象包装（无迭代器堆分配、无虚表）  

详见 [LANGUAGE.md](LANGUAGE.md) §7 与 [GETTING_STARTED.md](GETTING_STARTED.md)。

## 测试

- 单元：`src/**` 内 `#[cfg(test)]`（词法、解析等）
- 集成：`tests/programs.rs` 对照契约与全部 `examples/*.lg`
