# 代码导读（给编译原理小白）

按「一个 `.lg` 文件从打开到打印结果」的顺序读代码即可。  
契约见 [LANGUAGE.md](LANGUAGE.md)，模块地图见 [ARCHITECTURE.md](ARCHITECTURE.md)。  
英文类型名看不懂时，查 [TYPES_GLOSSARY.md](TYPES_GLOSSARY.md)。

## 0. 背景词汇

| 词 | 白话解释 |
|----|----------|
| Token | 语法里的「一个词」：`let`、`x`、`=`、`1`、`;` |
| AST | 抽象语法树，把程序画成树（表达式嵌套、语句顺序） |
| 类型检查 | 编译前确认「int 不能和 float 直接相加」这类问题 |
| 字节码 | 给虚拟机看的简化指令列表，像菜谱步骤 |
| 栈（stack） | 后进先出的托盘：后放的数先被拿走 |
| 栈机 | 大多数指令都从栈上取数、算完再压回栈的虚拟机 |

## 1. 建议阅读顺序

```text
examples/hello.lg
  → src/syntax/lexer.rs     源码如何被切成 Token
  → src/syntax/parser.rs    Token 如何变成 AST
  → src/sema/check.rs       类型是否合法、const 何时折叠
  → src/codegen/compile.rs  AST 如何变成指令
  → src/runtime/vm.rs       指令如何在栈上执行
  → src/module_loader.rs    多文件 import 如何合并
```

## 2. 用 `disasm` 对照字节码

```bash
cargo run -- disasm examples/hello.lg
cargo run -- disasm examples/zca.lg
```

`hello.lg` 里只有顶层 `print("hello, laughter");`，反汇编大致是：

```text
CONST  "hello, laughter"   // 把字符串常量压栈
PRINT                      // 从栈顶弹出并记入输出
RETURN
```

`zca.lg` 里 `const M = N * 2 + 1` 在**编译期**就算好，使用处会变成 `CONST 21`，不会看到乘法指令。

## 3. 栈上发生了什么（最小例子）

源码：

```text
let x = 1;
let y = 2;
x = x + y;
```

字节码意图（示意）：

```text
CONST 1          ; 栈: [1]           ← x 的槽位
CONST 2          ; 栈: [1, 2]        ← y 的槽位
GET_LOCAL x      ; 栈: [1, 2, 1]
GET_LOCAL y      ; 栈: [1, 2, 1, 2]
ADD              ; 栈: [1, 2, 3]
SET_LOCAL x      ; 把栈顶 3 写回 x 的槽
POP              ; 弹掉运算结果副本
```

- **局部变量槽**：函数开始时参数占最下面几格；`let` 往栈上再压一格，就形成一个「槽」。
- **`GET_LOCAL`**：复制槽里的值到栈顶（槽还在）。
- **`SET_LOCAL`**：用栈顶的值覆盖某个槽（栈顶可能仍在，所以后面常接 `POP`）。

## 4. 条件与跳转

`if cond { ... }`：

```text
计算 cond          ; 栈顶是 true/false
JUMP_IF_FALSE else ; 若 false，跳到 else 侧（指令本身不弹栈）
POP                ; true 路径：弹掉条件
... then 体 ...
JUMP end
else:
POP                ; false 路径：弹掉条件
end:
```

**要点**：`JUMP_IF_FALSE` / `JUMP_IF_TRUE` 只「看」栈顶，不弹出；  
所以每条离开条件的路径都要自己 `POP`，否则栈会乱。

循环里的 `break`/`continue`：先发射一个占位 `JUMP`，循环收尾时把偏移写回去（`patch_to`）。

## 5. 结构体为什么是「值语义」

- 数组：`Rc<RefCell<Vec<Value>>>`，像「遥控器」——多个变量可以控制同一份数据。
- 结构体：整份字段拷贝，像「复印件」——`let b = a` 后改 `b.x` 不影响 `a.x`。

`p.x = 1` 的实现思路：

1. 从局部槽取出结构体（或在槽上直接改）；
2. 修改字段；
3. 写回槽位（`SetLocal` / `SetLocalField`）。

## 6. 方法调用

```text
fun Point.sum(self: Point) -> int { return self.x + self.y; }
```

- 编译后函数名是 `Point.sum`，第一个参数就是 `self`（按值传入）。
- `p.sum()`：VM 看栈上接收者的**运行时类型名**是 `Point`，再去找 `Point.sum`。
- `Point.sum(p)`：编译期就能定位函数，直接 `Call`。

方法里改 `self.x` 只改到副本，调用方的 `p` 不变（值语义）。

## 7. const 何时算

`sema/check.rs` 的 `fold`：对常量表达式在**编译期**算出 `Value`，放进常量表；  
`codegen` 在看到这些名字时直接 `emit_const`，运行时不再读变量、也不再算一次。

## 8. import

`module_loader.rs`：

1. 读主文件，收集 `import`；
2. 递归读入被导入文件（禁止 `..`，用栈防环）；
3. 把声明（struct/const/fun）合并进一个大 AST；
4. **不执行**被导入文件的顶层语句；
5. 再走检查 → 编译 → 运行。

因此：带 `import` 的程序必须用 `laughter run 文件路径`，不能用只接受字符串的 `run_source`。

## 9. 自己动手改

建议实验（改完 `cargo run -- run ...`）：

1. 把 `examples/zca.lg` 里 `const M` 的表达式改复杂，用 `disasm` 看是否仍是字面量。  
2. 写两个结构体变量，赋值后分别改字段，观察是否互不影响。  
3. 在 `for` 里加 `break`/`continue`，用 `disasm` 看跳转。  
4. 故意写 `let x = 1 + 2.5;`，看类型错误信息。

读不懂某一段时，优先看该文件顶部的 `//!` 模块说明，再对照本导读。
