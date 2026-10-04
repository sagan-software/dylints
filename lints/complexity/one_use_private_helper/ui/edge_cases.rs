#![allow(dead_code)]

struct Order {
    amount: u64,
}

fn inline_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(5)
}

fn checkout(order: &Order) -> u64 {
    inline_discount(order)
}

pub fn public_helper(order: &Order) -> u64 {
    order.amount.saturating_add(1)
}

fn multi_statement_helper(order: &Order) -> u64 {
    let adjusted = order.amount.saturating_add(10);
    adjusted.saturating_sub(1)
}

fn uses_safe_helpers(order: &Order) -> u64 {
    public_helper(order) + multi_statement_helper(order)
}

fn main() {}

/// Applies the standard discount.
fn documented_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(1)
}

#[inline]
fn inlined_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(6)
}

#[allow(clippy::all)]
#[must_use]
fn attributed_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(2)
}

fn qualified_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(3)
}

fn closure_discount(order: &Order) -> u64 {
    order.amount.saturating_sub(4)
}

fn calls_documented(order: &Order) -> u64 {
    documented_discount(order)
        + attributed_discount(order)
        + inlined_discount(order)
        + self::qualified_discount(order)
        + (|| closure_discount(order))()
}

fn value_and_call(order: &Order) -> u64 {
    order.amount.saturating_add(2)
}

fn uses_as_value(orders: &[Order]) -> Vec<u64> {
    let first = value_and_call(&orders[0]);
    orders.iter().map(value_and_call).chain([first]).collect()
}

fn recursive_total(order: &Order) -> u64 {
    recursive_total(order)
}

fn mentioned_in_comment(order: &Order) -> u64 {
    order.amount
}

// mentioned_in_comment is still linted when its name occurs in a comment.
fn calls_mentioned(order: &Order) -> u64 {
    mentioned_in_comment(order)
}

fn macro_called(order: &Order) -> u64 {
    order.amount.saturating_add(5)
}

macro_rules! call_macro_called {
    ($order:expr) => {
        macro_called($order)
    };
}

fn uses_macro(order: &Order) -> u64 {
    call_macro_called!(order)
}

fn generic_amount<T: Into<u64>>(value: T) -> u64 {
    value.into()
}

unsafe fn unsafe_amount(order: &Order) -> u64 {
    order.amount
}

pub(crate) fn crate_amount(order: &Order) -> u64 {
    order.amount
}

fn uses_restricted(order: &Order) -> u64 {
    generic_amount(order.amount) + unsafe { unsafe_amount(order) } + crate_amount(order)
}

fn cfg_test_amount(order: &Order) -> u64 {
    order.amount.saturating_mul(3)
}

fn uses_cfg_test(order: &Order) -> u64 {
    cfg_test_amount(order)
}

#[cfg(test)]
mod tests {
    fn again(order: &super::Order) -> u64 {
        super::cfg_test_amount(order)
    }
}

const fn const_amount(amount: u64) -> u64 {
    amount + 1
}

const LIMIT: u64 = const_amount(1);

fn statement_only(order: &Order) {
    let _ = order.amount;
}

fn uses_statement_only(order: &Order) {
    statement_only(order);
}

// Literal contents must not count as another call to inline_discount.
const HELPER_LABELS: (&str, &str, &[u8], &[u8]) = (
    "inline_discount",
    r#"inline_discount"#,
    b"inline_discount",
    br#"inline_discount"#,
);
/* inline_discount /* inline_discount */ inline_discount */
