#![allow(dead_code)]

pub struct Uuid(String);

enum Uri {
    Absolute,
}

type Duration = u64;

type Millis = u64;
type Instant = Millis;

struct PaymentMethod(String);

mod custom_wrappers {
    pub struct PathBuf(std::path::PathBuf);

    pub struct Duration<T>(T);

    pub enum Method {
        Get,
        Post,
    }
}

mod actual_aliases {
    use std::path::Path as StdPath;
    use std::path::PathBuf as StdPathBuf;
    use std::time::Duration as StdDuration;

    type Path = StdPath;
    type PathBuf = StdPathBuf;
    type Duration = StdDuration;
    type Instant = std::time::Instant;
}

mod local_lookalike_aliases {
    pub struct Url(String);

    type URL = Url;
}

mod nested {
    pub mod http {
        pub enum Uri {
            Absolute,
            Relative,
        }
    }
}

#[cfg(any())]
mod disabled {
    pub struct Url(String);
}

fn main() {}
