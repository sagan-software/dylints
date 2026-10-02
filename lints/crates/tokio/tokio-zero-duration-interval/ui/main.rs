#![allow(dead_code)]

use std::time::Duration;
use tokio::time::{Instant, interval, interval_at};

fn invalid_periods() {
    let _ = interval(Duration::ZERO);
    let _ = interval(Duration::from_secs(0));
    let _ = interval(Duration::new(0, 0));
    let _ = interval_at(Instant::now(), Duration::from_millis(0));
}

fn valid_periods() {
    let _ = interval(Duration::from_nanos(1));
    let _ = interval_at(Instant::now(), Duration::from_secs(1));
}

fn other_interval(_period: Duration) {}

fn similarly_named_user_function() {
    other_interval(Duration::ZERO);
}

fn main() {}
