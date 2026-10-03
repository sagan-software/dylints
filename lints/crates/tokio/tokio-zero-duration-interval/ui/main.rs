#![allow(dead_code)]

use std::time::Duration;
use tokio::time::{Instant, interval, interval_at};

const ZERO_PERIOD: Duration = Duration::ZERO;
const POSITIVE_PERIOD: Duration = Duration::from_nanos(1);
const SUBNANOSECONDS: f64 = 1e-20;
const POSITIVE_SECONDS: u64 = 1;
const ZERO_LINK_00: Duration = Duration::ZERO;
const ZERO_LINK_01: Duration = ZERO_LINK_00;
const ZERO_LINK_02: Duration = ZERO_LINK_01;
const ZERO_LINK_03: Duration = ZERO_LINK_02;
const ZERO_LINK_04: Duration = ZERO_LINK_03;
const ZERO_LINK_05: Duration = ZERO_LINK_04;
const ZERO_LINK_06: Duration = ZERO_LINK_05;
const ZERO_LINK_07: Duration = ZERO_LINK_06;
const ZERO_LINK_08: Duration = ZERO_LINK_07;
const ZERO_LINK_09: Duration = ZERO_LINK_08;
const ZERO_LINK_10: Duration = ZERO_LINK_09;
const ZERO_LINK_11: Duration = ZERO_LINK_10;
const ZERO_LINK_12: Duration = ZERO_LINK_11;
const ZERO_LINK_13: Duration = ZERO_LINK_12;
const ZERO_LINK_14: Duration = ZERO_LINK_13;
const ZERO_LINK_15: Duration = ZERO_LINK_14;
const ZERO_SECONDS: u64 = 0;
static STATIC_ZERO_PERIOD: Duration = Duration::ZERO;

struct Period;

impl Period {
    const ZERO: Duration = Duration::ZERO;
}

fn invalid_periods() {
    let _ = interval(Duration::ZERO);
    let _ = interval(Duration::from_secs(0));
    let _ = interval(Duration::new(0, 0));
    let _ = interval(Duration::new(0, 1_000_000_000));
    let _ = interval(Duration::from_micros(0));
    let _ = interval(Duration::from_nanos(0));
    let _ = interval(Duration::default());
    let _ = interval(std::default::Default::default());
    let _ = interval(core::time::Duration::default());
    let _ = interval(Duration::from_secs_f64(0.0));
    let _ = interval(Duration::from_secs_f32(0.0));
    let _ = interval(Duration::from_secs_f64(1e-20));
    let _ = interval(Duration::from_secs_f32(1e-20));
    let _ = interval(Duration::from_secs_f64(SUBNANOSECONDS));
    let _ = interval(ZERO_PERIOD);
    let _ = interval(ZERO_LINK_14);
    let _ = interval(Duration::ZERO + Duration::ZERO);
    let _ = interval(Duration::from_nanos(1) - Duration::from_nanos(1));
    let _ = interval(Duration::ZERO * 5);
    let _ = interval(5 * Duration::ZERO);
    let _ = interval(Duration::ZERO / 5);
    let _ = interval(Duration::from_secs(ZERO_SECONDS));
    let _ = interval(Duration::from_secs(POSITIVE_SECONDS - 1));
    let _ = interval(Duration::from_nanos(0) / 1);
    let _ = interval({ Duration::ZERO });
    let _ = interval(Duration::from_secs(ZERO_SECONDS + ZERO_SECONDS));
    let _ = interval(Duration::from_secs(ZERO_SECONDS * 2));
    let _ = interval(Duration::from_secs(ZERO_SECONDS / 2));
    let _ = interval(Duration::from_secs({ ZERO_SECONDS }));
    let _ = interval(Duration::from_secs(
        0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64,
    ));
    let _ = interval(Duration::from_secs_f64({ 0.0 }));
    let _ = interval(Period::ZERO);
    let _ = interval(STATIC_ZERO_PERIOD);
    let _ = interval_at(Instant::now(), Duration::from_millis(0));
}

fn valid_periods() {
    let _ = interval(Duration::from_nanos(1));
    let _ = interval_at(Instant::now(), Duration::from_secs(1));
    let _ = interval(POSITIVE_PERIOD);
    let _ = interval(Duration::from_secs(POSITIVE_SECONDS));
    let _ = interval(Duration::from_secs_f64(1.0));
    let _ = interval(Duration::from_secs_f32(1.0));
    let _ = interval(ZERO_LINK_15);
    let _ = interval(Duration::from_nanos(2) - Duration::from_nanos(1));
    let _ = interval(Duration::from_nanos(1) + Duration::from_nanos(1));
    let _ = interval(Duration::new(18_446_744_073_709_551_615, 1_000_000_000));
    let _ =
        interval(Duration::new(18_446_744_073_709_551_615, 999_999_999) + Duration::from_nanos(1));
    let _ = interval(Duration::new(18_446_744_073_709_551_615, 0) * 2);
    let _ = interval(Duration::ZERO - Duration::from_nanos(1));
    let _ = interval(Duration::ZERO / 0);
}

fn failed_float_constructors_do_not_count_as_zero() {
    let _ = interval(Duration::from_secs_f64(-1.0));
    let _ = interval(Duration::from_secs_f32(f32::NAN));
}

fn negative_zero_is_a_zero_duration() {
    let _ = interval(Duration::from_secs_f64(-0.0));
}

fn other_interval(_period: Duration) {}

fn an_unknown_period(period: Duration) {
    let _ = interval(period);
}

fn similarly_named_user_function() {
    other_interval(Duration::ZERO);
}

mod shadowed_interval {
    use std::time::Duration;

    fn interval(_period: Duration) {}

    fn same_name_non_tokio_interval() {
        interval(Duration::ZERO);
    }
}

mod generated_interval {
    macro_rules! call {
        ($period:expr) => {
            ::tokio::time::interval($period)
        };
    }

    fn macro_argument_origin_is_preserved() {
        call!(std::time::Duration::ZERO);
    }
}

fn a_shadowed_duration_name_does_not_change_resolution() {
    #[derive(Default)]
    struct Duration;

    let _ = interval(std::time::Duration::default());
    let _ = Duration::default();
}

fn main() {}

fn zero_seconds() -> u64 {
    0
}

fn zero_float_seconds() -> f64 {
    0.0
}

fn unsupported_duration_values(flag: bool, seconds: u64, float_seconds: f64) {
    let make_duration = Duration::from_secs;
    let _ = interval(make_duration(0));
    let _ = interval((|| Duration::ZERO)());
    let _ = interval(if flag {
        Duration::ZERO
    } else {
        Duration::from_nanos(1)
    });
    let _ = interval({
        let period = Duration::ZERO;
        period
    });
    let _ = interval(Duration::ZERO.saturating_add(Duration::ZERO));
    let _ = interval(Duration::saturating_add(Duration::ZERO, Duration::ZERO));
    let _ = interval(Duration::from_secs(seconds));
    let _ = interval(Duration::from_secs_f64(float_seconds));
    let _ = interval(Duration::from_secs(0_u64 & 1_u64));
    let _ = interval(Duration::from_secs(zero_seconds()));
    let _ = interval(Duration::from_secs(
        0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64
            + 0_u64,
    ));
    let _ = interval(Duration::from_secs(CustomSeconds + 0_u16));
    let _ = interval(Duration::from_secs_f64(zero_float_seconds()));
    let _ = interval(Duration::from_secs(0_u64 % 2_u64));
    let _ = interval(Duration::from_secs_f64(SUBNANOSECONDS_LINK_14));
    let _ = interval(Duration::from_secs_f64(SUBNANOSECONDS_LINK_15));
    let _ = interval(Duration::from_secs_f32(-0.0));
    let _ = interval(Duration::from_secs_f64(f64::INFINITY));
    let _ = interval(<TraitDefaultPeriod as DefaultPeriod>::PERIOD);
    let _ = interval(UnmodeledDuration + Duration::ZERO);
    let _ = interval(UnmodeledDuration - Duration::ZERO);
    let _ = interval(UnmodeledDuration * UnmodeledDuration);
    let _ = interval(UnmodeledDuration / UnmodeledDuration);
    let _ = interval(UnmodeledDuration % UnmodeledDuration);
}

trait DefaultPeriod {
    const PERIOD: Duration = Duration::ZERO;
}

struct TraitDefaultPeriod;

impl DefaultPeriod for TraitDefaultPeriod {}

struct UnmodeledDuration;

struct CustomSeconds;

impl std::ops::Add<u16> for CustomSeconds {
    type Output = u64;

    fn add(self, _right: u16) -> Self::Output {
        0
    }
}

impl std::ops::Add<Duration> for UnmodeledDuration {
    type Output = Duration;

    fn add(self, right: Duration) -> Self::Output {
        right
    }
}

impl std::ops::Sub<Duration> for UnmodeledDuration {
    type Output = Duration;

    fn sub(self, right: Duration) -> Self::Output {
        right
    }
}

impl std::ops::Mul for UnmodeledDuration {
    type Output = Duration;

    fn mul(self, _right: Self) -> Self::Output {
        Duration::ZERO
    }
}

impl std::ops::Div for UnmodeledDuration {
    type Output = Duration;

    fn div(self, _right: Self) -> Self::Output {
        Duration::ZERO
    }
}

impl std::ops::Rem for UnmodeledDuration {
    type Output = Duration;

    fn rem(self, _right: Self) -> Self::Output {
        Duration::ZERO
    }
}

const SUBNANOSECONDS_LINK_00: f64 = SUBNANOSECONDS;
const SUBNANOSECONDS_LINK_01: f64 = SUBNANOSECONDS_LINK_00;
const SUBNANOSECONDS_LINK_02: f64 = SUBNANOSECONDS_LINK_01;
const SUBNANOSECONDS_LINK_03: f64 = SUBNANOSECONDS_LINK_02;
const SUBNANOSECONDS_LINK_04: f64 = SUBNANOSECONDS_LINK_03;
const SUBNANOSECONDS_LINK_05: f64 = SUBNANOSECONDS_LINK_04;
const SUBNANOSECONDS_LINK_06: f64 = SUBNANOSECONDS_LINK_05;
const SUBNANOSECONDS_LINK_07: f64 = SUBNANOSECONDS_LINK_06;
const SUBNANOSECONDS_LINK_08: f64 = SUBNANOSECONDS_LINK_07;
const SUBNANOSECONDS_LINK_09: f64 = SUBNANOSECONDS_LINK_08;
const SUBNANOSECONDS_LINK_10: f64 = SUBNANOSECONDS_LINK_09;
const SUBNANOSECONDS_LINK_11: f64 = SUBNANOSECONDS_LINK_10;
const SUBNANOSECONDS_LINK_12: f64 = SUBNANOSECONDS_LINK_11;
const SUBNANOSECONDS_LINK_13: f64 = SUBNANOSECONDS_LINK_12;
const SUBNANOSECONDS_LINK_14: f64 = SUBNANOSECONDS_LINK_13;
const SUBNANOSECONDS_LINK_15: f64 = SUBNANOSECONDS_LINK_14;
