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

macro_rules! generated_fan_out {
    () => {{
        crate::a::call();
        crate::b::call();
        crate::c::call();
        crate::d::call();
        crate::e::call();
        crate::f::call();
        crate::g::call();
        crate::h::call();
        crate::i::call();
    }};
}

mod exactly_at_limit {
    pub fn run() {
        crate::a::call();
        crate::b::call();
        crate::c::call();
        crate::d::call();
        crate::e::call();
        crate::f::call();
        crate::g::call();
    }
}

mod duplicate_references_count_once {
    pub fn run() {
        crate::a::call();
        crate::a::call();
        crate::a::call();
    }
}

mod macro_edges_are_opaque {
    pub fn run() {
        generated_fan_out!();
    }
}

mod nested_modules_collapse {
    pub mod first {
        pub fn call() {
            super::second::call();
        }
        pub fn leaf() {}
    }
    pub mod second {
        pub fn call() {
            super::first::leaf();
        }
    }
}

fn external_crates_are_excluded() {
    std::mem::drop(String::new());
}

mod method_target {
    pub struct Target;

    impl Target {
        pub fn act(&self) {}
    }
}

mod method_edges {
    macro_rules! generated_method_call {
        ($target:expr) => {
            $target.act()
        };
    }

    pub fn run(target: &crate::method_target::Target) {
        target.act();
        generated_method_call!(target);
        let _length = String::new().len();
    }
}

fn main() {}
