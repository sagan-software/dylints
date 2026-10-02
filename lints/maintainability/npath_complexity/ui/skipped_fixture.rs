#![allow(dead_code)]

fn too_many_paths(values: [bool; 9]) {
    if values[0] {}
    if values[1] {}
    if values[2] {}
    if values[3] {}
    if values[4] {}
    if values[5] {}
    if values[6] {}
    if values[7] {}
    if values[8] {}
}

#[rustfmt::skip]
fn paths_in_macro_arguments(f: [bool; 8]) {
    println!(
        "{}{}{}{}{}{}{}{}",
        if f[0] { 1 } else { 0 }, if f[1] { 1 } else { 0 }, if f[2] { 1 } else { 0 }, if f[3] { 1 } else { 0 }, if f[4] { 1 } else { 0 }, if f[5] { 1 } else { 0 }, if f[6] { 1 } else { 0 }, if f[7] { 1 } else { 0 }
    );
}

fn try_operators_double(values: [Option<u8>; 8]) -> Option<u8> {
    let _v0 = values[0]?;
    let _v1 = values[1]?;
    let _v2 = values[2]?;
    let _v3 = values[3]?;
    let _v4 = values[4]?;
    let _v5 = values[5]?;
    let _v6 = values[6]?;
    let _v7 = values[7]?;
    None
}

fn main() {}
