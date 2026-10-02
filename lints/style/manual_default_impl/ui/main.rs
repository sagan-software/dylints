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

fn main() {}
