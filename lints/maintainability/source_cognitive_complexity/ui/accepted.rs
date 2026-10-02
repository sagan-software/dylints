// compile-flags: --edition 2024

#![allow(dead_code)]

macro_rules! generated_nesting {
    () => {{
        if true {
            if true {
                if true {
                    if true {
                        if true {
                            if true {}
                        }
                    }
                }
            }
        }
    }};
}

fn exactly_at_limit(a: bool, b: bool, c: bool, d: bool, e: bool) {
    if a {
        if b {
            if c {
                if d {}
            }
        }
    }
    if a {}
    if b {}
    if c {}
    if e {}
}

fn flat_else_if_chain(value: u8) {
    if value == 0 {
    } else if value == 1 {
    } else if value == 2 {
    } else if value == 3 {
    } else if value == 4 {
    } else if value == 5 {
    }
}

fn macro_control_flow_is_opaque() {
    generated_nesting!();
}

async fn await_and_try_are_shorthand(value: Option<u8>) -> Option<u8> {
    async {}.await;
    Some(value?)
}

fn nested_callable_is_separate(a: bool, b: bool, c: bool, d: bool, e: bool) {
    let closure = || {
        if a {
            if b {
                if c {
                    if d {}
                }
            }
        }
        if e {}
    };
    closure();
}

fn while_desugaring_is_not_nested_control_flow(a: bool, b: bool, c: bool, d: bool) {
    while a {
        while b {
            while c {
                while d {}
            }
        }
    }
}

fn main() {}
