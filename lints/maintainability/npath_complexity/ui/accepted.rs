// compile-flags: --edition 2024

#![allow(dead_code)]

macro_rules! generated_paths {
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
    }};
}

fn below_limit(values: [bool; 7]) {
    if values[0] {}
    if values[1] {}
    if values[2] {}
    if values[3] {}
    if values[4] {}
    if values[5] {}
    if values[6] {}
}

fn match_and_branches_stay_below_limit(value: u8, flags: [bool; 5]) {
    match value {
        0 => {}
        1 => {}
        2 => {}
        _ => {}
    }
    if flags[0] {}
    if flags[1] {}
    if flags[2] {}
    if flags[3] {}
    if flags[4] {}
}

fn mutually_exclusive_branches_add(value: u8, flags: [bool; 5]) {
    match value {
        0 => {
            if flags[0] {}
            if flags[1] {}
            if flags[2] {}
            if flags[3] {}
            if flags[4] {}
        }
        1 => {
            if flags[0] {}
            if flags[1] {}
            if flags[2] {}
            if flags[3] {}
            if flags[4] {}
        }
        2 => {
            if flags[0] {}
            if flags[1] {}
            if flags[2] {}
            if flags[3] {}
            if flags[4] {}
        }
        _ => {
            if flags[0] {}
            if flags[1] {}
            if flags[2] {}
            if flags[3] {}
            if flags[4] {}
        }
    }
}

fn macro_control_flow_is_opaque() {
    generated_paths!();
}

fn nested_callable_is_separate(values: [bool; 7]) {
    let closure = || {
        if values[0] {}
        if values[1] {}
        if values[2] {}
        if values[3] {}
        if values[4] {}
        if values[5] {}
        if values[6] {}
    };
    closure();
}

async fn await_is_not_a_path_branch() {
    async {}.await;
}

fn main() {}
