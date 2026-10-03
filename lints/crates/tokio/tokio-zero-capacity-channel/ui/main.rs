#![allow(dead_code)]

use tokio::sync::{broadcast, mpsc};

fn invalid_capacities() {
    let _ = mpsc::channel::<u8>(0);
    let _ = broadcast::channel::<u8>(0);
}

fn valid_capacities() {
    let _ = mpsc::channel::<u8>(1);
    let _ = broadcast::channel::<u8>(16);
}

mod other {
    pub fn channel(_capacity: usize) {}
}

fn similarly_named_user_function() {
    other::channel(0);
}

fn main() {}

const BASE_ZERO_CAPACITY: usize = 1 - 1;
const ZERO_CAPACITY: usize = BASE_ZERO_CAPACITY;

fn constant_and_computed_zero_capacities_are_reported() {
    let _ = mpsc::channel::<u8>(ZERO_CAPACITY);
    let _ = broadcast::channel::<u8>(2 - 2);
}

macro_rules! macro_zero_capacity {
    () => {
        ZERO_CAPACITY
    };
}

fn macro_expanded_zero_capacity_is_reported() {
    let _ = mpsc::channel::<u8>(macro_zero_capacity!());
}

fn positive_arithmetic_capacity_is_valid() {
    let _ = mpsc::channel::<u8>(1 + 1);
}

fn runtime_capacity() -> usize {
    0
}

fn unknown_capacity_is_ignored() {
    let capacity = runtime_capacity();
    let _ = mpsc::channel::<u8>(capacity);
}

struct AssociatedCapacity;

impl AssociatedCapacity {
    const ZERO: usize = 0;
}

fn more_supported_zero_arithmetic_is_reported() {
    let _ = mpsc::channel::<u8>(0 * 2);
    let _ = broadcast::channel::<u8>(0 % 2);
    let _ = mpsc::channel::<u8>(AssociatedCapacity::ZERO);
}

static STATIC_ZERO_CAPACITY: usize = 0;

trait CapacityValue {
    const VALUE: usize;
}

fn unsupported_capacity_values_are_ignored<T: CapacityValue>() {
    let _ = mpsc::channel::<u8>(0_usize ^ 0_usize);
    let _ = broadcast::channel::<u8>(1_usize << 1_u32);
    let _ = mpsc::channel::<u8>(0_u8 as usize);
    let _ = broadcast::channel::<u8>(if true { 0 } else { 1 });
    let _ = mpsc::channel::<u8>(STATIC_ZERO_CAPACITY);
    let _ = broadcast::channel::<u8>(usize::MIN);
    let _ = mpsc::channel::<u8>(T::VALUE);
}

const DEEP_ZERO_0: usize = 0;
const DEEP_ZERO_1: usize = DEEP_ZERO_0;
const DEEP_ZERO_2: usize = DEEP_ZERO_1;
const DEEP_ZERO_3: usize = DEEP_ZERO_2;
const DEEP_ZERO_4: usize = DEEP_ZERO_3;
const DEEP_ZERO_5: usize = DEEP_ZERO_4;
const DEEP_ZERO_6: usize = DEEP_ZERO_5;
const DEEP_ZERO_7: usize = DEEP_ZERO_6;
const DEEP_ZERO_8: usize = DEEP_ZERO_7;
const DEEP_ZERO_9: usize = DEEP_ZERO_8;
const DEEP_ZERO_10: usize = DEEP_ZERO_9;
const DEEP_ZERO_11: usize = DEEP_ZERO_10;
const DEEP_ZERO_12: usize = DEEP_ZERO_11;
const DEEP_ZERO_13: usize = DEEP_ZERO_12;
const DEEP_ZERO_14: usize = DEEP_ZERO_13;
const DEEP_ZERO_15: usize = DEEP_ZERO_14;
const DEEP_ZERO_16: usize = DEEP_ZERO_15;
const DEEP_ZERO_17: usize = DEEP_ZERO_16;

fn a_deep_constant_is_ignored() {
    let _ = mpsc::channel::<u8>(DEEP_ZERO_17);
}

#[cfg(target_pointer_width = "64")]
#[allow(arithmetic_overflow, unconditional_panic)]
fn overflowing_or_panicking_capacity_arithmetic_is_ignored() {
    let _ = mpsc::channel::<u8>(18_446_744_073_709_551_615_usize + 1);
    let _ = broadcast::channel::<u8>(0_usize - 1);
    let _ = mpsc::channel::<u8>(1_usize / 0);
    let _ = broadcast::channel::<u8>(1_usize % 0);
}
