//! 集成测试：对照 docs/LANGUAGE.md。

use laughter::module_loader::run_file;
use laughter::runtime::vm::{compile_source, run_source};

fn run(src: &str) -> Vec<String> {
    run_source(src).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn hello_and_arith() {
    assert_eq!(run("print(\"hi\");"), vec!["hi"]);
    assert_eq!(run("print(1 + 2 * 3);"), vec!["7"]);
}

#[test]
fn fib() {
    let out = run_file("examples/fib.lg").unwrap();
    assert_eq!(out, vec!["55"]);
}

#[test]
fn all_examples() {
    for (path, expect) in [
        ("examples/hello.lg", vec!["hello, laughter"]),
        ("examples/vars.lg", vec!["7", "sum=", "7", "4.0", "ok"]),
        ("examples/branch.lg", vec!["55", "fifty-five"]),
        ("examples/fun.lg", vec!["55"]),
        ("examples/fib.lg", vec!["55"]),
        ("examples/arrays.lg", vec!["1", "3", "[1, 20, 3]", "24"]),
        (
            "examples/struct.lg",
            vec!["1", "30", "Point { x: 1, y: 30 }", "2", "1", "30", "4", "5"],
        ),
        (
            "examples/strings.lg",
            vec!["5", "h", "ell", "42", "true", "hello!"],
        ),
        ("examples/forin.lg", vec!["[1, 2, 3, 4]", "4", "6", "3"]),
        (
            "examples/methods_range.lg",
            vec!["7", "7", "0", "1", "3", "6"],
        ),
        ("examples/mod_main.lg", vec!["5", "42"]),
        (
            "examples/zca.lg",
            vec!["21", "zca", "1", "50", "103", "1", "25"],
        ),
    ] {
        let out = run_file(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(out, expect, "mismatch in {path}");
    }
}

#[test]
fn const_fold() {
    let out =
        run("const N: int = 10;\nconst M: int = N * 2 + 1;\nfun main() -> void { print(M); }");
    assert_eq!(out, vec!["21"]);
    let m = compile_source("const M: int = 10 * 2;\nfun main() -> void { print(M); }").unwrap();
    let f = m.functions.iter().find(|f| f.name == "main").unwrap();
    let d = f.chunk.disassemble("m");
    assert!(d.contains("(20)"), "{d}");
}

#[test]
fn value_semantics() {
    let out = run(r#"
        struct P { x: int }
        fun main() -> void {
            let a = P { x: 1 };
            let b = a;
            b.x = 9;
            print(a.x);
            print(b.x);
        }
        "#);
    assert_eq!(out, vec!["1", "9"]);
}

#[test]
fn multi_level_field_assign() {
    let out = run(r#"
        struct In { v: int }
        struct Out { inner: In }
        fun main() -> void {
            let o = Out { inner: In { v: 1 } };
            o.inner.v = 42;
            print(o.inner.v);
        }
        "#);
    assert_eq!(out, vec!["42"]);
}

#[test]
fn break_continue_and_range() {
    let out = run(r#"
        fun main() -> void {
            for i in 0..4 {
                if i == 1 { continue; }
                if i == 3 { break; }
                print(i);
            }
        }
        "#);
    assert_eq!(out, vec!["0", "2"]);
}

#[test]
fn type_errors() {
    assert!(run_source("let x = 1 + 2.5;").is_err());
    assert!(run_source("const N: int = 1;\nfun main() -> void { N = 2; }").is_err());
    assert!(run_source("fun main() -> void { break; }").is_err());
    assert!(run_source("import \"x.lg\"; fun main() -> void {}").is_err());
    let e = run_source("struct P { x: int }\nfun main() -> void { let p = P { }; }");
    assert!(e.is_err());
}

#[test]
fn runtime_errors() {
    assert!(run_source("print(1 / 0);").unwrap_err().contains("zero"));
    assert!(run_source("let a = [1]; print(a[3]);").is_err());
}

#[test]
fn methods_and_arrays_field() {
    let out = run(r#"
        struct P { x: int }
        fun P.get(self: P) -> int { return self.x; }
        fun main() -> void {
            let a: P[] = [P { x: 1 }];
            a[0].x = 7;
            print(a[0].get());
        }
        "#);
    assert_eq!(out, vec!["7"]);
}
