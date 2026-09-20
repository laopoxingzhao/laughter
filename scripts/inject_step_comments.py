#!/usr/bin/env python3
"""Inject step-by-step Chinese comments into key source files."""
from pathlib import Path

def patch(path, pairs):
    p = Path(path)
    t = p.read_text(encoding="utf-8")
    for old, new in pairs:
        if old not in t:
            print(f"  MISS in {path}: {old[:50]!r}...")
            continue
        if t.count(old) != 1:
            print(f"  MULTI in {path}: {old[:50]!r} count={t.count(old)}")
            continue
        t = t.replace(old, new, 1)
    p.write_text(t, encoding="utf-8")
    print("patched", path)

# ---- VM exec: comment each opcode arm ----
exec_path = Path("src/runtime/vm/exec.rs")
t = exec_path.read_text(encoding="utf-8")
t = t.replace(
    "//! 主解释循环。\nuse super::*;\n\nimpl<'m> Vm<'m> {\n    pub(crate) fn loop_run(&mut self, out: &mut Vec<String>) -> Result<Vec<String>, VmError> {\n        loop {\n",
    """//! 主解释循环：一步一步执行字节码。
//!
//! 每次循环做四件事：
//! 1. 取当前帧（正在跑的函数 + 指令位置 ip + 局部槽基址 base）
//! 2. 读 `code[ip]` 这个字节，翻译成操作码 `Op`
//! 3. 按操作码改栈 / 改帧（见各分支中文注释）
//! 4. 把 `ip` 推进到下一条指令
//!
//! 帧栈空了 → 程序结束，返回 `print` 收集到的行。

use super::*;

impl<'m> Vm<'m> {
    pub(crate) fn loop_run(&mut self, out: &mut Vec<String>) -> Result<Vec<String>, VmError> {
        loop {
            // 步骤1：没有帧 = 主程序已返回
""",
)

replacements = [
(
"            let Some(fr) = self.frames.last() else {\n                return Ok(std::mem::take(out));\n            };\n            let func = fr.func;\n            let ip = fr.ip;\n            let base = fr.base;\n",
"""            let Some(fr) = self.frames.last() else {
                return Ok(std::mem::take(out));
            };
            // 步骤2：读出当前执行位置（函数编号、指令指针、局部基址）
            let func = fr.func;
            let ip = fr.ip;
            let base = fr.base;
""",
),
(
"            let byte = self.module.functions[func].chunk.code[ip];\n            let line = self.line();\n            let Some(op) = Op::from_u8(byte) else {\n",
"""            // 步骤3：取出操作码字节，并查源码行号（报错用）
            let byte = self.module.functions[func].chunk.code[ip];
            let line = self.line();
            let Some(op) = Op::from_u8(byte) else {
""",
),
(
"            match op {\n                Op::Const => {\n                    let i = self.u16(ip + 1)? as usize;\n",
"""            // 步骤4：按操作码分支执行（下面每个分支就是「这一步该做什么」）
            match op {
                // Const <u16>：从常量池取出第 i 项，压入栈
                Op::Const => {
                    let i = self.u16(ip + 1)? as usize;
""",
),
(
"                Op::True => {\n",
"""                // True：把布尔 true 压栈
                Op::True => {
""",
),
(
"                Op::False => {\n",
"""                // False：把布尔 false 压栈
                Op::False => {
""",
),
(
"                Op::Pop => {\n                    self.pop(line)?;\n",
"""                // Pop：丢弃栈顶（表达式算完后清掉临时值）
                Op::Pop => {
                    self.pop(line)?;
""",
),
(
"                Op::GetLocal => {\n                    let s = self.u16(ip + 1)? as usize;\n",
"""                // GetLocal <槽号>：复制局部变量到栈顶（槽里原值还在）
                Op::GetLocal => {
                    let s = self.u16(ip + 1)? as usize;
""",
),
(
"                Op::SetLocal => {\n                    let s = self.u16(ip + 1)? as usize;\n",
"""                // SetLocal <槽号>：用栈顶的值覆盖局部槽（不弹栈，常与 Pop 连用）
                Op::SetLocal => {
                    let s = self.u16(ip + 1)? as usize;
""",
),
(
"                Op::SetLocalField => {\n",
"""                // SetLocalField <槽号> <字段名常量>：弹出值，写入栈槽里结构体的该字段
                Op::SetLocalField => {
""",
),
(
"                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {\n",
"""                // 算术：先弹右操作数 b，再弹左操作数 a，算完把结果压回
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
""",
),
(
"                Op::Neg => {\n",
"""                // Neg：栈顶取负（-x）
                Op::Neg => {
""",
),
(
"                Op::Not => {\n",
"""                // Not：栈顶逻辑非（!flag）
                Op::Not => {
""",
),
(
"                Op::Eq | Op::Ne => {\n",
"""                // == / !=：比较两个栈顶值，结果为 bool
                Op::Eq | Op::Ne => {
""",
),
(
"                Op::Lt | Op::Le | Op::Gt | Op::Ge => {\n",
"""                // < <= > >=：两个数比较，结果为 bool
                Op::Lt | Op::Le | Op::Gt | Op::Ge => {
""",
),
(
"                Op::Jump => {\n",
"""                // Jump：无条件向前跳转（偏移量在操作数里）
                Op::Jump => {
""",
),
(
"                Op::JumpIfFalse | Op::JumpIfTrue => {\n",
"""                // 条件跳转：只「看」栈顶 bool，不弹出；代码生成负责随后 Pop
                Op::JumpIfFalse | Op::JumpIfTrue => {
""",
),
(
"                Op::Loop => {\n",
"""                // Loop：向后跳回循环头（while / for 脱糖后的回边）
                Op::Loop => {
""",
),
(
"                Op::Call => {\n",
"""                // Call <函数下标>：进入被调函数（新压一帧）
                Op::Call => {
""",
),
(
"                Op::CallMethod => {\n",
"""                // CallMethod <方法名> <argc>：按接收者运行时类型名查找 Type.method 并调用
                Op::CallMethod => {
""",
),
(
"                Op::Return => {\n",
"""                // Return：弹出当前帧
                //   void：截断栈到 base（清掉参数/局部）
                //   非void：先拿返回值，截断后再压回去（供调用方使用）
                Op::Return => {
""",
),
(
"                Op::NewArray => {\n",
"""                // NewArray <n>：弹出 n 个元素，包成数组句柄压栈
                Op::NewArray => {
""",
),
(
"                Op::GetIndex => {\n",
"""                // GetIndex：栈 [数组, 下标] → 弹出后把元素压栈（越界报错）
                Op::GetIndex => {
""",
),
(
"                Op::SetIndex => {\n",
"""                // SetIndex：栈 [数组, 下标, 新值] → 写回数组（引用语义，就地改）
                Op::SetIndex => {
""",
),
(
"                Op::NewStruct => {\n",
"""                // NewStruct <类型下标> <字段数>：按声明顺序组装字段，压入结构体值
                Op::NewStruct => {
""",
),
(
"                Op::GetField => {\n",
"""                // GetField <字段名>：弹出结构体，把字段值拷贝压栈
                Op::GetField => {
""",
),
(
"                Op::SetField => {\n",
"""                // SetField <字段名>：栈 [结构体, 新值] → 改字段后的结构体压回（值语义）
                Op::SetField => {
""",
),
(
"                Op::Len => {\n",
"""                // Len：数组长度或字符串字符数
                Op::Len => {
""",
),
(
"                Op::Print => {\n",
"""                // Print：弹出栈顶，把显示文本记入 out（CLI 再打印）
                Op::Print => {
""",
),
(
"                Op::Push => {\n",
"""                // push(arr, v)：弹出值和数组，追加到数组末尾
                Op::Push => {
""",
),
(
"                Op::ArrayPop => {\n",
"""                // pop(arr)：弹出数组，取出末元素压栈；空数组报错
                Op::ArrayPop => {
""",
),
(
"                Op::Input => {\n",
"""                // input()：从标准输入读一行，压入 string（去掉换行）
                Op::Input => {
""",
),
(
"                Op::StrAt => {\n",
"""                // str_at(s, i)：取第 i 个字符；越界报错
                Op::StrAt => {
""",
),
(
"                Op::StrSub => {\n",
"""                // str_sub(s, start, n)：子串；越界报错
                Op::StrSub => {
""",
),
(
"                Op::ToString => {\n",
"""                // to_string(v)：把任意可打印值转成字符串显示
                Op::ToString => {
""",
),
]
for old, new in replacements:
    if old not in t:
        print("MISS exec", old.splitlines()[0][:40])
    else:
        t = t.replace(old, new, 1)
exec_path.write_text(t, encoding="utf-8")
print("wrote exec.rs comments")

# ---- fold.rs ----
patch("src/sema/check/fold.rs", [
(
"    pub(crate) fn fold(&self, e: &Expr) -> Result<(Type, Value), CheckError> {\n        match e {\n",
"""    /// 把表达式在**编译期**算成具体值（仅当整棵子树都是常量时）。
    ///
    /// 步骤：
    /// 1. 字面量 → 直接得到 Value
    /// 2. 变量 → 查已折叠的 const 表；查不到说明不是常量
    /// 3. 一元/二元 → 先递归折叠子表达式，再算运算
    /// 4. 其它（函数调用、变量等）→ 报错「不是编译期常量」
    ///
    /// 返回 (类型, 值)，供检查器核对标注类型、供 codegen 发 Const 指令。
    pub(crate) fn fold(&self, e: &Expr) -> Result<(Type, Value), CheckError> {
        match e {
""",
),
(
"            Expr::Binary { op, lhs, rhs, span } => {\n                let (lt, lv) = self.fold(lhs)?;\n",
"""            // 二元：先折左，再折右，最后 fold_bin 做运算
            Expr::Binary { op, lhs, rhs, span } => {
                let (lt, lv) = self.fold(lhs)?;
""",
),
])

# ---- parser expr.rs ----
patch("src/syntax/parser/expr.rs", [
(
"//! 表达式解析（优先级见模块注释）。\nuse super::*;\n\nimpl Parser {\n    pub fn expr(&mut self) -> Result<Expr, ParseError> {\n        self.range_expr()\n    }\n",
"""//! 表达式解析（优先级见模块注释）。
//!
//! **每一步在做什么**：像算术课从「优先级最低」的运算开始拆括号。
//! 调用链：expr → range → or → and → equality → cmp → term → factor
//!         → unary → postfix → primary
//! 每一层：先解析更「紧」的左边，再看当前层运算符是否出现，出现则继续解析右边并组装 Binary。

use super::*;

impl Parser {
    /// 表达式入口：允许 `a..b`（范围；主要给 for 使用）。
    pub fn expr(&mut self) -> Result<Expr, ParseError> {
        self.range_expr()
    }
""",
),
(
"    pub(crate) fn or(&mut self) -> Result<Expr, ParseError> {\n        let mut lhs = self.and()?;\n",
"""    /// `||` 层：左边解析完后，连续吃 `|| 右边`。
    pub(crate) fn or(&mut self) -> Result<Expr, ParseError> {
        // 步骤1：先解析优先级更高的 &&
        let mut lhs = self.and()?;
""",
),
(
"    pub(crate) fn and(&mut self) -> Result<Expr, ParseError> {\n        let mut lhs = self.equality()?;\n",
"""    /// `&&` 层。
    pub(crate) fn and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.equality()?;
""",
),
(
"    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {\n",
"""    /// 一元 `-` / `!`：若有则吃掉运算符，再解析右边（可以连续多个一元）。
    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"    pub(crate) fn postfix(&mut self) -> Result<Expr, ParseError> {\n",
"""    /// 后缀：在「原子」表达式后面反复吃 `.字段` `.方法()` `[下标]`。
    pub(crate) fn postfix(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"    pub(crate) fn primary(&mut self) -> Result<Expr, ParseError> {\n",
"""    /// 原子：数字/字符串/true/false/变量/(表达式)/[数组]/结构体字面量/函数调用。
    /// 这是优先级最高的一层，不再向更深层拆分。
    pub(crate) fn primary(&mut self) -> Result<Expr, ParseError> {
""",
),
])

# ---- parser stmt.rs ----
patch("src/syntax/parser/stmt.rs", [
(
"    pub(crate) fn block(&mut self) -> Result<Block, ParseError> {\n        let start = self.expect(TokenKind::LBrace, \"`{`\")?.span;\n",
"""    /// 语句块 `{ ... }`：
    /// 1. 吃掉 `{`
    /// 2. 循环解析语句，直到 `}` 或文件结束
    /// 3. 吃掉 `}`，组装 Block
    pub(crate) fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace, \"`{`\")?.span;
""",
),
(
"    pub(crate) fn let_stmt(&mut self) -> Result<LetStmt, ParseError> {\n",
"""    /// `let 名字 [: 类型] = 值;`
    /// 步骤：吃 let → 名字 → 可选 `: 类型` → `=` → 表达式 → `;`
    pub(crate) fn let_stmt(&mut self) -> Result<LetStmt, ParseError> {
""",
),
(
"    pub(crate) fn if_stmt(&mut self) -> Result<IfStmt, ParseError> {\n",
"""    /// `if 条件 { 块 } [else { 块 } | else if ...]`
    pub(crate) fn if_stmt(&mut self) -> Result<IfStmt, ParseError> {
""",
),
(
"    pub(crate) fn while_stmt(&mut self) -> Result<WhileStmt, ParseError> {\n",
"""    /// `while 条件 { 块 }`
    pub(crate) fn while_stmt(&mut self) -> Result<WhileStmt, ParseError> {
""",
),
(
"    pub(crate) fn for_stmt(&mut self) -> Result<ForStmt, ParseError> {\n",
"""    /// `for 变量 in 迭代式 { 块 }`
    /// 迭代式可以是数组或 `a..b` 范围（在 expr 里已能解析 Range）。
    pub(crate) fn for_stmt(&mut self) -> Result<ForStmt, ParseError> {
""",
),
])

# ---- compile expr / stmt ----
patch("src/codegen/compile/expr.rs", [
(
"//! 表达式与调用编译。\nuse super::*;\n\nimpl Compiler {\n    pub(crate) fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {\n        match e {\n",
"""//! 表达式与调用编译。
//!
//! **每一步在做什么**：把 AST 节点变成「压栈 / 运算 / 弹栈」的指令序列。
//! 例如 `1 + 2 * 3` 会变成：Const 1, Const 2, Const 3, Mul, Add（栈机后缀式）。

use super::*;

impl Compiler {
    /// 编译一个表达式：保证执行完后，结果在**栈顶**。
    pub(crate) fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {
        match e {
""",
),
(
"            Expr::Var { name } => {\n",
"""            // 变量：若已折叠成 const 则直接发字面量；否则 GetLocal 槽号
            Expr::Var { name } => {
""",
),
(
"            Expr::Call { callee, args, span } => self.call(&callee.name, args, *span),\n",
"""            // 函数调用 / 内建，见 call()
            Expr::Call { callee, args, span } => self.call(&callee.name, args, *span),
""",
),
(
"    pub(crate) fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<(), CompileError> {\n        let line = span.line;\n        match name {\n",
"""    /// 编译函数/内建调用。
    /// 步骤：识别内建 → 否则查函数表 → 依次编译实参（压栈）→ 发 Call
    pub(crate) fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let line = span.line;
        match name {
""",
),
])

patch("src/codegen/compile/stmt.rs", [
(
"    pub(crate) fn for_stmt(&mut self, f: &ForStmt) -> Result<(), CompileError> {\n        let line = f.span.line;\n        self.f().begin();\n",
"""    /// 编译 `for`。
    ///
    /// **范围 for `a..b`**：
    /// 1. 算 a → 存入循环变量槽；算 b → 存入临时槽
    /// 2. 循环头：`i < end`，假则跳出
    /// 3. 执行循环体
    /// 4. continue 跳到这里：`i = i + 1`，再 Loop 回循环头
    /// 5. break / 正常结束：清理局部并继续后续代码
    ///
    /// **数组 for-in**：思路相同，多了「每次从数组取 arr[i] 绑定循环变量」。
    pub(crate) fn for_stmt(&mut self, f: &ForStmt) -> Result<(), CompileError> {
        let line = f.span.line;
        self.f().begin();
""",
),
(
"    pub(crate) fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {\n        let line = a.span.line;\n",
"""    /// 编译赋值。
    /// 1) `x = e`：算 e → SetLocal → Pop
    /// 2) `a[i] = e`：压 a、i、e → SetIndex
    /// 3) `p.f = e`：值语义，取出/写回见下方分支
    pub(crate) fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
        let line = a.span.line;
""",
),
])

# ---- CLI ----
patch("src/bin/laughter.rs", [
(
"fn main() -> ExitCode {\n    let args: Vec<String> = env::args().skip(1).collect();\n",
"""/// CLI 主流程（每一步）：
/// 1. 读命令行参数
/// 2. 按子命令分派
/// 3. 调用库 API（run_file / compile_file / pack_program / ...）
/// 4. 成功则打印结果；失败打印错误并返回非 0 退出码
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
""",
),
(
"        \"run\" => {\n",
"""        // run：源码 → 检查+编译+执行，打印 print 的每一行
        \"run\" => {
""",
),
(
"        \"compile\" => {\n",
"""        // compile：源码 → 写入 .lgb 字节码文件（默认同名，可用 -o）
        \"compile\" => {
""",
),
(
"        \"pack\" => {\n",
"""        // pack：编译入口并打成 ZIP 包 .lgpack（清单+字节码+资源）
        \"pack\" => {
""",
),
(
"        \"exec\" => {\n",
"""        // exec：根据扩展名选择 .lg / .lgb / .lgpack 的执行路径
        \"exec\" => {
""",
),
(
"        \"disasm\" => {\n",
"""        // disasm：反汇编 .lg / .lgb / .lgpack，打印人类可读指令
        \"disasm\" => {
""",
),
(
"        \"list\" => {\n",
"""        // list：列出 .lgpack 内条目（类似 jar tf）
        \"list\" => {
""",
),
])

# ---- module_loader compile_path ----
patch("src/module_loader.rs", [
(
"fn parse_src(file: &str, src: &str) -> Result<Program, String> {\n",
"""/// 单文件：词法 + 语法 → AST（错误带 `file:line:col`）。
fn parse_src(file: &str, src: &str) -> Result<Program, String> {
""",
),
(
"pub fn compile_path(path: &Path) -> Result<Module, String> {\n",
"""/// 按路径编译（支持 import）。步骤：
/// 1. load：递归读入 import，合并声明
/// 2. Checker：类型检查 + const 折叠
/// 3. Compiler：AST → 字节码 Module
pub fn compile_path(path: &Path) -> Result<Module, String> {
""",
),
(
"pub fn run_path(path: &Path) -> Result<Vec<String>, String> {\n",
"""/// 按路径编译并执行，返回 print 的各行。
pub fn run_path(path: &Path) -> Result<Vec<String>, String> {
""",
),
])

# ---- lexer / check stmt / parser decl ----
patch("src/syntax/lexer.rs", [
(
"    pub(crate) fn tokenize(mut self) -> Result<Vec<Token>, LexError> {\n        let mut out = Vec::new();\n        loop {\n",
"""    /// 把整个源文件切成 Token。
    /// 步骤：循环「取下一个 Token」→ 收集 → 直到 Eof。
    pub(crate) fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
""",
),
])

patch("src/sema/check/stmt.rs", [
(
"    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {\n        match s {\n",
"""    /// 检查一条语句。
    /// 步骤：按语句种类分支 → 求相关表达式类型 → 对照语言规则，不符则报错。
    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
        match s {
""",
),
])

patch("src/syntax/parser/decl.rs", [
(
"    pub(crate) fn struct_decl(&mut self) -> Result<StructDecl, ParseError> {\n",
"""    /// `struct 名 { 字段: 类型, ... }`
    /// 步骤：吃 struct → 名字 → `{` → 循环「字段名 : 类型」→ `}`
    pub(crate) fn struct_decl(&mut self) -> Result<StructDecl, ParseError> {
""",
),
(
"    pub(crate) fn fun_decl(&mut self) -> Result<FunDecl, ParseError> {\n",
"""    /// 函数/方法声明。
    /// 步骤：fun → 名字（若是 `Type.method` 则记录 on_type）→ 参数表 → `->` 返回类型 → 函数体块
    pub(crate) fn fun_decl(&mut self) -> Result<FunDecl, ParseError> {
""",
),
])

patch("src/sema/check/expr.rs", [
(
"    pub(crate) fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {\n        match e {\n",
"""    /// 求表达式的类型（语义阶段）。
    /// 步骤：按节点种类处理 → 字面量直接给类型；变量查作用域/const；
    ///      二元运算调用 bin_result；调用检查签名与实参。
    pub(crate) fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {
        match e {
""",
),
])

print("all patches done")
