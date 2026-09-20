#!/usr/bin/env python3
"""Translate diagnostic messages to Chinese (format prefix + body)."""
from pathlib import Path
import re

REPLACEMENTS = [
    # diagnostic prefixes
    ('"{file}:{}:{}: error: {}"', '"{file}:{}:{}: 错误: {}"'),
    # lexer
    ('message: "expected `&&`".into()', 'message: "期望 `&&`".into()'),
    ('message: "expected `||`".into()', 'message: "期望 `||`".into()'),
    ('format!("unexpected character `{}`", c as char)', 'format!("意外的字符 `{}`", c as char)'),
    ('format!("invalid float `{text}`")', 'format!("无效的浮点数 `{text}`")'),
    ('format!("invalid integer `{text}`")', 'format!("无效的整数 `{text}`")'),
    ('message: "unterminated string".into()', 'message: "字符串未闭合".into()'),
    ('message: "invalid escape".into()', 'message: "无效的转义字符".into()'),
    ('message: "invalid UTF-8 in string".into()', 'message: "字符串中含无效 UTF-8".into()'),
    # parser - many messages via format!("expected {what}... keep bilingual for now where what is Chinese
    # check/compile/vm
    ('"undefined `{}`", name.name', '"未定义的变量 `{}`", name.name'),
    ('"range only allowed in `for`"', '"范围 `a..b` 仅允许出现在 `for` 中"'),
    ('"unknown struct `{}`", name.name', '"未知结构体 `{}`", name.name'),
    ('"missing field `{fname}`"', '"缺少字段 `{fname}`"'),
    ('"undefined function `{name}`"', '"未定义函数 `{name}`"'),
    ('"`break` outside loop"', '"`break` 只能用在循环内"'),
    ('"`continue` outside loop"', '"`continue` 只能用在循环内"'),
    ('"undefined `{}`", a.name.name', '"未定义的变量 `{}`", a.name.name'),
    ('"ip out of range".into()', '"指令指针越界".into()'),
    ('format!("bad opcode {byte}")', 'format!("未知操作码 {byte}")'),
    ('"bad field constant".into()', '"字段名常量无效".into()'),
    ('format!("no field `{fname}`")', 'format!("没有字段 `{fname}`")'),
    ('format!("cannot set field on {}", other.type_name())', 'format!("无法在 {} 上设置字段", other.type_name())'),
    ('"bad local slot".into()', '"局部槽下标无效".into()'),
    ('format!("cannot negate {}", o.type_name())', 'format!("无法对 {} 取负", o.type_name())'),
    ('format!("cannot apply `!` to {}", o.type_name())', 'format!("无法对 {} 使用 `!`", o.type_name())'),
    ('format!("condition must be bool, got {}", o.type_name())', 'format!("条件必须是 bool，实际是 {}", o.type_name())'),
    ('"bad method name".into()', '"方法名常量无效".into()'),
    ('"method call underflow".into()', '"方法调用栈下溢".into()'),
    ('"receiver is not a struct".into()', '"方法接收者不是结构体".into()'),
    ('format!("undefined method `{full}`")', 'format!("未定义方法 `{full}`")'),
    ('format!("`{}` returned without value", f.name)', 'format!("函数 `{}` 缺少返回值", f.name)'),
    ('"missing array elements".into()', '"数组元素缺失".into()'),
    ('format!("cannot index {}", a.type_name())', 'format!("无法对 {} 做下标访问", a.type_name())'),
    ('format!("array index {idx} out of bounds (len {})", b.len())', 'format!("数组下标 {idx} 越界（长度 {}）", b.len())'),
    ('"missing struct fields".into()', '"结构体字段缺失".into()'),
    ('format!("cannot get field on {}", o.type_name())', 'format!("无法在 {} 上读取字段", o.type_name())'),
    ('"`push` expects array, got {}", a.type_name()', '"`push` 需要数组，实际是 {}", a.type_name()'),
    ('"`pop` expects array, got {}", a.type_name()', '"`pop` 需要数组，实际是 {}", a.type_name()'),
    ('"pop from empty array".into()', '"不能对空数组执行 pop".into()'),
    ('"str_at expects (string, int)".into()', '"`str_at` 需要 (string, int)".into()'),
    ('format!("str_at index {i} out of bounds")', 'format!("`str_at` 下标 {i} 越界")'),
    ('"str_sub expects (string, int, int)".into()', '"`str_sub` 需要 (string, int, int)".into()'),
    ('"str_sub negative index".into()', '"`str_sub` 下标不能为负".into()'),
    ('"index must be int, got {}", o.type_name()', '"下标必须是 int，实际是 {}", o.type_name()'),
    ('"division by zero".into()', '"除数不能为零".into()'),
    ('"remainder by zero".into()', '"取余运算除数不能为零".into()'),
    ('"bad int op".into()', '"非法的整数运算".into()'),
    ('"bad float op".into()', '"非法的浮点运算".into()'),
    ('format!("cannot compare {} and {}", a.type_name(), b.type_name())', 'format!("无法比较 {} 与 {}", a.type_name(), b.type_name())'),
    ('"cannot compare NaN".into()', '"无法比较 NaN".into()'),
    ('format!("cannot order {} and {}", a.type_name(), b.type_name())', 'format!("无法比较大小 {} 与 {}", a.type_name(), b.type_name())'),
    ('format!("stack overflow (recursion depth {})", self.max_depth)', 'format!("栈溢出（递归深度 {}）", self.max_depth)'),
    ('"missing arguments".into()', '"调用参数缺失".into()'),
    ('"truncated instruction".into()', '"指令被截断".into()'),
    ('"stack underflow".into()', '"栈下溢".into()'),
    ('format!("cannot apply arithmetic to {} and {}", a.type_name(), b.type_name())', 'format!("无法对 {} 与 {} 做算术运算", a.type_name(), b.type_name())'),
    # lgb / lgpack
    ('write!(f, "bytecode error: {}", self.message)', 'write!(f, "字节码错误: {}", self.message)'),
    ('format!("runtime error: {}", e.message)', 'format!("运行时错误: {}", e.message)'),
    ('format!("runtime error at line {}: {}", e.line, e.message)', 'format!("运行时错误（第 {} 行）: {}", e.line, e.message)'),
    # module_loader common
    ('"compile failed: {e}"', '"编译失败: {e}"'),
    ('"cannot read `{}`: {e}", path.display()', '"无法读取 `{}`: {e}", path.display()'),
    ('"import path must not contain `..`: {rel}"', '"import 路径不能包含 `..`: {rel}"'),
    ('"cyclic import: {}", path.display()', '"循环 import: {}", path.display()'),
    ('"cannot resolve import `{}`"', '"无法解析 import `{}`"'),
    # parser expect messages - leave English "expected" for now via what param
]

# parser and checker format strings
MORE = [
    ('format!("expected {what}, found {}", self.kind())', 'format!("期望 {what}，实际是 {}", self.kind())'),
    ('format!("expected identifier, found {}", t.kind)', 'format!("期望标识符，实际是 {}", t.kind)'),
    ('format!("expected a type, found {}", t.kind)', 'format!("期望类型，实际是 {}", t.kind)'),
    ('format!("expected string path, found {other}")', 'format!("期望字符串路径，实际是 {other}")'),
    ('"import path must not contain `..`".into()', '"import 路径不能包含 `..`".into()'),
    ('"field type cannot be void".into()', '"字段类型不能是 void".into()'),
    ('"parameter type cannot be `void`".into()', '"参数类型不能是 void".into()'),
    ('"`void[]` is not a type".into()', '"`void[]` 不是合法类型".into()'),
    ('format!("expected expression, found {other}")', 'format!("期望表达式，实际是 {other}")'),
    # check
    ('format!("struct `{}` already defined", s.name.name)', 'format!("结构体 `{}` 重复定义", s.name.name)'),
    ('format!("duplicate field `{}`", f.name.name)', 'format!("字段 `{}` 重复", f.name.name)'),
    ('format!("`{}` already defined", c.name.name)', 'format!("`{}` 已定义", c.name.name)'),
    ('format!("const type must be a scalar, found `{declared}`")', 'format!("const 类型必须是标量，实际是 `{declared}`")'),
    ('format!("`{}` is not a compile-time constant", name.name)', 'format!("`{}` 不是编译期常量", name.name)'),
    ('format!("const `{}` declared `{declared}` but value is `{vt}`", c.name.name)', 'format!("const `{}` 标注为 `{declared}`，值却是 `{vt}`", c.name.name)'),
    ('format!("`{}` is a builtin", f.name.name)', 'format!("`{}` 是内建函数，不可重定义", f.name.name)'),
    ('format!("unknown struct `{}`", on.name)', 'format!("未知结构体 `{}`", on.name)'),
    ('format!("method receiver must be `{}`", on.name)', 'format!("方法接收者类型必须是 `{}`", on.name)'),
    ('format!("method `{}` conflicts with a function", f.name.name)', 'format!("方法 `{}` 与同名函数冲突", f.name.name)'),
    ('format!("method `{}` already defined", f.name.name)', 'format!("方法 `{}` 已定义", f.name.name)'),
    ('"`main` must be `fun main() -> void`".into()', '"`main` 必须是 `fun main() -> void`".into()'),
    ('format!("function `{}` already defined", f.name.name)', 'format!("函数 `{}` 重复定义", f.name.name)'),
    ('format!("function `{}` conflicts with a method", f.name.name)', 'format!("函数 `{}` 与方法名冲突", f.name.name)'),
    ('format!("function `{}` must return `{}` on all paths", f.name.name, self.ret)', 'format!("函数 `{}` 必须在所有路径返回 `{}`", f.name.name, self.ret)'),
    ('format!("variable `{n}` already declared in this scope")', 'format!("变量 `{n}` 在本作用域已声明")'),
    ('format!("struct `{st}` has no field `{}`", field.name)', 'format!("结构体 `{st}` 没有字段 `{}`", field.name)'),
    ('format!("let `{}` declared `{t}` but got `{vt}`", l.name.name)', 'format!("let `{}` 标注 `{t}`，但初始值是 `{vt}`", l.name.name)'),
    ('"empty `[]` requires a type annotation".into()', '"空数组 `[]` 需要类型标注，如 `int[]`".into()'),
    ('format!("`[]` needs array type, got `{t}`")', 'format!("`[]` 需要数组类型，实际是 `{t}`")'),
    ('format!("cannot assign to const `{}`", a.name.name)', 'format!("不能给 const `{}` 赋值", a.name.name)'),
    ('format!("undefined variable `{}`", a.name.name)', 'format!("未定义的变量 `{}`", a.name.name)'),
    ('"index must be `int`".into()', '"下标类型必须是 `int`".into()'),
    ('format!("cannot index `{}`", a.name.name)', 'format!("无法对 `{}` 做下标访问", a.name.name)'),
    ('format!("cannot index `{other}`")', 'format!("无法对 `{other}` 做下标访问")'),
    ('format!("cannot access field on `{}`", field.name)', 'format!("无法在该值上访问字段 `{}`", field.name)'),
    ('format!("cannot access field on `{other}`")', 'format!("无法在 `{other}` 上访问字段")'),
    ('format!("cannot assign `{vt}` to `{cur}`")', 'format!("不能把 `{vt}` 赋给 `{cur}`")'),
    ('"while condition must be `bool`".into()', '"while 条件必须是 `bool`".into()'),
    ('"for-in expects array, found `{other}`"', '"for-in 需要数组，实际是 `{other}`"'),
    ('"range bounds must be `int`".into()', '"范围两端类型必须是 `int`".into()'),
    ('"`break`/`continue` outside loop".into()', '"`break`/`continue` 只能用在循环内".into()'),
    ('"`return` outside function".into()', '"`return` 只能用在函数内".into()'),
    ('format!("must return `{}`", self.ret)', 'format!("此处必须返回 `{}`", self.ret)'),
    ('"void function cannot return a value".into()', '"void 函数不能返回值".into()'),
    ('format!("expected `{}`, found `{t}`", self.ret)', 'format!("期望返回类型 `{}`，实际是 `{t}`", self.ret)'),
    ('"if condition must be `bool`".into()', '"if 条件必须是 `bool`".into()'),
    ('format!("undefined variable `{}`", name.name)', 'format!("未定义的变量 `{}`", name.name)'),
    ('format!("cannot add `{lt}` and `{rt}`")', 'format!("不能将 `{lt}` 与 `{rt}` 相加")'),
    ('format!("cannot apply arithmetic to `{lt}` and `{rt}`")', 'format!("不能对 `{lt}` 与 `{rt}` 做算术运算")'),
    ('format!("cannot compare `{lt}` with `{rt}`")', 'format!("不能比较 `{lt}` 与 `{rt}`")'),
    ('format!("cannot order `{lt}` and `{rt}`")', 'format:"不能比较 `{lt}` 与 `{rt}` 的大小"'),  # typo risk
    ('"logical ops need `bool`".into()', '"逻辑运算需要 `bool`".into()'),
    ('format!("unknown struct `{}`", name.name)', 'format!("未知结构体 `{}`", name.name)'),
    ('format!("`{}` expects {} field(s)", name.name, decl.len())', 'format!("`{}` 需要 {} 个字段", name.name, decl.len())'),
    ('format!("no field `{}` on `{}`", fn_.name, name.name)', 'format!("`{}` 没有字段 `{}`", name.name, fn_.name)'),
    ('format!("field `{}`: expected `{exp}`, found `{got}`", fn_.name)', 'format!("字段 `{}`：期望 `{exp}`，实际是 `{got}`", fn_.name)'),
    ('"`print` takes 1 argument".into()', '"`print` 需要 1 个参数".into()'),
    ('"cannot print void".into()', '"不能 print void".into()'),
    ('"`len` takes 1 argument".into()', '"`len` 需要 1 个参数".into()'),
    ('format!("`len` expects array or string, found `{t}`")', 'format!("`len` 需要数组或字符串，实际是 `{t}`")'),
    ('format!("undefined function `{name}`")', 'format!("未定义函数 `{name}`")'),
    ('format!("no method `{}` on `{}`", method.name, name.name)', 'format!("`{}` 上没有方法 `{}`", name.name, method.name)'),
    ('format!("no method `{}` on `{sn}`", method.name)', 'format!("`{sn}` 上没有方法 `{}`", method.name)'),
    ('format!("cannot call method on `{rt}`")', 'format!("无法在 `{rt}` 上调用方法")'),
    ('format!("`{}` takes {} arg(s) after receiver", method.name, info.params.len() - 1)', 'format!("方法 `{}` 在接收者之后还需要 {} 个参数", method.name, info.params.len() - 1)'),
]

# fix the typo line
MORE = [(a, b) if not b.startswith('format:"') else (
    'format!("cannot order `{lt}` and `{rt}`")',
    'format!("不能比较 `{lt}` 与 `{rt}` 的大小")',
) for a, b in MORE]

def apply_all():
    files = list(Path("src").rglob("*.rs"))
    total_hits = 0
    for path in files:
        t = path.read_text(encoding="utf-8")
        orig = t
        for old, new in REPLACEMENTS + MORE:
            if old == new:
                continue
            if old in t:
                t = t.replace(old, new)
                total_hits += 1
        # generic diagnostic prefix
        t = t.replace(': error: {}', ': 错误: {}')
        t = t.replace('"error: {}"', '"错误: {}"')
        t = t.replace('runtime error: {}', '运行时错误: {}')
        t = t.replace('runtime error at line {}: {}', '运行时错误（第 {} 行）: {}')
        if t != orig:
            path.write_text(t, encoding="utf-8")
            print("updated", path)
    print("replacements applied:", total_hits)

apply_all()

# tests: update English assertions
tp = Path("tests/programs.rs")
if tp.exists():
    t = tp.read_text(encoding="utf-8")
    t = t.replace('contains("zero")', 'contains("零")')
    t = t.replace('contains("out of bounds")', 'contains("越界")')
    t = t.replace('contains("break") || err.contains("error")', 'contains("循环") || err.contains("错误") || err.contains("break")')
    t = t.replace('err.contains("error") || err.contains("错误")', 'err.contains("错误")')
    t = t.replace('.contains("error")', '.contains("错误")')
    t = t.replace('.contains("import")', '.contains("import")')
    tp.write_text(t, encoding="utf-8")
    print("tests updated")

# LANGUAGE.md diagnostics
ld = Path("docs/LANGUAGE.md")
if ld.exists():
    t = ld.read_text(encoding="utf-8")
    t = t.replace(
        "- 编译期：`file:line:col: error: message`\n- 运行时：`file:line: runtime error: message`（无行号时省略行号段）",
        "- 编译期：`file:line:col: 错误: message`（message 为中文）\n- 运行时：`file:line: 运行时错误: message`（无行号时省略行号段）",
    )
    ld.write_text(t, encoding="utf-8")
    print("LANGUAGE.md updated")
print("done")
