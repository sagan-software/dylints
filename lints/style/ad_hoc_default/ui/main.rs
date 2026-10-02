#![allow(dead_code)]

struct Config {
    retries: u8,
}

type ConfigAlias = Config;

impl Config {
    const DEFAULT_RETRIES: u8 = 3;

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

struct WithDefault {
    retries: u8,
}

impl Default for WithDefault {
    fn default() -> Self {
        Self { retries: 3 }
    }
}

impl WithDefault {
    fn new() -> Self {
        Self::default()
    }
}

struct Wrapper<T>(Vec<T>);

impl<T> Default for Wrapper<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> Wrapper<T> {
    fn new() -> Self {
        Self(Vec::new())
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

trait Builder {
    fn new() -> Self;
}

impl Builder for Factory {
    fn new() -> Self {
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
