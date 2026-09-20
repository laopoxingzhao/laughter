//! 集成测试：通过 `laughter::vm::run_source` 跑 `.lg` 片段与 examples。

use laughter::vm::{run_source, run_source_file};

#[test]
fn hello() {
    let out = run_source("print(\"hello, laughter\");").unwrap();
    assert_eq!(out, vec!["hello, laughter"]);
}

#[test]
fn vars_and_ops() {
    let out = run_source(
        r#"
        let x: int = 1;
        let y = 2;
        x = x + y * 3;
        print(x);
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["7"]);
}

#[test]
fn while_sum() {
    let out = run_source(
        r#"
        let n = 10;
        let acc = 0;
        while n > 0 {
            acc = acc + n;
            n = n - 1;
        }
        print(acc);
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["55"]);
}

#[test]
fn fib_main() {
    let out = run_source(include_str!("../examples/fib.lg")).unwrap();
    assert_eq!(out, vec!["55"]);
}

#[test]
fn fun_example_alias() {
    let out = run_source(include_str!("../examples/fun.lg")).unwrap();
    assert_eq!(out, vec!["55"]);
}

#[test]
fn arrays_main() {
    let out = run_source(include_str!("../examples/arrays.lg")).unwrap();
    assert_eq!(out, vec!["1", "3", "[1, 20, 3]", "24"]);
}

#[test]
fn if_without_else_does_not_corrupt_stack() {
    let out = run_source(
        r#"
        fun f() -> int {
            if true {
                print(1);
            }
            return 2;
        }
        fun main() -> void {
            print(f());
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["1", "2"]);
}

#[test]
fn if_false_without_else() {
    let out = run_source(
        r#"
        fun f() -> int {
            let x = 10;
            if false {
                print(0);
            }
            return x;
        }
        fun main() -> void {
            print(f());
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["10"]);
}

#[test]
fn empty_array_with_annotation() {
    let out = run_source(
        r#"
        fun main() -> void {
            let a: int[] = [];
            print(len(a));
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["0"]);
}

#[test]
fn type_error_rejected() {
    let err = run_source("let x = 1 + 2.5;").unwrap_err();
    assert!(err.contains("error"));
}

#[test]
fn type_error_fixture() {
    let src = include_str!("fixtures/type_error.lg");
    let err = run_source_file("tests/fixtures/type_error.lg", src).unwrap_err();
    assert!(err.starts_with("tests/fixtures/type_error.lg:"), "{err}");
    assert!(err.contains("error"), "{err}");
}

#[test]
fn error_format_includes_file() {
    let err = run_source_file("foo.lg", "let x = 1 + 2.5;").unwrap_err();
    assert!(err.starts_with("foo.lg:"), "{err}");
    assert!(err.contains(": error:"), "{err}");
}

#[test]
fn oob_is_runtime_error() {
    let err = run_source("let a = [1]; print(a[2]);").unwrap_err();
    assert!(err.contains("out of bounds"), "{err}");
}

#[test]
fn div_by_zero() {
    let err = run_source("print(1 / 0);").unwrap_err();
    assert!(err.contains("division by zero"), "{err}");
}

#[test]
fn string_concat() {
    let out = run_source("print(\"a\" + \"b\");").unwrap();
    assert_eq!(out, vec!["ab"]);
}

#[test]
fn short_circuit() {
    let out = run_source(
        r#"
        fun boom() -> bool {
            return true;
        }
        fun main() -> void {
            print(false && boom());
            print(true || boom());
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["false", "true"]);
}

#[test]
fn struct_example() {
    let out = run_source(include_str!("../examples/struct.lg")).unwrap();
    assert_eq!(
        out,
        vec!["1", "30", "Point { x: 1, y: 30 }", "2", "1", "30", "4", "5"]
    );
}

#[test]
fn strings_example() {
    let out = run_source(include_str!("../examples/strings.lg")).unwrap();
    assert_eq!(out, vec!["5", "h", "ell", "42", "true", "hello!"]);
}

#[test]
fn forin_example() {
    let out = run_source(include_str!("../examples/forin.lg")).unwrap();
    assert_eq!(out, vec!["[1, 2, 3, 4]", "4", "6", "3"]);
}

#[test]
fn push_pop_runtime() {
    let out = run_source(
        r#"
        fun main() -> void {
            let a: int[] = [];
            push(a, 7);
            push(a, 8);
            print(pop(a));
            print(len(a));
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["8", "1"]);
}

#[test]
fn str_at_oob() {
    let err = run_source("print(str_at(\"ab\", 5));").unwrap_err();
    assert!(err.contains("out of bounds"), "{err}");
}

#[test]
fn for_in_non_array_rejected() {
    let err = run_source("for x in 3 { print(x); }").unwrap_err();
    assert!(err.contains("error"), "{err}");
}

#[test]
fn struct_missing_field_rejected() {
    let src = r#"
    struct P { x: int, y: int }
    fun main() -> void { let p = P { x: 1 }; }
    "#;
    let err = run_source(src).unwrap_err();
    assert!(err.contains("error"), "{err}");
}

#[test]
fn empty_control_flow_bodies() {
    let out = run_source(
        r#"
        fun main() -> void {
            let a: int[] = [1, 2];
            for x in a { }
            let c = false;
            while c { }
            if c { } else { }
            print("ok");
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["ok"]);
}

#[test]
fn input_typechecks() {
    // 只检查能否编译；不在测试里真正读 stdin
    let err = run_source("fun main() -> void { let s = input(); }");
    assert!(err.is_ok(), "{err:?}");
}

#[test]
fn empty_struct_lit_is_error_or_parse_ok_block() {
    // 空字面量不被 lookahead 吃成 struct lit；`P {}` 应解析失败或语义失败
    let src = r#"
    struct P { x: int }
    fun main() -> void { let p = P { }; }
    "#;
    let err = run_source(src).unwrap_err();
    assert!(err.contains("error"), "{err}");
}

#[test]
fn methods_range_example() {
    let out = laughter::loader::run_file("examples/methods_range.lg").unwrap();
    assert_eq!(out, vec!["7", "7", "0", "1", "3", "6"]);
}

#[test]
fn import_example() {
    let out = laughter::loader::run_file("examples/mod_main.lg").unwrap();
    assert_eq!(out, vec!["5", "42"]);
}

#[test]
fn range_for_break_continue() {
    let out = run_source(
        r#"
        fun main() -> void {
            for i in 0..4 {
                if i == 1 { continue; }
                if i == 3 { break; }
                print(i);
            }
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["0", "2"]);
}

#[test]
fn struct_method_instance_call() {
    let out = run_source(
        r#"
        struct P { x: int }
        fun P.get(self: P) -> int { return self.x; }
        fun main() -> void {
            let p = P { x: 9 };
            print(p.get());
        }
        "#,
    )
    .unwrap();
    assert_eq!(out, vec!["9"]);
}

#[test]
fn import_in_run_source_rejected() {
    let err = run_source("import \"x.lg\"; fun main() -> void {}");
    assert!(err.unwrap_err().contains("import"));
}

#[test]
fn break_outside_loop_rejected() {
    let err = run_source("fun main() -> void { break; }").unwrap_err();
    assert!(err.contains("break") || err.contains("error"), "{err}");
}
