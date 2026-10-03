#![allow(dead_code)]

fn invalid_retry_budgets() {
    let negative = reqwest::retry::for_host("example.com").max_extra_load(-0.1);
    let _builder = reqwest::Client::builder().retry(negative);

    let too_large = reqwest::retry::for_host("example.com").max_extra_load(1000.1);
    let _builder = reqwest::Client::builder().retry(too_large);
}

fn valid_retry_budgets() {
    let _zero = reqwest::retry::for_host("example.com").max_extra_load(0.0);
    let _bounded = reqwest::retry::for_host("example.com").max_extra_load(0.2);
    let _upper_bound = reqwest::retry::for_host("example.com").max_extra_load(1000.0);
}

fn main() {}

const BAD: f32 = -1.0;
const TOO_LARGE: f32 = 1001.0;
const NEGATIVE_BASE: f32 = 0.0 - 1.0;
const BAD_ARITHMETIC: f32 = NEGATIVE_BASE;
const NAN: f32 = f32::NAN;
const INF: f32 = f32::INFINITY;
const NEG_INF: f32 = f32::NEG_INFINITY;
const BAD_MULTIPLICATION: f32 = -0.5 * 2.0;
const BAD_DIVISION: f32 = -2.0 / 2.0;
const BAD_REMAINDER: f32 = -1.0 % 2.0;
const JUST_ABOVE_UPPER_BOUND_ROUNDS_TO_BOUND: f32 = 1000.00001_f32;
const JUST_ABOVE_UPPER_BOUND_REMAINS_INVALID: f32 = 1000.0001_f32;

struct CustomFloatOperand;

impl std::ops::Add for CustomFloatOperand {
    type Output = f32;

    fn add(self, _right: Self) -> Self::Output {
        -1.0
    }
}

impl std::ops::Neg for CustomFloatOperand {
    type Output = f32;

    fn neg(self) -> Self::Output {
        -1.0
    }
}

fn invalid_constant_retry_budgets() {
    let _ = reqwest::retry::for_host("example.com").max_extra_load(BAD);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(TOO_LARGE);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(BAD_ARITHMETIC);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(NAN);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(INF);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(NEG_INF);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(BAD_MULTIPLICATION);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(BAD_DIVISION);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(BAD_REMAINDER);
    let _ = reqwest::retry::for_host("example.com")
        .max_extra_load(JUST_ABOVE_UPPER_BOUND_REMAINS_INVALID);
}

macro_rules! macro_invalid_retry_budget {
    () => {
        BAD
    };
}

fn macro_expanded_invalid_retry_budget_is_reported() {
    let _ = reqwest::retry::for_host("example.com").max_extra_load(macro_invalid_retry_budget!());
}

fn overloaded_float_operators_are_ignored() {
    let _ = reqwest::retry::for_host("example.com")
        .max_extra_load(CustomFloatOperand + CustomFloatOperand);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(-CustomFloatOperand);
}

fn valid_constant_retry_budgets() {
    const ZERO_BUDGET: f32 = 0.0;
    const UPPER_BOUND: f32 = 1000.0;
    let _ = reqwest::retry::for_host("example.com").max_extra_load(ZERO_BUDGET);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(UPPER_BOUND);
    let _ = reqwest::retry::for_host("example.com")
        .max_extra_load(JUST_ABOVE_UPPER_BOUND_ROUNDS_TO_BOUND);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(999.0 + 1.0);
}

fn runtime_retry_budget() -> f32 {
    1.0
}

fn unknown_retry_budget_is_ignored() {
    let budget = runtime_retry_budget();
    let _ = reqwest::retry::for_host("example.com").max_extra_load(budget);
}

static STATIC_BAD_BUDGET: f32 = -1.0;

trait RetryBudgetValue {
    const VALUE: f32;
}

fn unsupported_retry_budget_values_are_ignored<T: RetryBudgetValue>() {
    let _ = reqwest::retry::for_host("example.com").max_extra_load(STATIC_BAD_BUDGET);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(f32::MAX);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(T::VALUE);
    let _ = reqwest::retry::for_host("example.com").max_extra_load((-1.0_f64) as f32);
    let _ = reqwest::retry::for_host("example.com").max_extra_load(if true { -1.0 } else { 0.0 });
}

const DEEP_BAD_0: f32 = -1.0;
const DEEP_BAD_1: f32 = DEEP_BAD_0;
const DEEP_BAD_2: f32 = DEEP_BAD_1;
const DEEP_BAD_3: f32 = DEEP_BAD_2;
const DEEP_BAD_4: f32 = DEEP_BAD_3;
const DEEP_BAD_5: f32 = DEEP_BAD_4;
const DEEP_BAD_6: f32 = DEEP_BAD_5;
const DEEP_BAD_7: f32 = DEEP_BAD_6;
const DEEP_BAD_8: f32 = DEEP_BAD_7;
const DEEP_BAD_9: f32 = DEEP_BAD_8;
const DEEP_BAD_10: f32 = DEEP_BAD_9;
const DEEP_BAD_11: f32 = DEEP_BAD_10;
const DEEP_BAD_12: f32 = DEEP_BAD_11;
const DEEP_BAD_13: f32 = DEEP_BAD_12;
const DEEP_BAD_14: f32 = DEEP_BAD_13;
const DEEP_BAD_15: f32 = DEEP_BAD_14;
const DEEP_BAD_16: f32 = DEEP_BAD_15;
const DEEP_BAD_17: f32 = DEEP_BAD_16;

fn a_deep_retry_budget_is_ignored() {
    let _ = reqwest::retry::for_host("example.com").max_extra_load(DEEP_BAD_17);
}
