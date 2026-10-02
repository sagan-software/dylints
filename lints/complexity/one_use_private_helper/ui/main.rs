#![allow(dead_code, unused_variables)]

struct User {
    id: u64,
}

fn inline_total(amount: u64) -> u64 {
    amount.saturating_add(1)
}

fn trigger(amount: u64) -> u64 {
    inline_total(amount)
}

fn reused_helper(amount: u64) -> u64 {
    amount.saturating_mul(2)
}

fn keep_multiple_uses(amount: u64) -> u64 {
    reused_helper(amount) + reused_helper(1)
}

fn validate_user(user: &User) -> bool {
    user.id > 0
}

fn keep_validation(user: &User) -> bool {
    validate_user(user)
}

fn fixture_user() -> User {
    User { id: 1 }
}

fn keep_fixture() -> u64 {
    fixture_user().id
}

fn can_release_funds(user: &User) -> bool {
    user.id > 10
}

fn keep_domain_rule(user: &User) -> bool {
    can_release_funds(user)
}

#[cold]
fn attributed_helper(amount: u64) -> u64 {
    amount + 1
}

fn keep_attributed(amount: u64) -> u64 {
    attributed_helper(amount)
}

pub fn public_helper(amount: u64) -> u64 {
    amount + 1
}

fn keep_public(amount: u64) -> u64 {
    public_helper(amount)
}

fn main() {}
