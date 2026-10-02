#![allow(dead_code)]

fn too_nested(a: bool, b: bool, c: bool, d: bool, e: bool, f: bool) {
    if a {
        if b {
            if c {
                if d {
                    if e {
                        if f {}
                    }
                }
            }
        }
    }
}

fn nesting_in_macro_arguments(a: [bool; 5]) {
    println!(
        "{}",
        if a[0] {
            if a[1] {
                if a[2] {
                    if a[3] {
                        if a[4] { 1 } else { 0 }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        }
    );
}

fn main() {}
