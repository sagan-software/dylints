#![allow(dead_code)]

use tokio::runtime::Builder;

fn invalid_thread_counts() {
    let _ = Builder::new_multi_thread().worker_threads(0).build();
    let _ = Builder::new_multi_thread().max_blocking_threads(0).build();
}

fn valid_thread_counts() {
    let _ = Builder::new_multi_thread().worker_threads(2).build();
    let _ = Builder::new_multi_thread().max_blocking_threads(16).build();
    let _ = Builder::new_multi_thread().build();
}

struct OtherBuilder;

impl OtherBuilder {
    fn worker_threads(self, _count: usize) -> Self {
        self
    }
}

fn similarly_named_user_method() {
    let _ = OtherBuilder.worker_threads(0);
}

fn main() {}

const BASE_ZERO_THREAD_COUNT: usize = 0;
const ZERO_THREAD_COUNT: usize = BASE_ZERO_THREAD_COUNT;

fn constant_and_computed_zero_thread_counts_are_reported() {
    let _ = Builder::new_multi_thread()
        .worker_threads(ZERO_THREAD_COUNT)
        .build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(1 / 2)
        .build();
}

macro_rules! macro_zero_thread_count {
    () => {
        ZERO_THREAD_COUNT
    };
}

fn macro_expanded_zero_thread_count_is_reported() {
    let _ = Builder::new_multi_thread()
        .worker_threads(macro_zero_thread_count!())
        .build();
}

fn runtime_thread_count() -> usize {
    0
}

fn unknown_thread_count_is_ignored() {
    let count = runtime_thread_count();
    let _ = Builder::new_multi_thread().worker_threads(count).build();
}

struct AssociatedThreadCount;

impl AssociatedThreadCount {
    const ZERO: usize = 0;
}

fn more_supported_zero_arithmetic_is_reported() {
    let _ = Builder::new_multi_thread().worker_threads(0 * 2).build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(0 % 2)
        .build();
    let _ = Builder::new_multi_thread()
        .worker_threads(AssociatedThreadCount::ZERO)
        .build();
}

static STATIC_ZERO_THREAD_COUNT: usize = 0;

trait ThreadCountValue {
    const VALUE: usize;
}

fn unsupported_thread_counts_are_ignored<T: ThreadCountValue>() {
    let _ = Builder::new_multi_thread()
        .worker_threads(0_usize ^ 0_usize)
        .build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(1_usize << 1_u32)
        .build();
    let _ = Builder::new_multi_thread()
        .worker_threads(0_u8 as usize)
        .build();
    let _ = Builder::new_multi_thread()
        .worker_threads(if true { 0 } else { 1 })
        .build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(STATIC_ZERO_THREAD_COUNT)
        .build();
    let _ = Builder::new_multi_thread()
        .worker_threads(usize::MIN)
        .build();
    let _ = Builder::new_multi_thread().worker_threads(T::VALUE).build();
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

fn a_deep_thread_count_is_ignored() {
    let _ = Builder::new_multi_thread()
        .worker_threads(DEEP_ZERO_17)
        .build();
}

#[cfg(target_pointer_width = "64")]
#[allow(arithmetic_overflow, unconditional_panic)]
fn overflowing_or_panicking_thread_arithmetic_is_ignored() {
    let _ = Builder::new_multi_thread()
        .worker_threads(18_446_744_073_709_551_615_usize + 1)
        .build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(0_usize - 1)
        .build();
    let _ = Builder::new_multi_thread()
        .worker_threads(1_usize / 0)
        .build();
    let _ = Builder::new_multi_thread()
        .max_blocking_threads(1_usize % 0)
        .build();
}
