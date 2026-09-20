#!/usr/bin/env python3
"""Add step-by-step Chinese comments to remaining files + tests."""
from pathlib import Path

def patch(path, pairs):
    p = Path(path)
    t = p.read_text(encoding="utf-8")
    ok = 0
    for old, new in pairs:
        if old not in t:
            print(f"  MISS {path}: {old[:60]!r}")
            continue
        if t.count(old) != 1:
            print(f"  MULTI {path}: {old[:50]!r}")
            continue
        t = t.replace(old, new, 1)
        ok += 1
    p.write_text(t, encoding="utf-8")
    print(f"patched {path} ({ok}/{len(pairs)})")

# ---- lgb.rs ----
patch("src/codegen/lgb.rs", [
(
"pub fn encode_module(module: &Module) -> Result<Vec<u8>, BytecodeError> {\n    let mut out = Vec::new();\n",
"""/// 把内存里的 Module 编码成 .lgb 字节。
/// 步骤：
/// 1. 写文件头：魔数 LGB1、版本、main 下标
/// 2. 写结构体布局表
/// 3. 对每个函数：名字、arity、void、常量池、指令、行号表
pub fn encode_module(module: &Module) -> Result<Vec<u8>, BytecodeError> {
    let mut out = Vec::new();
""",
),
(
"pub fn decode_module(bytes: &[u8]) -> Result<Module, BytecodeError> {\n    let mut r = Reader { buf: bytes, pos: 0 };\n",
"""/// 从 .lgb 字节解码回 Module。
/// 步骤：校验魔数与版本 → 读入口信息 → 读结构体表 → 读函数表 → 校验下标范围
pub fn decode_module(bytes: &[u8]) -> Result<Module, BytecodeError> {
    let mut r = Reader { buf: bytes, pos: 0 };
""",
),
(
"pub fn exec_module(module: &Module) -> Result<Vec<String>, String> {\n",
"""/// 在内存 Module 上跑 VM（.lgb 与 .lgpack 共用）。
pub fn exec_module(module: &Module) -> Result<Vec<String>, String> {
""",
),
])

# ---- lgpack.rs ----
patch("src/codegen/lgpack.rs", [
(
"pub fn pack_program(\n    main_lg: &Path,\n",
"""/// 打包 .lgpack（类 JAR）。步骤：
/// 1. 编译入口 .lg（import 已合并）→ 得到 Module
/// 2. encode 成 .lgb 字节
/// 3. 写 ZIP：清单 META-INF/MANIFEST.MF + 入口 app.lgb + resources/*
pub fn pack_program(
    main_lg: &Path,
""",
),
(
"pub fn read_package(path: &Path) -> Result<(PackMeta, Vec<u8>), PackError> {\n",
"""/// 打开 ZIP 包：读清单 → 找到 Main-Bytecode 指向的条目 → 取出 .lgb 字节
pub fn read_package(path: &Path) -> Result<(PackMeta, Vec<u8>), PackError> {
""",
),
(
"pub fn exec_package(path: &Path) -> Result<Vec<String>, String> {\n",
"""/// 执行 .lgpack：读包 → 校验包版本 → 解码字节码 → VM 执行
pub fn exec_package(path: &Path) -> Result<Vec<String>, String> {
""",
),
(
"pub fn parse_manifest(text: &str) -> PackMeta {\n",
"""/// 解析清单 Key: value 行；缺省 Main-Bytecode 为 app.lgb
pub fn parse_manifest(text: &str) -> PackMeta {
""",
),
])

# ---- chunk.rs emit ----
patch("src/codegen/chunk.rs", [
(
"    pub fn emit(&mut self, op: Op, line: u32) {\n",
"""    /// 写入一条无操作数指令：code 追加操作码字节，lines 记录源码行。
    pub fn emit(&mut self, op: Op, line: u32) {
""",
),
(
"    pub fn emit_const(&mut self, v: Value, line: u32) -> Result<(), String> {\n",
"""    /// 压入常量：先放进常量池（相同值复用下标），再发 Const <下标>。
    pub fn emit_const(&mut self, v: Value, line: u32) -> Result<(), String> {
""",
),
(
"    pub fn emit_jump(&mut self, op: Op, line: u32) -> usize {\n",
"""    /// 发射带占位偏移的跳转，返回待回填的位置；目标未知时先填 0xFFFF。
    pub fn emit_jump(&mut self, op: Op, line: u32) -> usize {
""",
),
(
"    pub fn patch_to(&mut self, at: usize, dest: usize) -> Result<(), String> {\n",
"""    /// 把跳转操作数回填为：从操作数后一字节到 dest 的正向距离。
    pub fn patch_to(&mut self, at: usize, dest: usize) -> Result<(), String> {
""",
),
(
"    pub fn disassemble(&self, name: &str) -> String {\n",
"""    /// 反汇编：逐条打印地址、行号、指令名与操作数；最后列出常量表。
    pub fn disassemble(&self, name: &str) -> String {
""",
),
])

# ---- value.rs ----
patch("src/runtime/value.rs", [
(
"    pub fn get(&self, f: &str) -> Option<&Value> {\n",
"""    /// 按字段名查找字段值（不修改结构体）。
    pub fn get(&self, f: &str) -> Option<&Value> {
""",
),
(
"    pub fn set(&mut self, f: &str, v: Value) -> bool {\n",
"""    /// 按字段名写入；成功 true，字段不存在 false。
    pub fn set(&mut self, f: &str, v: Value) -> bool {
""",
),
(
"    pub fn display(&self) -> String {\n",
"""    /// 显示成 print/to_string 的文本（数组 `[a, b]`，结构体 `Name { f: v }`）。
    pub fn display(&self) -> String {
""",
),
])

# ---- vm source ----
patch("src/runtime/vm/source.rs", [
(
"pub fn compile_source(src: &str) -> Result<Module, String> {\n",
"""/// 单文件字符串 → Module（测试常用）。步骤同 compile_source_file。
pub fn compile_source(src: &str) -> Result<Module, String> {
""",
),
(
"pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {\n",
"""/// 单文件编译：parse → 若含 import 则报错 → Checker → Compiler
pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
""",
),
(
"pub fn run_source(src: &str) -> Result<Vec<String>, String> {\n",
"""/// 单文件源码直接执行（无 import）。
pub fn run_source(src: &str) -> Result<Vec<String>, String> {
""",
),
])

# ---- compile/mod compile() ----
patch("src/codegen/compile/mod.rs", [
(
"    pub fn compile(\n",
"""    /// 编译整个程序。步骤：
    /// 1. 登记结构体布局与函数索引（方法名 `Type.name`）
    /// 2. 为每个函数建 FnC（参数已占槽）
    /// 3. 依次编译函数体，末尾补 Return
    /// 4. 编译顶层语句（合成 $toplevel）
    /// 5. 汇总为 Module
    pub fn compile(
""",
),
])

# ---- check/mod check() ----
patch("src/sema/check/mod.rs", [
(
"    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {\n",
"""    /// 语义检查总入口。步骤：
    /// 1. 登记 struct / fun / const（const 会折叠）
    /// 2. 检查每个函数体
    /// 3. 检查顶层语句（有 main 时顶层不执行但仍须合法）
    /// 4. 返回折叠后的 const 表
    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {
""",
),
])

# ---- module_loader load ----
patch("src/module_loader.rs", [
(
"fn load(\n    path: &Path,\n",
"""/// 递归加载一个 .lg 文件。步骤：
/// 1. 规范化路径，检测 import 环
/// 2. 读文件并 parse 成 AST
/// 3. 对每个 import：解析相对路径 → 递归 load → 按扁平或 as ns 合并声明
/// 4. 把本文件的 fun/struct/const/顶层语句追加到 out（丢掉 import 项）
fn load(
    path: &Path,
""",
),
])

# ---- check/expr call ----
patch("src/sema/check/expr.rs", [
(
"    pub(crate) fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Type, CheckError> {\n",
"""    /// 函数/内建调用的类型检查。
    /// 步骤：先匹配内建（print/len/...）→ 否则查用户函数表 → 比对参数个数与类型
    pub(crate) fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Type, CheckError> {
""",
),
])

# ---- tests/programs.rs: annotate each test ----
t = Path("tests/programs.rs").read_text(encoding="utf-8")
t = t.replace(
    '//! 集成测试：对照 docs/LANGUAGE.md。',
    '''//! 集成测试：对照 docs/LANGUAGE.md。
//!
//! 每个 `#[test]` 上方用中文说明「测什么、期望什么」。
//! 运行：`cargo test --test programs`
''',
    1,
)
annots = {
    "fn hello_and_arith": "/// 测：打印字符串；整数运算优先级 `1+2*3==7`。\n",
    "fn fib": "/// 测：examples/fib.lg 递归 fib(10)==55。\n",
    "fn all_examples": "/// 测：全部 examples/*.lg 的标准输出与契约一致（防止回归）。\n",
    "fn const_fold": "/// 测：const 编译期折叠；disasm 中应出现字面量 (20)。\n",
    "fn value_semantics": "/// 测：结构体值语义——拷贝后互不影响。\n",
    "fn multi_level_field_assign": "/// 测：多层字段赋值 `o.inner.v = 42`。\n",
    "fn break_continue_and_range": "/// 测：范围 for + continue(跳过1) + break(在3停止) → 0,2。\n",
    "fn type_errors": "/// 测：类型错误/const 赋值/break 出循环/import 在 run_source/空结构体字面量。\n",
    "fn runtime_errors": "/// 测：除零、数组越界等运行时错误。\n",
    "fn methods_and_arrays_field": "/// 测：数组元素字段写回 + 方法调用。\n",
    "fn compile_exec_roundtrip": "/// 测：.lgb 编码→解码→执行 fib==55，魔数 LGB1。\n",
    "fn lgb_file_on_disk": "/// 测：把 .lgb 写到磁盘再加载执行（zca 输出）。\n",
    "fn lgpack_exec_roundtrip": "/// 测：pack 出 .lgpack 后 list 含清单，并 exec 多文件示例。\n",
}
for fn, doc in annots.items():
    marker = f"fn {fn}("
    if marker not in t:
        print("MISS test", fn)
        continue
    t = t.replace(marker, doc + marker, 1)
Path("tests/programs.rs").write_text(t, encoding="utf-8")
print("tests annotated")

# ---- vm/mod helpers ----
patch("src/runtime/vm/mod.rs", [
(
"    pub fn push_frame(&mut self, func: usize) -> Result<(), VmError> {\n",
"""    /// 压入调用帧。步骤：
    /// 1. 检查递归深度
    /// 2. base = 当前栈长 - 参数个数（参数已在栈上）
    /// 3. 记录 func / ip=0 / base
    pub fn push_frame(&mut self, func: usize) -> Result<(), VmError> {
""",
),
(
"    fn u16(&self, at: usize) -> Result<u16, VmError> {\n",
"""    /// 从当前函数字节码 at 处读一个小端 u16 操作数。
    fn u16(&self, at: usize) -> Result<u16, VmError> {
""",
),
(
"    fn pop(&mut self, line: u32) -> Result<Value, VmError> {\n",
    """    /// 弹出栈顶值；栈空则报 stack underflow。
    fn pop(&mut self, line: u32) -> Result<Value, VmError> {
""",
),
])

# ---- compile stmt if/while headers already done; add assign remaining ----
patch("src/codegen/compile/stmt.rs", [
(
"    pub(crate) fn if_stmt(&mut self, i: &IfStmt) -> Result<(), CompileError> {\n",
"""    /// 编译 if。步骤：
    /// 1. 编译条件（结果在栈顶）
    /// 2. JumpIfFalse 到 else/出口；true 路径先 Pop 条件
    /// 3. 编译 then 块；有 else 则 Jump 过 else，false 路径 Pop 后编译 else
    /// 4. 两路都要 Pop 条件，避免栈残留
    pub(crate) fn if_stmt(&mut self, i: &IfStmt) -> Result<(), CompileError> {
""",
),
(
"    pub(crate) fn while_stmt(&mut self, w: &WhileStmt) -> Result<(), CompileError> {\n",
"""    /// 编译 while。步骤：
    /// 1. 记录循环头地址
    /// 2. 编译条件 → 假则跳出
    /// 3. Pop 条件后编译体；continue 回到条件；break 到出口
    /// 4. 出口再 Pop 条件（假路径）
    pub(crate) fn while_stmt(&mut self, w: &WhileStmt) -> Result<(), CompileError> {
""",
),
])

print("done")
