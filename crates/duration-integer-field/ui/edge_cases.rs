#![allow(dead_code)]

type Milliseconds = u64;
type SignedSeconds = i32;

struct BusinessDays(u16);
struct RetryCount(u16);

struct Settings {
    connect_timeout_millis: u64,
    cache_ttl_seconds: i32,
    delay_ms: usize,
    alias_timeout_ms: Milliseconds,
    alias_window_seconds: SignedSeconds,
    retry_count: u16,
    typed_timeout: std::time::Duration,
    business_days: BusinessDays,
    retry_days: RetryCount,
}

fn main() {}
