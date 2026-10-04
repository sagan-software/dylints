#![allow(dead_code, non_camel_case_types)]

use clock::SignedEpoch as ImportedEpoch;
use std::primitive::usize as MachineTimestamp;

struct DateTimeUtc;
struct TimestampWrapper(i64);

type EpochMillis = i64;
type UnixSeconds = std::primitive::u64;

mod clock {
    pub struct u64;
    pub type SignedEpoch = std::primitive::i128;
}

struct Session {
    created_at_ts: i64,
    expires_epoch: u64,
    timestamp_ms: usize,
    aliased_created_at_ts: EpochMillis,
    aliased_expires_epoch: UnixSeconds,
    imported_unix_time: ImportedEpoch,
    unix_mode: u32,
    small_date: u8,
    tiny_ts: i16,
    machine_timestamp: MachineTimestamp,
    retry_count: u64,
    created_at: DateTimeUtc,
    wrapped_created_at_ts: TimestampWrapper,
    lookalike_expires_epoch: clock::u64,
}

fn main() {}

struct OptionalSession {
    created_at_ts: Option<i64>,
    typed_created_at: Option<DateTimeUtc>,
}
