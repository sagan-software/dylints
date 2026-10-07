use sqlx::PoolOptions;

fn main() {
    let _ = PoolOptions.max_connections(0);
    let _ = PoolOptions.max_connections(1);
}

const BASE_ZERO_MAX_CONNECTIONS: u32 = 1 - 1;
const ZERO_MAX_CONNECTIONS: u32 = BASE_ZERO_MAX_CONNECTIONS;

fn constant_and_computed_zero_limits_are_reported() {
    let _ = PoolOptions.max_connections(ZERO_MAX_CONNECTIONS);
    let _ = PoolOptions.max_connections(1 / 2);
}

macro_rules! macro_zero_connection_limit {
    () => {
        ZERO_MAX_CONNECTIONS
    };
}

fn macro_expanded_zero_connection_limit_is_reported() {
    let _ = PoolOptions.max_connections(macro_zero_connection_limit!());
}

fn runtime_max_connections() -> u32 {
    0
}

fn unknown_limit_is_ignored() {
    let limit = runtime_max_connections();
    let _ = PoolOptions.max_connections(limit);
}

fn positive_arithmetic_limit_is_valid() {
    let _ = PoolOptions.max_connections(1 + 1);
}

struct AssociatedConnectionLimit;

impl AssociatedConnectionLimit {
    const ZERO: u32 = 0;
}

fn more_supported_zero_arithmetic_is_reported() {
    let _ = PoolOptions.max_connections(0_u32 * 2);
    let _ = PoolOptions.max_connections(0_u32 % 2);
    let _ = PoolOptions.max_connections(AssociatedConnectionLimit::ZERO);
}

static STATIC_ZERO_CONNECTION_LIMIT: u32 = 0;

trait ConnectionLimitValue {
    const VALUE: u32;
}

fn unsupported_connection_limits_are_ignored<T: ConnectionLimitValue>() {
    let _ = PoolOptions.max_connections(0_u32 ^ 0_u32);
    let _ = PoolOptions.max_connections(1_u32 << 1_usize);
    let _ = PoolOptions.max_connections(0_u8 as u32);
    let _ = PoolOptions.max_connections(if true { 0 } else { 1 });
    let _ = PoolOptions.max_connections(STATIC_ZERO_CONNECTION_LIMIT);
    let _ = PoolOptions.max_connections(u32::MIN);
    let _ = PoolOptions.max_connections(T::VALUE);
}

const DEEP_ZERO_0: u32 = 0;
const DEEP_ZERO_1: u32 = DEEP_ZERO_0;
const DEEP_ZERO_2: u32 = DEEP_ZERO_1;
const DEEP_ZERO_3: u32 = DEEP_ZERO_2;
const DEEP_ZERO_4: u32 = DEEP_ZERO_3;
const DEEP_ZERO_5: u32 = DEEP_ZERO_4;
const DEEP_ZERO_6: u32 = DEEP_ZERO_5;
const DEEP_ZERO_7: u32 = DEEP_ZERO_6;
const DEEP_ZERO_8: u32 = DEEP_ZERO_7;
const DEEP_ZERO_9: u32 = DEEP_ZERO_8;
const DEEP_ZERO_10: u32 = DEEP_ZERO_9;
const DEEP_ZERO_11: u32 = DEEP_ZERO_10;
const DEEP_ZERO_12: u32 = DEEP_ZERO_11;
const DEEP_ZERO_13: u32 = DEEP_ZERO_12;
const DEEP_ZERO_14: u32 = DEEP_ZERO_13;
const DEEP_ZERO_15: u32 = DEEP_ZERO_14;
const DEEP_ZERO_16: u32 = DEEP_ZERO_15;
const DEEP_ZERO_17: u32 = DEEP_ZERO_16;

fn a_deep_connection_limit_is_ignored() {
    let _ = PoolOptions.max_connections(DEEP_ZERO_17);
}

#[allow(arithmetic_overflow, unconditional_panic)]
fn overflowing_or_panicking_connection_arithmetic_is_ignored() {
    let _ = PoolOptions.max_connections(4_294_967_295_u32 + 1);
    let _ = PoolOptions.max_connections(0_u32 - 1);
    let _ = PoolOptions.max_connections(1_u32 / 0);
    let _ = PoolOptions.max_connections(1_u32 % 0);
}
