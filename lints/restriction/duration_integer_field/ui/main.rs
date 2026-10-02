struct RetryConfig {
    retry_delay_seconds: u64,
    timeout_ms: usize,
    hours_until_expiry: i32,
    max_retries: u32,
    poll_interval: std::time::Duration,
}

struct PrefixConfig {
    seconds_between_checks: u16,
}

fn main() {}
