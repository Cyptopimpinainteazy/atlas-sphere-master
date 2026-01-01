//! Sample X3 strategy sources for benchmarking.
//!
//! These are simplified X3 programs that exercise different patterns:
//! - Arithmetic and control flow
//! - Function calls
//! - Conditionals
//! - Loops (when supported)

/// Provide a suite of sample X3 sources for benchmarking.
pub fn sample_suite() -> Vec<(&'static str, &'static str)> {
    vec![
        ("constant_fold_heavy", CONSTANT_FOLD_HEAVY),
        ("arithmetic_chain", ARITHMETIC_CHAIN),
        ("conditional_logic", CONDITIONAL_LOGIC),
        ("dead_code_sample", DEAD_CODE_SAMPLE),
        ("copy_chain", COPY_CHAIN),
        ("peephole_targets", PEEPHOLE_TARGETS),
        ("simple_function", SIMPLE_FUNCTION),
        ("multi_function", MULTI_FUNCTION),
    ]
}

/// Heavy constant folding opportunities.
const CONSTANT_FOLD_HEAVY: &str = r#"
fn main() -> i64 {
    let a = 10 + 20;
    let b = 30 * 2;
    let c = a + b;
    let d = 100 - 50;
    let e = c + d;
    return e;
}
"#;

/// Chain of arithmetic operations.
const ARITHMETIC_CHAIN: &str = r#"
fn compute(x: i64) -> i64 {
    let a = x + 1;
    let b = a * 2;
    let c = b - 3;
    let d = c + 4;
    let e = d * 5;
    return e;
}

fn main() -> i64 {
    return compute(10);
}
"#;

/// Conditional logic with branches.
const CONDITIONAL_LOGIC: &str = r#"
fn classify(value: i64) -> i64 {
    if value > 100 {
        return 3;
    }
    if value > 50 {
        return 2;
    }
    if value > 0 {
        return 1;
    }
    return 0;
}

fn main() -> i64 {
    return classify(75);
}
"#;

/// Code with dead assignments that DCE should remove.
const DEAD_CODE_SAMPLE: &str = r#"
fn main() -> i64 {
    let x = 10;
    let y = 20;
    let unused1 = x + 100;
    let unused2 = y * 2;
    let unused3 = 42;
    let result = x + y;
    return result;
}
"#;

/// Chain of copies for copy propagation.
const COPY_CHAIN: &str = r#"
fn main() -> i64 {
    let original = 42;
    let copy1 = original;
    let copy2 = copy1;
    let copy3 = copy2;
    return copy3;
}
"#;

/// Patterns targeted by peephole optimization.
const PEEPHOLE_TARGETS: &str = r#"
fn main() -> i64 {
    let x = 10;
    let a = x + 0;
    let b = x * 1;
    let c = x - 0;
    let d = a + b + c;
    return d;
}
"#;

/// Simple single function.
const SIMPLE_FUNCTION: &str = r#"
fn add(a: i64, b: i64) -> i64 {
    return a + b;
}

fn main() -> i64 {
    return add(10, 20);
}
"#;

/// Multiple functions calling each other.
const MULTI_FUNCTION: &str = r#"
fn double(x: i64) -> i64 {
    return x * 2;
}

fn add_ten(x: i64) -> i64 {
    return x + 10;
}

fn process(x: i64) -> i64 {
    let a = double(x);
    let b = add_ten(a);
    return b;
}

fn main() -> i64 {
    return process(5);
}
"#;
