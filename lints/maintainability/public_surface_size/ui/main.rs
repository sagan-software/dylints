// compile-flags: --edition 2024 --crate-type lib

macro_rules! public_functions {
    ($($name:ident),+ $(,)?) => { $(pub fn $name() {})+ };
}

public_functions!(
    f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11, f12, f13, f14, f15, f16, f17, f18, f19,
    f20, f21, f22, f23, f24, f25, f26,
);
