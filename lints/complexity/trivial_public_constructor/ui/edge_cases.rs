#![allow(dead_code, non_upper_case_globals)]

struct Token {
    pub value: String,
    pub scope: String,
}

impl Token {
    pub fn new(value: String, scope: String) -> Self {
        Self { value, scope }
    }

    pub fn with_default_scope(value: String) -> Self {
        Self {
            value,
            scope: "default".to_owned(),
        }
    }
}

pub struct TupleToken(String);

impl TupleToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

#[non_exhaustive]
pub struct Extensible {
    pub value: u32,
}

impl Extensible {
    pub fn new(value: u32) -> Self {
        Self { value }
    }
}

pub struct Internal {
    pub value: u32,
}

impl Internal {
    pub(crate) fn new(value: u32) -> Self {
        Self { value }
    }
}

pub trait Build {
    fn new(value: u32) -> Self;
}

pub struct ViaTrait {
    pub value: u32,
}

impl Build for ViaTrait {
    fn new(value: u32) -> Self {
        Self { value }
    }
}

const limit: u32 = 3;

pub struct WithConstant {
    pub value: u32,
    pub limit: u32,
}

impl WithConstant {
    pub fn new(value: u32, _unused: u32) -> Self {
        Self { value, limit }
    }
}

pub struct Destructured {
    pub value: u32,
}

impl Destructured {
    pub fn new((value,): (u32,)) -> Self {
        Self { value }
    }
}

pub enum Shape {
    Square { side: u32 },
}

impl Shape {
    pub fn new(side: u32) -> Self {
        Self::Square { side }
    }
}

macro_rules! constructor {
    ($name:ident) => {
        pub struct $name {
            pub value: u32,
        }

        impl $name {
            pub fn new(value: u32) -> Self {
                Self { value }
            }
        }
    };
}

constructor!(Generated);

pub struct WithStatement {
    pub value: u32,
}

impl WithStatement {
    pub fn new(value: u32) -> Self {
        let value = value + 1;
        Self { value }
    }
}

fn main() {}
