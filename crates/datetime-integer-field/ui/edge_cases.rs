#![allow(dead_code)]

struct DateTimeUtc;

struct Audit {
    updated_at_epoch_ms: i128,
    deleted_timestamp: u32,
    expires_at_ts: i64,
    retry_count: usize,
    typed_updated_at: DateTimeUtc,
}

fn main() {}
