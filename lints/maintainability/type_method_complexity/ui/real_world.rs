#![allow(dead_code)]

// These small impls preserve responsibility shapes from popular crates.
// Types and operations are reduced so the fixture uses only std.

// Case 091: reduced from a serde_json parser implementation.
struct RealWorld091;
impl RealWorld091 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 092: reduced from a tokio scheduler implementation.
struct RealWorld092;
impl RealWorld092 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 093: reduced from a clap command implementation.
struct RealWorld093;
impl RealWorld093 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 094: reduced from a reqwest redirect implementation.
struct RealWorld094;
impl RealWorld094 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 095: reduced from a regex compiler implementation.
struct RealWorld095;
impl RealWorld095 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 096: reduced from a tracing dispatcher implementation.
struct RealWorld096;
impl RealWorld096 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 097: reduced from a rayon iterator implementation.
struct RealWorld097;
impl RealWorld097 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 098: reduced from a anyhow context implementation.
struct RealWorld098;
impl RealWorld098 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 099: reduced from a axum router implementation.
struct RealWorld099;
impl RealWorld099 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

// Case 100: reduced from a bytes buffer implementation.
struct RealWorld100;
impl RealWorld100 {
    fn classify(&self, value: u8) -> u8 {
        match value {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        }
    }
    fn validate(&self, first: bool, second: bool) -> bool {
        if !first {
            return false;
        }
        second
    }
    fn select(&self, primary: Option<u8>, fallback: u8) -> u8 {
        if let Some(value) = primary {
            value
        } else {
            fallback
        }
    }
    fn finish(&self, values: &[u8]) -> usize {
        let mut count = 0;
        for value in values {
            if *value > 0 {
                count += 1;
            }
        }
        count
    }
}

fn main() {}
