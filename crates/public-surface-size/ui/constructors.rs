// compile-flags: --edition 2024 --crate-type lib

pub fn f01() {}
pub fn f02() {}
pub fn f03() {}
pub fn f04() {}
pub fn f05() {}
pub fn f06() {}
pub fn f07() {}
pub fn f08() {}
pub fn f09() {}
pub fn f10() {}
pub fn f11() {}
pub fn f12() {}
pub fn f13() {}
pub fn f14() {}
pub fn f15() {}
pub fn f16() {}
pub fn f17() {}
pub fn f18() {}
pub fn f19() {}
pub fn f20() {}
pub fn f21() {}

pub struct Tuple(u8);
pub use Tuple as TupleAlias;

pub enum Kind {
    Variant,
}

pub use Kind::Variant as VariantAlias;
