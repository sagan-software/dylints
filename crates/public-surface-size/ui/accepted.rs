// compile-flags: --edition 2024 --crate-type lib

macro_rules! public_functions {
    ($($name:ident),+ $(,)?) => { $(pub fn $name() {})+ };
}

public_functions!(
    f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11, f12, f13, f14, f15, f16, f17, f18, f19,
    f20, f21, f22, f23, f24, f25,
);

mod private_implementation {
    public_functions!(
        p01, p02, p03, p04, p05, p06, p07, p08, p09, p10, p11, p12, p13, p14, p15, p16, p17, p18,
        p19, p20, p21, p22, p23, p24, p25, p26,
    );
}
