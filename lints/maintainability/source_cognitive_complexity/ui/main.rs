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

fn main() {}
