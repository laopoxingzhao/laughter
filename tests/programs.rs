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
