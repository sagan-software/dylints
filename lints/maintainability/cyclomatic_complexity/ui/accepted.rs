// compile-flags: --edition 2024

#![allow(dead_code)]

macro_rules! generated_branches {
    () => {{
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
        if true {}
    }};
}

fn exactly_at_limit(values: [bool; 9]) {
    if values[0] {}
    if values[1] {}
    if values[2] {}
    if values[3] {}
    if values[4] {}
    if values[5] {}
    if values[6] {}
    if values[7] {}
}

fn macro_control_flow_is_opaque() {
    generated_branches!();
}

fn nested_callable_is_separate(values: [bool; 9]) {
    let closure = || {
        if values[0] {}
        if values[1] {}
        if values[2] {}
        if values[3] {}
        if values[4] {}
        if values[5] {}
        if values[6] {}
        if values[7] {}
    };
    closure();
}

async fn await_is_not_a_decision() {
    async {}.await;
    async {}.await;
}

fn plain_loop_is_not_an_extra_decision() {
    loop {
        break;
    }
}

fn while_desugaring_is_not_a_second_decision(values: [bool; 5]) {
    while values[0] {}
    while values[1] {}
    while values[2] {}
    while values[3] {}
    while values[4] {}
}

fn cfg_disabled_code_is_absent() {
    #[cfg(any())]
    if true {}
}

fn main() {}
