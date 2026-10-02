#![allow(dead_code)]

struct Config {
    retries: u8,
}

type ConfigAlias = Config;

impl Default for Config {
    fn default() -> Self {
        Self { retries: 3 }
    }
}

impl Config {
    fn new() -> Self {
        Self { retries: 3 }
    }

    fn empty() -> Config {
        Config { retries: 3 }
    }

    fn blank() -> ConfigAlias {
        Config { retries: 3 }
    }

    fn with_retries(retries: u8) -> Self {
        Self { retries }
    }
}

struct Factory;

impl Factory {
    fn new() -> Config {
        Config { retries: 0 }
    }

    fn empty(_name: &str) -> Self {
        Self
    }
}

mod qualified {
    pub struct Qualified;

    impl Qualified {
        pub fn default_config() -> crate::qualified::Qualified {
            Self
        }
    }
}

fn blank() -> Config {
    Config { retries: 0 }
}

fn main() {}
