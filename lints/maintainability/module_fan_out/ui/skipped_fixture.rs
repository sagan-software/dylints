#![allow(dead_code)]

mod a {
    pub fn call() {}
}
mod b {
    pub fn call() {}
}
mod c {
    pub fn call() {}
}
mod d {
    pub fn call() {}
}
mod e {
    pub fn call() {}
}
mod f {
    pub fn call() {}
}
mod g {
    pub fn call() {}
}
mod h {
    pub fn call() {}
}
mod i {
    pub fn call() {}
}

mod coordinator {
    pub fn run() {
        crate::a::call();
        crate::b::call();
        crate::c::call();
        crate::d::call();
        crate::e::call();
        crate::f::call();
        crate::g::call();
        crate::h::call();
        crate::i::call();
    }
}

fn main() {}
