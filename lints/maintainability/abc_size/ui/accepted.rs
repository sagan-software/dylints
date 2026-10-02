// compile-flags: --edition 2024

#![allow(dead_code)]

fn call() {}

fn exactly_at_limit() {
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
    call();
}

macro_rules! generated_binding {
    () => {
        let _generated = call();
    };
}

fn conditions_and_assignments(value: u8, a: bool, b: bool) -> u8 {
    let mut total = 0;
    total = total + 1;
    total += 1;
    match value {
        0 => total += 1,
        n if n > 5 => total += 2,
        _ => {}
    }
    if a && b || a {
        total += 1;
    }
    generated_binding!();
    total
}

fn main() {}
