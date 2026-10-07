#![allow(dead_code)]
#![deny(unfulfilled_lint_expectations)]

// Rustc normalizes the decomposed accent in this identifier before resolution.
#[expect(one_use_private_helper)]
fn café_total(amount: u64) -> u64 {
    amount + 1
}

fn main() {
    let _total = café_total(1);
}
