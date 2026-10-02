// compile-flags: --edition 2024

#![allow(dead_code)]

fn call() {}

fn exceeds_limit() {
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
    call();
}

fn too_many_assignments() {
    let _a01 = 1;
    let _a02 = 2;
    let _a03 = 3;
    let _a04 = 4;
    let _a05 = 5;
    let _a06 = 6;
    let _a07 = 7;
    let _a08 = 8;
    let _a09 = 9;
    let _a10 = 10;
    let _a11 = 11;
    let _a12 = 12;
    let _a13 = 13;
    let _a14 = 14;
    let _a15 = 15;
    let _a16 = 16;
    let _a17 = 17;
    let _a18 = 18;
    let _a19 = 19;
    let _a20 = 20;
    let _a21 = 21;
    let _a22 = 22;
    let _a23 = 23;
    let _a24 = 24;
    let _a25 = 25;
    let _a26 = 26;
}

fn too_many_conditions(values: [bool; 26]) {
    if values[0] {}
    if values[1] {}
    if values[2] {}
    if values[3] {}
    if values[4] {}
    if values[5] {}
    if values[6] {}
    if values[7] {}
    if values[8] {}
    if values[9] {}
    if values[10] {}
    if values[11] {}
    if values[12] {}
    if values[13] {}
    if values[14] {}
    if values[15] {}
    if values[16] {}
    if values[17] {}
    if values[18] {}
    if values[19] {}
    if values[20] {}
    if values[21] {}
    if values[22] {}
    if values[23] {}
    if values[24] {}
    if values[25] {}
}

fn main() {}
