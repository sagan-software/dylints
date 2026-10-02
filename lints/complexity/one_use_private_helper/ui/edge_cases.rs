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
