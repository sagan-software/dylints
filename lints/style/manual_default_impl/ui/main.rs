// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

struct Config {
    retries: u8,
    labels: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            retries: Default::default(),
            labels: <Vec<String>>::default(),
        }
    }
}

struct Qualified {
    enabled: bool,
}

impl std::default::Default for Qualified {
    fn default() -> Self {
        Self {
            enabled: std::default::Default::default(),
        }
    }
}

struct CustomLiteral {
    retries: u8,
}

impl Default for CustomLiteral {
    fn default() -> Self {
        Self { retries: 3 }
    }
}

struct CustomFunction {
    name: String,
}

impl Default for CustomFunction {
    fn default() -> Self {
        Self { name: make_name() }
    }
}

fn make_name() -> String {
    String::new()
}

struct ConditionalDefault {
    enabled: bool,
}

impl Default for ConditionalDefault {
    fn default() -> Self {
        if cfg!(test) {
            Self { enabled: true }
        } else {
            Self {
                enabled: Default::default(),
            }
        }
    }
}

struct MethodDefault {
    value: String,
}

trait LocalDefaultMethod {
    fn default(self) -> Self;
}

impl LocalDefaultMethod for String {
    fn default(self) -> Self {
        self
    }
}

impl Default for MethodDefault {
    fn default() -> Self {
        Self {
            value: String::new().default(),
        }
    }
}

#[derive(Debug)]
pub struct Literals {
    count: u32,
    enabled: bool,
    name: Option<String>,
}

impl Default for Literals {
    fn default() -> Self {
        Literals {
            count: 0,
            enabled: false,
            name: None,
        }
    }
}

struct Pair(u8, Vec<u8>);

impl Default for Pair {
    fn default() -> Self {
        Self(u8::default(), Vec::default())
    }
}

struct Unit;

impl Default for Unit {
    fn default() -> Self {
        Self
    }
}

struct Documented {
    retries: u8,
}

/// The documentation stays with this impl, so the lint offers no fix.
impl Default for Documented {
    fn default() -> Self {
        Self {
            retries: Default::default(),
        }
    }
}

struct Generic<T> {
    values: Vec<T>,
}

impl<T> Default for Generic<T> {
    fn default() -> Self {
        Self {
            values: Vec::default(),
        }
    }
}

struct WithBase {
    first: u8,
    second: u8,
}

impl Default for WithBase {
    fn default() -> Self {
        let base = Self {
            first: 0,
            second: 0,
        };
        Self { first: 0, ..base }
    }
}

struct NonZero {
    count: u8,
    enabled: bool,
}

impl Default for NonZero {
    fn default() -> Self {
        Self {
            count: 1,
            enabled: true,
        }
    }
}

enum Mode {
    Fast,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Fast
    }
}

macro_rules! generated_default {
    ($name:ident) => {
        struct $name {
            value: u8,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    value: Default::default(),
                }
            }
        }
    };
}

generated_default!(Generated);

fn main() {}
