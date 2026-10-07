// compile-flags: --edition 2024

#![allow(dead_code)]

struct FocusedType;

impl FocusedType {
    fn m01(&self) {}
    fn m02(&self) {}
    fn m03(&self) {}
    fn m04(&self) {}
    fn m05(&self) {}
    fn m06(&self) {}
    fn m07(&self) {}
    fn m08(&self) {}
    fn m09(&self) {}
    fn m10(&self) {}
    fn m11(&self) {}
    fn m12(&self) {}
    fn m13(&self) {}
    fn m14(&self) {}
    fn m15(&self) {}
    fn m16(&self) {}
    fn m17(&self) {}
    fn m18(&self) {}
    fn m19(&self) {}
    fn m20(&self) {}
}

trait ExtraBehavior {
    fn extra(&self);
}

impl ExtraBehavior for FocusedType {
    fn extra(&self) {}
}

trait Marker {}

impl dyn Marker {
    fn describe(&self) {}
}

fn main() {}
