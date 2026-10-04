struct RetryConfig {
    retry_delay_seconds: u64,
    timeout_ms: usize,
    hours_until_expiry: i32,
    max_retries: u32,
    poll_interval: std::time::Duration,
}

struct CalendarTime {
    hour: u8,
    minute: u8,
    second: u8,
    day: u8,
    week: u8,
    min_connections: u32,
}

struct PrefixConfig {
    seconds_between_checks: u16,
}

fn main() {}

struct OptionalRetryConfig {
    timeout_ms: Option<u64>,
    typed_retry_delay: Option<std::time::Duration>,
}
