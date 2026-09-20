use laughter::vm::run_source;

// The binary crate exposes modules only to itself; integration tests need a lib.
// These tests invoke the same pipeline via a thin approach: we duplicate isn't needed
// if we add src/lib.rs. For now keep tests in unit modules + these script-level tests
// through std::process once built — simpler: declare lib.

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
    let out = run_source(include_str!("../examples/fun.lg")).unwrap();
    assert_eq!(out, vec!["55"]);
}

#[test]
fn arrays_main() {
    let out = run_source(include_str!("../examples/arrays.lg")).unwrap();
    assert_eq!(out, vec!["1", "3", "[1, 20, 3]", "24"]);
}

#[test]
fn type_error_rejected() {
    let err = run_source("let x = 1 + 2.5;").unwrap_err();
    assert!(err.contains("error"));
}

#[test]
fn oob_is_runtime_error() {
    let err = run_source("let a = [1]; print(a[2]);").unwrap_err();
    assert!(err.contains("out of bounds"));
}

#[test]
fn div_by_zero() {
    let err = run_source("print(1 / 0);").unwrap_err();
    assert!(err.contains("division by zero"));
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
