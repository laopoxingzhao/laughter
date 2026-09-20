#!/usr/bin/env python3
"""Insert Chinese step comments inside function bodies."""
from pathlib import Path

def patch_file(path, pairs):
    p = Path(path)
    if not p.exists():
        print("skip missing", path)
        return
    t = p.read_text(encoding="utf-8")
    n = 0
    for old, new in pairs:
        if not old:
            continue
        if old not in t:
            print(f"  MISS {path}: {old.splitlines()[0][:55]}")
            continue
        if t.count(old) != 1:
            print(f"  MULTI {path}: {old.splitlines()[0][:40]} x{t.count(old)}")
            continue
        t = t.replace(old, new, 1)
        n += 1
    p.write_text(t, encoding="utf-8")
    print(f"ok {path} +{n}")

# ===== compile/mod.rs =====
patch_file("src/codegen/compile/mod.rs", [
(
"""    pub fn compile(
        program: &Program,
        consts: HashMap<String, Value>,
    ) -> Result<Module, CompileError> {
""",
"""    pub fn compile(
        program: &Program,
        consts: HashMap<String, Value>,
    ) -> Result<Module, CompileError> {
        // 步骤1：扫描所有 struct 声明，建立「类型名 → 字段名列表」
        //         NewStruct / 字段布局会用到这张表
""",
),
(
"        let mut fidx = HashMap::new();\n        let mut voids = HashSet::new();\n",
"""        // 步骤2：给每个函数编号。方法的键名是 `Type.method`
        //         voids 记录无返回值的函数（VM 在 Return 时不压返回值）
        let mut fidx = HashMap::new();
        let mut voids = HashSet::new();
""",
),
(
"        let mut fns = vec![];\n        for f in &decls {\n",
"""        // 步骤3：为每个函数创建编译上下文 FnC
        //         参数名按顺序登记为槽 0..arity-1
        let mut fns = vec![];
        for f in &decls {
""",
),
(
"        for (i, f) in decls.iter().enumerate() {\n            cx.current = i;\n            for stmt in &f.body.stmts {\n",
"""        // 步骤4：编译每个函数体；函数末尾再补一条 Return（兜底）
        for (i, f) in decls.iter().enumerate() {
            cx.current = i;
            for stmt in &f.body.stmts {
""",
),
(
"        cx.cur = tl;\n        for stmt in program.top_level_stmts() {\n",
"""        // 步骤5：顶层语句编译进合成函数 $toplevel
        //         有 main 时 run 不会执行它；无 main 时作为入口
        cx.cur = tl;
        for stmt in program.top_level_stmts() {
""",
),
(
"        let voids = cx.voids.clone();\n        let struct_types = cx.stypes.clone();\n",
"""        // 步骤6：把 FnC 收成 VM 使用的 Function 列表，组装 Module
        let voids = cx.voids.clone();
        let struct_types = cx.stypes.clone();
""",
),
(
"""    fn chunk(&mut self) -> &mut Chunk {
        &mut self.fns[self.cur].chunk
    }
""",
"""    /// 当前正在编译的函数的字节码缓冲。
    fn chunk(&mut self) -> &mut Chunk {
        &mut self.fns[self.cur].chunk
    }
""",
),
(
"""    fn void_call(&self, n: &str) -> bool {
        self.voids.contains(n)
    }
""",
"""    /// 判断某调用目标是否不产生返回值（表达式语句因此不必 Pop）。
    fn void_call(&self, n: &str) -> bool {
        self.voids.contains(n)
    }
""",
),
])

# ===== vm/source.rs =====
patch_file("src/runtime/vm/source.rs", [
(
"""fn parse(src: &str, file: &str) -> Result<crate::syntax::ast::Program, String> {
    use crate::syntax::lexer::Lexer;
    use crate::syntax::parser::Parser;
    let toks = Lexer::new(src).tokenize().map_err(|e| {
""",
"""/// 解析步骤：
/// 1. Lexer：源码字符串 → Token 流
/// 2. Parser：Token 流 → AST（Program）
/// 任一步失败都带上 `文件:行:列: 错误: ...`
fn parse(src: &str, file: &str) -> Result<crate::syntax::ast::Program, String> {
    use crate::syntax::lexer::Lexer;
    use crate::syntax::parser::Parser;
    // 步骤1：切词
    let toks = Lexer::new(src).tokenize().map_err(|e| {
""",
),
(
"    Parser::new(toks).parse_program().map_err(|e| {",
"    // 步骤2：组装语法树\n    Parser::new(toks).parse_program().map_err(|e| {",
),
(
"""pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::codegen::compile::Compiler;
    use crate::sema::check::Checker;
""",
"""pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::codegen::compile::Compiler;
    use crate::sema::check::Checker;
    // 步骤1：parse
""",
),
(
"    if program.imports().next().is_some() {",
"    // 步骤2：单文件 API 不支持 import，尽早报错\n    if program.imports().next().is_some() {",
),
(
"    let consts = Checker::new(&program).check().map_err(|e| {",
"    // 步骤3：类型检查 + const 折叠\n    let consts = Checker::new(&program).check().map_err(|e| {",
),
(
"    Compiler::compile(&program, consts)",
"    // 步骤4：生成字节码 Module\n    Compiler::compile(&program, consts)",
),
(
"""pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    let module = compile_source_file(file, src)?;
""",
"""pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    // 步骤1：编译
    let module = compile_source_file(file, src)?;
""",
),
(
"    let mut vm = Vm::new(&module);\n    vm.run().map_err(|e| {",
"    // 步骤2：在栈机上执行，收集 print 行\n    let mut vm = Vm::new(&module);\n    vm.run().map_err(|e| {",
),
])

# ===== check/expr.rs =====
patch_file("src/sema/check/expr.rs", [
(
"""    pub(crate) fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {
        match e {
""",
"""    pub(crate) fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {
        // 按 AST 节点种类分支：每种节点如何得到类型
        match e {
""",
),
(
"            Expr::Var { name } => {\n",
"""            // 变量：先查 const 表，再查作用域栈（内层优先）
            Expr::Var { name } => {
""",
),
(
"            Expr::Unary { op, expr, span } => {\n",
"""            // 一元：先递归求操作数类型，再按 - / ! 校验
            Expr::Unary { op, expr, span } => {
""",
),
(
"            Expr::Binary { op, lhs, rhs, span } => {\n",
"""            // 二元：左右都要有类型，再交给 bin_result 判断能否运算
            Expr::Binary { op, lhs, rhs, span } => {
""",
),
(
"            Expr::Index { base, index, span } => {\n",
"""            // 下标：基必须是数组，下标必须是 int，结果是元素类型
            Expr::Index { base, index, span } => {
""",
),
(
"            Expr::Field { base, name, span } => {\n",
"""            // 字段访问：基必须是 struct，再去字段表查类型
            Expr::Field { base, name, span } => {
""",
),
(
"            Expr::Array { elems, span } => {\n",
"""            // 数组字面量：非空；元素类型必须全部相同
            Expr::Array { elems, span } => {
""",
),
(
"            Expr::StructLit { name, fields, span } => {\n",
"""            // 结构体字面量：字段数齐全，每个字段类型与声明一致
            Expr::StructLit { name, fields, span } => {
""",
),
(
"""    pub(crate) fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, CheckError> {
        match name {
""",
"""    pub(crate) fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, CheckError> {
        // 先匹配内建函数；都不是再查用户函数表
        match name {
""",
),
])

# ===== check/stmt.rs =====
patch_file("src/sema/check/stmt.rs", [
(
"""    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
        match s {
""",
"""    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
        // 按语句种类检查；每条路径违反语言规则就返回 Err
        match s {
""",
),
(
"            Stmt::Let(l) => {\n",
"""            // let：先求初始值类型；[] 需要标注；再写入当前作用域
            Stmt::Let(l) => {
""",
),
(
"            Stmt::Assign(a) => {\n",
"""            // 赋值：不能改 const；求出目标类型后与右值比较
            Stmt::Assign(a) => {
""",
),
(
"            Stmt::For(f) => {\n",
"""            // for：区分「数组」与「范围 a..b」；循环变量进新作用域
            Stmt::For(f) => {
""",
),
(
"            Stmt::Return(r) => {\n",
"""            // return：顶层禁止；类型必须与当前函数返回类型一致
            Stmt::Return(r) => {
""",
),
])

# ===== check/mod.rs =====
patch_file("src/sema/check/mod.rs", [
(
"""    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {
""",
"""    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {
        // 步骤1：登记结构体（供字段/类型标注解析）
""",
),
(
"        for f in self.program.functions() {\n            self.declare_fun(f)?;\n        }\n",
"""        // 步骤2：登记函数与方法签名
        for f in self.program.functions() {
            self.declare_fun(f)?;
        }
""",
),
(
"        for c in self.program.consts() {\n            self.declare_const(c)?;\n        }\n",
"""        // 步骤3：登记并折叠 const（此时已知结构体/函数名，可查重）
        for c in self.program.consts() {
            self.declare_const(c)?;
        }
""",
),
(
"        for f in self.program.functions() {\n            self.check_fun(f)?;\n        }\n",
"""        // 步骤4：逐个检查函数体
        for f in self.program.functions() {
            self.check_fun(f)?;
        }
""",
),
(
"        for st in self.program.top_level_stmts() {\n            self.check_stmt(st)?;\n        }\n",
"""        // 步骤5：检查顶层语句（有 main 时运行期不执行，但仍须类型合法）
        for st in self.program.top_level_stmts() {
            self.check_stmt(st)?;
        }
""",
),
(
"        Ok(self\n            .consts\n            .into_iter()",
"        // 步骤6：返回「名字 → 折叠后的值」供 codegen 直接 Const\n        Ok(self\n            .consts\n            .into_iter()",
),
(
"""    fn declare_struct(&mut self, s: &StructDecl) -> Result<(), CheckError> {
""",
"""    /// 登记结构体：查重 → 逐字段解析类型 → 查重字段名 → 写入 structs 表。
    fn declare_struct(&mut self, s: &StructDecl) -> Result<(), CheckError> {
""",
),
(
"""    fn declare_const(&mut self, c: &ConstDecl) -> Result<(), CheckError> {
""",
"""    /// 登记 const：查重 → 解析标注类型 → 折叠右侧表达式 → 核对类型一致。
    fn declare_const(&mut self, c: &ConstDecl) -> Result<(), CheckError> {
""",
),
(
"""    fn declare_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
""",
"""    /// 登记函数/方法：解析参数与返回类型；方法校验接收者类型。
    fn declare_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
""",
),
(
"""    fn check_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
""",
"""    /// 检查函数体：新作用域 + 参数槽 → 检查语句 → 非 void 须所有路径 return。
    fn check_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
""",
),
(
"""    fn lookup(&self, n: &str) -> Option<Type> {
""",
"""    /// 从内向外查变量类型（作用域栈顶优先）。
    fn lookup(&self, n: &str) -> Option<Type> {
""",
),
])

# ===== lexer key fns =====
patch_file("src/syntax/lexer.rs", [
(
"""    fn peek(&self) -> Option<u8> {
""",
"""    /// 看当前字节，不前进。
    fn peek(&self) -> Option<u8> {
""",
),
(
"""    fn bump(&mut self) -> Option<u8> {
""",
"""    /// 读走当前字节并前进；若为换行则行号+1、列归1。
    fn bump(&mut self) -> Option<u8> {
""",
),
(
"""    fn number(&mut self, span: Span) -> Result<Token, LexError> {
""",
"""    /// 数字：先吃连续数字，若出现「小数点+后继数字」则再吃小数部分。
    fn number(&mut self, span: Span) -> Result<Token, LexError> {
""",
),
(
"""    fn string(&mut self, span: Span) -> Result<Token, LexError> {
""",
"""    /// 字符串：吃掉开头引号，直到结束引号；处理 \\n 等转义。
    fn string(&mut self, span: Span) -> Result<Token, LexError> {
""",
),
])

# ===== parser/mod.rs tools =====
patch_file("src/syntax/parser/mod.rs", [
(
"""    fn peek(&self) -> &Token {
""",
"""    /// 看当前 Token，不消耗。
    fn peek(&self) -> &Token {
""",
),
(
"""    fn advance(&mut self) -> Token {
""",
"""    /// 消耗并返回当前 Token（已到 Eof 则停住不越界）。
    fn advance(&mut self) -> Token {
""",
),
(
"""    fn expect(&mut self, k: TokenKind, what: &str) -> Result<Token, ParseError> {
""",
"""    /// 期望某种 Token：是则消耗，否则报错（what 用于中文错误说明）。
    fn expect(&mut self, k: TokenKind, what: &str) -> Result<Token, ParseError> {
""",
),
(
"""    fn check(&self, k: &TokenKind) -> bool {
""",
"""    /// 当前 Token 是否为指定种类（只比较判别式，不比较 Ident 内容）。
    fn check(&self, k: &TokenKind) -> bool {
""",
),
])

# ===== parser/expr.rs =====
patch_file("src/syntax/parser/expr.rs", [
(
"""    pub(crate) fn equality(&mut self) -> Result<Expr, ParseError> {
""",
"""    /// 相等层 `==` `!=`：循环吃运算符，左结合组装 Binary。
    pub(crate) fn equality(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"""    pub(crate) fn cmp(&mut self) -> Result<Expr, ParseError> {
""",
"""    /// 比较层 `< <= > >=`。
    pub(crate) fn cmp(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"""    pub(crate) fn term(&mut self) -> Result<Expr, ParseError> {
""",
"""    /// 加减层 `+ -`。
    pub(crate) fn term(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"""    pub(crate) fn factor(&mut self) -> Result<Expr, ParseError> {
""",
"""    /// 乘除余层 `* / %`。
    pub(crate) fn factor(&mut self) -> Result<Expr, ParseError> {
""",
),
(
"""    pub(crate) fn arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
""",
"""    /// 实参列表：空参或 `e, e, ...`。
    pub(crate) fn arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
""",
),
])

# ===== parser/decl.rs =====
patch_file("src/syntax/parser/decl.rs", [
(
"""    pub(crate) fn ty(&mut self) -> Result<TypeExpr, ParseError> {
""",
"""    /// 解析类型：标量关键字 / 结构体名，后面可跟 `[]` 表示数组。
    pub(crate) fn ty(&mut self) -> Result<TypeExpr, ParseError> {
""",
),
(
"""    pub(crate) fn import_item(&mut self) -> Result<ImportItem, ParseError> {
""",
"""    /// 解析 import：字符串路径 + 可选 `as 别名` + 分号；拒绝 `..`。
    pub(crate) fn import_item(&mut self) -> Result<ImportItem, ParseError> {
""",
),
(
"""    pub(crate) fn const_decl(&mut self) -> Result<ConstDecl, ParseError> {
""",
"""    /// 解析 const：名字 : 类型 = 表达式 ;
    pub(crate) fn const_decl(&mut self) -> Result<ConstDecl, ParseError> {
""",
),
])

# ===== parser/stmt.rs remaining =====
patch_file("src/syntax/parser/stmt.rs", [
(
"""    pub(crate) fn stmt(&mut self) -> Result<Stmt, ParseError> {
""",
"""    /// 语句分派：关键字开头的专用语句优先，否则尝试赋值/表达式语句。
    pub(crate) fn stmt(&mut self) -> Result<Stmt, ParseError> {
""",
),
])

# ===== token Display / Span =====
patch_file("src/syntax/token.rs", [
(
"""    pub fn new(line: u32, col: u32) -> Self {
""",
"""    /// 构造源位置（行、列均从 1 开始）。
    pub fn new(line: u32, col: u32) -> Self {
""",
),
])

# ===== chunk.rs remaining =====
patch_file("src/codegen/chunk.rs", [
(
"""    pub fn emit_u16(&mut self, n: u16, line: u32) {
""",
"""    /// 写入 2 字节小端操作数；行号表补两格与字节对齐。
    pub fn emit_u16(&mut self, n: u16, line: u32) {
""",
),
(
"""    pub fn add_const(&mut self, v: Value) -> Result<u16, String> {
""",
"""    /// 常量入池：若已存在相同值则复用下标，否则追加。
    pub fn add_const(&mut self, v: Value) -> Result<u16, String> {
""",
),
])

# ===== module_loader =====
patch_file("src/module_loader.rs", [
(
"""fn parse_src(file: &str, src: &str) -> Result<Program, String> {
""",
"""/// 文件路径 + 源码文本 → AST。
fn parse_src(file: &str, src: &str) -> Result<Program, String> {
""",
),
(
"""fn resolve(base_file: &Path, rel: &str) -> Result<PathBuf, String> {
""",
"""/// 把 import 相对路径拼到「当前文件所在目录」；禁止 `..`。
fn resolve(base_file: &Path, rel: &str) -> Result<PathBuf, String> {
""",
),
])

# ===== lgb encode/decode =====
patch_file("src/codegen/lgb.rs", [
(
"""    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
""",
"""    // 步骤1：文件头
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
""",
),
(
"    // struct types\n",
"    // 步骤2：结构体布局表\n    // struct types\n",
),
(
"    // functions\n",
"    // 步骤3：函数表（常量池 + 指令 + 行号）\n    // functions\n",
),
])

# ===== lgpack =====
patch_file("src/codegen/lgpack.rs", [
(
"    let module = compile_file(&main_lg.display().to_string())",
"    // 步骤1：编译入口源码（import 已在 loader 合并）\n    let module = compile_file(&main_lg.display().to_string())",
),
(
"    let lgb = encode_module(&module).map_err(|e| err(e.message))?;",
"    // 步骤2：编码为 .lgb 字节\n    let lgb = encode_module(&module).map_err(|e| err(e.message))?;",
),
(
"    let file = std::fs::File::create(output)",
"    // 步骤3：创建 ZIP，写入清单与 app.lgb\n    let file = std::fs::File::create(output)",
),
(
"    let (meta, bytes) = read_package(path).map_err(|e| e.to_string())?;",
"    // 步骤：读包 → 校验版本 → 解码 → 执行\n    let (meta, bytes) = read_package(path).map_err(|e| e.to_string())?;",
),
])

print("all done")
