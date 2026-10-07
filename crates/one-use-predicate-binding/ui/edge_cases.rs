// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code, unused_variables)]

fn consume<T>(_value: T) {}

fn single_use_comparison(count: usize, limit: usize) {
    let is_over_limit = count > limit;
    if is_over_limit {
        consume(count);
    }
}

fn meaningful_domain_name(user_count: usize) {
    let is_capacity_exhausted = user_count > 100;
    if is_capacity_exhausted {
        consume(user_count);
    }
}

fn reused_predicate(items: &[u8]) {
    let is_empty = items.is_empty();
    if is_empty {
        consume(items.len());
    }
    consume(is_empty);
}

fn keep_closure_use(items: &[u8]) {
    let is_empty = items.is_empty();
    if is_empty {
        consume(items.len());
    }
    let check = || is_empty;
    consume(check());
}

#[derive(PartialEq)]
struct Point {
    x: i32,
}

fn struct_literal_init(point: &Point) {
    let is_valid = *point == Point { x: 0 };
    if is_valid {
        consume(point.x);
    }
}

fn negated_comparison(count: usize) {
    let has_items = count > 0;
    if !has_items {
        consume(count);
    }
}

fn negated_call(items: &[u8]) {
    let is_empty = items.is_empty();
    if !is_empty {
        consume(items.len());
    }
}

fn comment_between(items: &[u8]) {
    let is_empty = items.is_empty();
    // Explain the branch.
    if is_empty {
        consume(items.len());
    }
}

fn tail_branch(items: &[u8]) -> usize {
    let is_empty = items.is_empty();
    if is_empty { 0 } else { items.len() }
}

macro_rules! empty_check {
    ($items:expr) => {
        $items.is_empty()
    };
}

fn macro_initializer(items: &[u8]) {
    let is_empty = empty_check!(items);
    if is_empty {
        consume(items.len());
    }
}

macro_rules! declare_and_branch {
    ($items:expr) => {
        let is_empty = $items.is_empty();
        if is_empty {
            consume($items.len());
        }
    };
}

fn macro_statement(items: &[u8]) {
    declare_and_branch!(items);
}

fn let_else_binding(items: &[u8]) {
    let is_empty = items.is_empty() else {
        return;
    };
    if is_empty {
        consume(items.len());
    }
}

fn followed_by_let(items: &[u8]) {
    let is_empty = items.is_empty();
    let count = items.len();
    consume((is_empty, count));
}

fn ends_block(items: &[u8]) {
    let is_empty = items.is_empty();
}

fn keep_compound_condition(items: &[u8], flag: bool) {
    let is_empty = items.is_empty();
    if is_empty && flag {
        consume(items.len());
    }
}

fn keep_bitwise_initializer(left: bool, right: bool) {
    let is_valid = left & right;
    if is_valid {
        consume(left);
    }
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

struct Checker;

impl Checker {
    fn is_good(value: u8) -> bool {
        value > 1
    }
}

fn predicate_calls(text: &str, value: u8) {
    let is_empty = is_blank(text);
    if is_empty {
        consume(text);
    }
    let is_valid = Checker::is_good(value);
    if is_valid {
        consume(value);
    }
}

fn keep_closure_call(make: fn() -> fn() -> bool) {
    let is_ready = make()();
    if is_ready {
        consume(());
    }
}

fn keep_unprefixed_name(items: &[u8]) {
    let ready_flag = items.is_empty();
    if ready_flag {
        consume(items.len());
    }
}

fn main() {}
