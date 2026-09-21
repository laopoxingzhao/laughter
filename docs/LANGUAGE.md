# Laughter 语言手册

> **本文档由 MiMo（小米 AI 助手）撰写整理**，权威语义以本文件 + 代码测试为准。署名见 [AUTHORS.md](AUTHORS.md)。

面向初学者的完整语言说明。想先跑起来请看 [GETTING_STARTED.md](GETTING_STARTED.md)；  
想理解「为什么这样设计、代码如何执行」请看 [COMPILE_PRIMER.md](COMPILE_PRIMER.md)。

---

## 0. 一分钟认识

```text
// 这是注释，程序不会执行它
fun main() -> void {
    let name = "Laughter";
    print("hello, " + name);

    let nums: int[] = [1, 2, 3];
    for n in nums {
        print(n);
    }
}
```

- 源文件后缀：`.lg`
- 花括号 `{}` 包住代码块
- 大多数语句以 `;` 结束
- 可选 `fun main() -> void` 作为程序入口；没有 `main` 则从文件顶部语句顺序执行

---

## 1. 类型（数据分哪几种）

| 写法 | 中文名 | 例子 | 说明 |
|------|--------|------|------|
| `int` | 整数 | `1`, `-3`, `42` | 64 位整数 |
| `float` | 浮点数 | `1.5`, `3.14` | 可带小数 |
| `bool` | 布尔 | `true`, `false` | 真/假，常用于条件 |
| `string` | 字符串 | `"hi"` | 文本 |
| `T[]` | 数组 | `[1,2,3]` | 一串同类型元素 |
| `struct` | 结构体 | 见下文 | 把多个字段捆成一种类型 |
| `void` | 无返回值 | 函数「不返回东西」 | 只出现在函数返回类型位置 |

### 重要规则：不能混用 int 与 float

```text
let x = 1 + 2.5;   // 错误：不能将 int 与 float 相加
```

必须自己转换语义（当前语言未提供隐式转换；教学版也不提供 `to_int` 等）。

### 数组是「遥控器」，结构体是「复印件」

- **数组**：`let b = a;` 后，改 `b[0]` 也会影响 `a[0]`（同一份数据）。
- **结构体**：`let b = a;` 是**整份字段拷贝**；再改 `b.x` 不会影响 `a.x`。

---

## 2. 变量与常量

### let：声明变量

```text
let x: int = 1;    // 写明类型
let y = 2;         // 不写类型时，取右边值的类型
x = x + y;         // 赋值
```

### const：编译期就定死的常量

```text
const N: int = 10;
const M: int = N * 2 + 1;   // 编译期就算成 21
print(M);
```

- 只能写在**文件顶层**
- 必须写类型
- 右边只能是「编译期就能算出来」的式子（字面量、其它 const、加减乘除等）
- 不能对 `N = 1` 这样赋值

---

## 3. 运算符

| 类别 | 符号 | 例子 |
|------|------|------|
| 算术 | `+ - * / %` | `1 + 2`，`7 / 2`，`7 % 2`（取余） |
| 比较 | `== != < <= > >=` | `a < b` 结果是 `bool` |
| 逻辑 | `&& \|\| !` | `if x > 0 && x < 10` |
| 字符串 | `+` | `"a" + "b"` → `"ab"` |

- 逻辑运算有**短路**：`false && f()` 不会调用 `f`
- 整数除法向零取整；**除数为 0** 会在运行时报错

---

## 4. 条件与循环

### if

```text
if n < 0 {
    print("负数");
} else if n == 0 {
    print("零");
} else {
    print("正数");
}
```

条件必须是 `bool`。

### while

```text
let i = 0;
while i < 3 {
    print(i);
    i = i + 1;
}
```

### 遍历数组 / 数字范围

```text
let a: int[] = [10, 20];
for x in a {
    print(x);
}

for i in 0..3 {    // 半开区间：0,1,2（不包含 3）
    print(i);
}
```

### break 与 continue

```text
for i in 0..10 {
    if i == 2 { continue; }  // 跳过本次，进入下一轮
    if i == 5 { break; }     // 整个循环结束
    print(i);
}
```

---

## 5. 函数

```text
fun add(a: int, b: int) -> int {
    return a + b;
}

fun main() -> void {
    print(add(1, 2));   // 3
}
```

- 参数类型、返回类型都要写全  
- `-> void` 表示不返回值，函数里用 `return;` 或不写 return（最后一行）  
- 非 void 函数：**所有路径**都要有 `return`（否则编译错误）  
- 可以递归（函数调用自己），如 `examples/fib.lg`  
- 没有嵌套函数、没有「把函数当值传来传去」  

### 入口 main

```text
fun main() -> void {
    // 程序从这里开始（若文件里存在 main）
}
```

有 `main` 时：文件顶层的其它语句**不会执行**（只合并声明）。  
没有 `main` 时：从文件第一条顶层语句执行到最后。

---

## 6. 字符串与内建函数

语言自带这些**内建**（不能自己重名定义）：

| 函数 | 作用 | 例子 |
|------|------|------|
| `print(v)` | 打印并换行（CLI 输出一行） | `print(1);` |
| `len(s或数组)` | 长度 | `len("ab")==2`，`len([1,2])==2` |
| `str_at(s, i)` | 第 i 个字符（从 0 起） | `str_at("hi",0)==\"h\"` |
| `str_sub(s, start, n)` | 从 start 起取 n 个字符 | `str_sub("hello",1,3)==\"ell\"` |
| `to_string(v)` | 转成字符串 | `to_string(42)==\"42\"` |
| `push(数组, 值)` | 追加到数组末尾 | `push(a, 4);` |
| `pop(数组)` | 弹出并返回末元素 | `print(pop(a));` |
| `input()` | 从键盘/标准输入读一行 | `let s = input();` |

数组越界、`pop` 空数组、除零等 → **运行时错误**（程序中断并打印中文信息）。

---

## 7. 数组

```text
let a: int[] = [1, 2, 3];
print(a[0]);          // 读
a[1] = 20;            // 写
push(a, 4);           // 变成 [1,20,3,4]
print(len(a));        // 4

let empty: int[] = [];   // 空数组必须写类型标注
```

- 元素类型必须一致（不能 `[1, \"a\"]`）  
- 下标从 `0` 开始  

---

## 8. 结构体

把多个数据捆成一种新类型。

```text
struct Point {
    x: int,
    y: int,
}

fun main() -> void {
    let p = Point { x: 1, y: 2 };
    print(p.x);       // 1
    p.y = 30;         // 改字段
    print(p.y);       // 30
}
```

- 字面量必须**写全所有字段**（顺序可乱）  
- 结构体赋值/传参是**拷贝**（见第 1 节）  

### 方法（写在结构体上的函数）

```text
struct Point {
    x: int,
    y: int,
}

fun Point.sum(self: Point) -> int {
    // self 就是「这个点自己」，按值传入
    return self.x + self.y;
}

fun main() -> void {
    let p = Point { x: 1, y: 2 };
    print(p.sum());        // 3
    print(Point.sum(p));   // 也可以这样写，含义相同
}
```

---

## 9. 多文件（import）

把代码拆到多个 `.lg` 文件。

**math.lg**

```text
fun add(a: int, b: int) -> int {
    return a + b;
}
```

**main.lg**

```text
import "math.lg";
// 或带命名空间：import "math.lg" as math;  然后写 math.add(1,2)

fun main() -> void {
    print(add(2, 3));
}
```

- 路径相对**当前文件**所在目录  
- 不能写 `..`（防止跑到项目外）  
- 只合并函数/结构体/常量等**声明**，不会执行被导入文件的顶层语句  
- 带 `import` 的程序必须用**文件路径**运行：`cargo run -- run main.lg`  

---

## 10. 注释

```text
// 单行注释，到本行结束
```

---

## 11. 错误信息怎么读

```text
main.lg:3:5: 错误: 未定义的变量 `x`
```

| 片段 | 含义 |
|------|------|
| `main.lg` | 哪个文件 |
| `3` | 第几行 |
| `5` | 第几列 |
| `错误` | 编译期错误（运行前发现） |
| 后面文字 | 具体原因 |

运行时则是 `运行时错误:`，例如除零、下标越界。

---

## 14. 指针（安全引用）

| 写法 | 含义 |
|------|------|
| `&T` | 指向 `T` 的**只读**引用（可 `nil`） |
| `&mut T` | 指向 `T` 的**可写**引用 |
| `&x` / `&mut x` | 取变量的地址（本期主要支持变量） |
| `*p` | 解引用读取 |
| `*p = v` | 通过 `&mut` 写入；只读引用赋值 → 编译错误 |
| `nil` | 空引用；解引用 nil → 运行时错误 |

```text
fn main() -> void {
    let x = 1;
    let m: &mut int = &mut x;
    *m = 42;
    print(x);              // 42
    let p: &int = nil;
    print(p == nil);       // true
}
```

指针比较：`p == q` / `p == nil`（比较身份/是否为空）。  
**无**指针算术、无裸地址。

## 15. 现代化语法（融合）

| 特性 | 示例 |
|------|------|
| `fn` | 与 `fun` 等价：`fn add(a: int, b: int) -> int { ... }` |
| 隐式 return | 函数体最后一条**无分号**表达式：`{ a + b }` |
| 字符串插值 | `s"x={x}"` |
| 结构体简写 | `let p = Point { x, y };`（字段名与变量同名） |

```text
struct Point { x: int, y: int }

fn add(a: int, b: int) -> int {
    a + b
}

fn main() -> void {
    let n = 3;
    print(s"n={n}, sum={add(n, n)}");
    let x = 7;
    let y = 8;
    print(Point { x, y }.x);
}
```

旧的 `fun` / 显式 `return e;` / 完整字段写法仍然可用。

## 16. 目前还没有的功能（暂不支持）

- 枚举 / match、泛型、闭包、`impl` 块  
- 包管理、隐式 int↔float  
- 完整字符串库、文件与网络（除 `input`）  
- 指针算术、借用检查器  

---

## 17. 下一步

| 你想… | 请看 |
|-------|------|
| 把语言跑起来 | [GETTING_STARTED.md](GETTING_STARTED.md) |
| 懂编译器在干什么 | [COMPILE_PRIMER.md](COMPILE_PRIMER.md) |
| 在源码里找实现 | [ARCHITECTURE.md](ARCHITECTURE.md) |
| 字节码与打包 | [TOOLS.md](TOOLS.md) |
| 英文单词查中文 | [GLOSSARY.md](GLOSSARY.md) |
