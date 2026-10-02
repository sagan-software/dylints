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

fn main() {}
