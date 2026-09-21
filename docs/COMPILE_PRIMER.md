# 编译原理从零读懂（小白版）

> **本文档由 MiMo（小米 AI 助手）撰写**，用比喻和例子讲清「编译」到底在做什么。署名见 [AUTHORS.md](AUTHORS.md)。

> 你不需要先学过编译课。  
> 读完应能回答：**我写的一行 `print(1+2)` 是怎样变成程序输出 `3` 的？**

---

## 1. 先打个比方

把 Laughter 程序想成「菜谱」，计算机/虚拟机想成「只会按指令操作托盘（栈）的厨师」。

| 阶段 | 比方 | 产出 |
|------|------|------|
| 1. 词法 | 把菜谱拆成一个个词 | Token 序列 |
| 2. 语法 | 看懂句子结构：谁是主语、谁是修饰 | 语法树 AST |
| 3. 语义/类型 | 检查：「盐」能不能和「糖」这样混？ | 检查通过 + 常量表 |
| 4. 编译 | 翻译成厨师能执行的步骤清单 | 字节码 |
| 5. 虚拟机 | 厨师一步步做菜 | 打印结果 |

Laughter 的管线正是：

```text
.lg 源码 → 词法 → 语法 → 类型检查 → 字节码 → 栈式 VM → 输出
```

---

## 2. 阶段一：词法分析（Lexer）

### 做什么？

把一长串字符切成**一个个有意义的词**（Token）。

### 例子

源码：

```text
let x = 10;
```

切完大致是：

| Token | 中文含义 |
|-------|----------|
| `let` | 关键字：声明变量 |
| `x` | 标识符（变量名） |
| `=` | 赋值号 |
| `10` | 整数字面量 |
| `;` | 语句结束 |

### 为什么要单独一步？

后面阶段不用再纠结「`x=` 是一个词还是两个」。词法还负责：

- 跳过空格和 `//` 注释  
- 记录**行号、列号**（报错要指到源码位置）  

### 对应源码

`syntax/lexer.rs`、`syntax/token.rs`  
可以自己动手：`cargo run -- disasm` 之前，程序其实已经先把 `.lg` 切成了 Token。

---

## 3. 阶段二：语法分析（Parser）

### 做什么？

把 Token **组装成语法树**，表达「谁套着谁、谁先谁后」。

### 例子

```text
1 + 2 * 3
```

因为乘法优先级更高，树应该长这样（而不是 `(1+2)*3`）：

```text
      +
     / \
    1   *
       / \
      2   3
```

对应代码里的 AST（语法树）节点：

```text
Binary {
  op: Add,
  lhs: 1,
  rhs: Binary { op: Mul, lhs: 2, rhs: 3 }
}
```

### 递归下降是什么？

一种常见解析写法：**每个语法用一个函数**。

```text
表达式 → 项 + 项 → … → 数字/变量/调用
```

`parser` 里 `or → and → … → primary` 就是：  
先解析优先级低的；遇到运算符再解析优先级高的右边。

### 对应源码

`syntax/parser/`（`mod`/`decl`/`stmt`/`expr`）

---

## 4. 阶段三：语义与类型检查（Sema / Checker）

### 做什么？

语法「长得对」还不够，还要**意义正确**：

| 问题 | 例子 | 结果 |
|------|------|------|
| 变量从哪来？ | `print(x);` 但没有 x | 错误：未定义的变量 |
| 类型能不能一起算？ | `1 + 2.5` | 错误：int 与 float |
| 函数参数个数对吗？ | `add(1)` 只给一个参 | 错误 |
| break 能写在这里吗？ | 循环外的 `break` | 错误 |
| 函数有返回值却漏了 return？ | 空函数体却声明 `-> int` | 错误 |

### 「作用域」是什么？

```text
fun main() -> void {
    let x = 1;
    {
        let y = 2;
        print(x);   // 可以：内层能看见外层的 x
        print(y);   // 可以：y 在本块内
    }
    // print(y);    // 错误：y 已经离开作用域
}
```

检查器用「作用域栈」：进入块压一层，离开块弹一层。

### const 折叠

```text
const M: int = 10 * 2 + 1;
```

检查阶段会**在编译前**算出 `21`，记入常量表。  
后面编译器看到 `M`，直接发「常量 21」，运行时不再计算乘法。

### 对应源码

`sema/check/`（`mod` / `stmt` / `expr` / `fold`）、`sema/types.rs`

---

## 5. 阶段四：编译成字节码（Compiler）

### 什么是字节码？

一种**给虚拟机看的简化指令**，不是 CPU 原生机器码。  
用 `disasm` 可以人类可读地打印出来：

```bash
cargo run -- disasm examples/hello.lg
```

可能看到类似：

```text
== <toplevel> ==
0000 L1    Const 0 (hello, laughter)
0003 L1    Print
0004 L0    Return
```

含义：

1. 把常量表第 0 项（字符串）**压入栈**  
2. **Print**：从栈顶取出并记入输出  
3. **Return**：结束  

### 「栈」是什么？

像一摞盘子：**后放的先拿走**（后进先出）。

```text
Const 1        栈: [1]
Const 2        栈: [1, 2]
Add            栈: [3]        // 弹出 1、2，压回 3
Print          栈: []         // 弹出 3 去打印
```

所以 `1+2` 的字节码不是「一个加号按钮」，而是：

```text
压入 1 → 压入 2 → 弹出两个做加法 → 结果压回
```

这种「指令只和栈顶打交道」的机器叫 **栈机**。

### 局部变量放在哪？

函数被调用时，参数先在栈上。  
每个函数还有一块**局部槽**（可以理解为带编号的格子）：

- `GetLocal 0`：把 0 号槽的值**复制**到栈顶  
- `SetLocal 0`：用栈顶的值**覆盖** 0 号槽  

`let x = 1;` 大致对应：算出 1 → 占用一个槽 → 以后用 `GetLocal 槽号` 取 x。

### 跳转与 if

```text
if c { A } else { B }
```

字节码思路：

```text
计算 c          // 栈顶是 true/false
JumpIfFalse → B // 为假则跳
Pop             // 为真：弹掉条件，执行 A
... A ...
Jump → 结束
B: Pop          // 为假：弹掉条件，执行 B
... B ...
结束:
```

注意：`JumpIfFalse` **只看不弹**，所以两条路都要自己 `Pop`，否则栈会脏。

### 循环与 break

`for i in 0..n` 会被**脱糖**（改写）成带下标的 while，大致：

```text
i = 0
循环头:
  if !(i < n) goto 出口
  循环体
  i = i + 1
  goto 循环头
出口:
```

`break` / `continue` 先留下「待填的跳转」，循环编译完再把目标地址写回去。

### 函数调用

- `Call 函数下标`：压入新的一帧（记录：调用到哪个函数、指令位置、局部槽从哪开始）  
- `Return`：  
  - void：把栈截断到「本帧开始的地方」  
  - 有返回值：先拿结果，再截断，再把结果压回给调用方  

### 对应源码

`codegen/compile/`、`codegen/chunk.rs`、`codegen/op.rs`

---

## 6. 阶段五：虚拟机执行（VM）

虚拟机就是一个**死循环**：

```text
1. 取当前帧（正在跑哪个函数、ip 在哪）
2. 读 code[ip] 这个字节 → 操作码
3. 按操作码改栈 / 改帧
4. ip 前进
```

帧栈空了 → 程序结束，把 `print` 收集到的行交给 CLI 打印。

### 中文注释在哪？

打开 `src/runtime/vm/exec.rs`，每条指令分支都有中文说明，例如：

```text
// GetLocal <槽号>：复制局部变量到栈顶（槽里原值还在）
// Return：void 截断到 base；非 void 先拿返回值再压回
```

### 对应源码

`runtime/vm/`（`mod` / `exec` / `source`）、`runtime/value.rs`

---

## 7. 把五个阶段串起来（一张图）

```text
hello.lg
   │
   ▼
[Lexer]  词：print ( "hello" ) ;
   │
   ▼
[Parser] AST：调用 print，参数是字符串
   │
   ▼
[Checker] print 合法；类型 string 可以打印
   │
   ▼
[Compiler] Const "hello" ; Print ; Return
   │
   ▼
[VM] 栈操作 → 输出 hello
```

---

## 8. 动手实验（强烈建议）

### 实验 A：看字节码

```bash
cargo run -- disasm examples/hello.lg
cargo run -- disasm examples/zca.lg
```

观察 `zca.lg` 里 `const` 是否已经变成数字字面量（例如 `CONST 21`），而不是乘法指令。

### 实验 B：故意写错类型

```text
let x = 1 + 2.5;
```

```bash
cargo run -- run 你的文件.lg
```

看中文错误信息是否指向行列。

### 实验 C：结构体是不是复印件

```text
struct P { x: int }
fun main() -> void {
    let a = P { x: 1 };
    let b = a;
    b.x = 9;
    print(a.x);  // 若输出 1，则是拷贝语义
}
```

### 实验 D：打包再执行

```bash
cargo run -- pack examples/fib.lg
cargo run -- exec examples/fib.lgpack
cargo run -- list examples/fib.lgpack
```

---

## 9. 常见疑问

**Q：为什么不直接生成 .exe？**  
A：教学版选择「字节码 + 自己写的 VM」，每一步可打印、可打断点。真实工业编译器会走 LLVM 等生成机器码，路径更长。

**Q：字节码和 Java 的 class 一样吗？**  
A：思路类似（都是 VM 执行的指令），格式完全不同。我们的 `.lgb` / `.lgpack` 是教学格式。

**Q：一定要会 Rust 才能学这个项目吗？**  
A：写 `.lg` 程序不需要；读编译器源码需要一些 Rust。对照 [GLOSSARY.md](GLOSSARY.md) 可降低门槛。

**Q：类型检查和编译有什么区别？**  
A：检查决定「能不能编译」；编译决定「变成哪些指令」。检查失败就不会进入有意义的字节码。

---

## 10. 下一步

| 主题 | 文档 |
|------|------|
| 语言语法细节 | [LANGUAGE.md](LANGUAGE.md) |
| 源码目录对应关系 | [ARCHITECTURE.md](ARCHITECTURE.md) |
| CLI / 字节码文件 / 包 | [TOOLS.md](TOOLS.md) |
| 英文对照 | [GLOSSARY.md](GLOSSARY.md) |
