use std::vec::Vec as Items;

fn drain_while(mut items: Items<u16>) {
    while !items.is_empty() {
        let _removed = items.remove(0);
    }
}

fn drain_unit_elements(mut items: Vec<()>) {
    while !items.is_empty() {
        items.remove(0);
    }
}

struct Marker;

fn drain_named_zero_sized_elements(mut items: Vec<Marker>) {
    while !items.is_empty() {
        items.remove(0);
    }
}

fn drain_generic_elements<T>(mut items: Vec<T>) {
    while !items.is_empty() {
        let _removed = items.remove(0);
    }
}

fn drain_for(mut items: Vec<u16>) {
    for _ in 0..items.len() {
        let _removed = items.remove(0);
    }
    for _ in 0..=1 {
        let _removed = items.remove(0);
    }
    for _ in [(), ()].iter() {
        let _removed = items.remove(0);
    }
}

fn singleton_vector_may_warn() {
    let mut items = vec![1];
    while !items.is_empty() {
        items.remove(0);
    }
}

fn fixed_singleton(mut items: Vec<u16>) {
    for _ in 0..1 {
        let _removed = items.remove(0);
    }
    for _ in [()] {
        let _removed = items.remove(0);
    }
}

fn signed_singleton_ranges(mut items: Vec<u16>) {
    for _ in -1..0 {
        let _removed = items.remove(0);
    }
    for _ in -1..-1 {
        let _removed = items.remove(0);
    }
    for _ in 0..-1 {
        let _removed = items.remove(0);
    }
    for _ in 0..=0 {
        let _removed = items.remove(0);
    }
    for _ in 0..=1 {
        let _removed = items.remove(0);
    }
    for _ in -START..0 {
        let _removed = items.remove(0);
    }
}

const START: i32 = 1;

fn one_explicit_removal(mut items: Vec<u16>) {
    items.remove(0);
}

fn one_iteration_break(mut items: Vec<u16>) {
    loop {
        items.remove(0);
        break;
    }
}

// Keep the missing semicolon so the break remains the block's tail expression.
#[rustfmt::skip]
fn final_break_expression(mut items: Vec<u16>) {
    loop {
        items.remove(0);
        break
    }
}

fn nonzero_index_in_loop(mut items: Vec<u16>) {
    while !items.is_empty() {
        items.remove(1);
    }
}

fn fresh_items() -> Vec<u16> {
    vec![1]
}

fn temporary_receiver() {
    loop {
        fresh_items().remove(0);
    }
}

fn make_inclusive_range(start: usize, end: usize) -> std::ops::RangeInclusive<usize> {
    start..=end
}

fn inclusive_range_from_helper(mut items: Vec<u16>) {
    for _ in make_inclusive_range(0, 2) {
        items.remove(0);
    }
}

fn inclusive_range_from_standard_constructor(mut items: Vec<u16>) {
    for _ in std::ops::RangeInclusive::new(0, 2) {
        items.remove(0);
    }
}

fn inclusive_range_from_closure(mut items: Vec<u16>) {
    for _ in (|start: usize, end: usize| start..=end)(0, 2) {
        items.remove(0);
    }
}

fn char_range_is_not_counted_as_integer(mut items: Vec<u16>) {
    for _ in 'a'..'z' {
        items.remove(0);
    }
}

fn loop_with_trailing_local(mut items: Vec<u16>) {
    loop {
        items.remove(0);
        let _remaining = items.len();
    }
}

static mut GLOBAL_ITEMS: Vec<u16> = Vec::new();

#[allow(static_mut_refs)]
fn nonlocal_static_receiver() {
    unsafe {
        while !GLOBAL_ITEMS.is_empty() {
            GLOBAL_ITEMS.remove(0);
        }
    }
}

fn singleton_constructed_inside_loop() {
    for _ in 0..100 {
        let mut items = vec![7];
        items.remove(0);
    }
}

fn vector_replaced_each_iteration(mut items: Vec<u16>) {
    loop {
        items = vec![7];
        items.remove(0);
    }
}

fn reference_target_replaced_each_iteration(items: &mut Vec<u16>) {
    loop {
        *items = vec![7];
        items.remove(0);
    }
}

struct Lookalike(Vec<u16>);

impl Lookalike {
    fn remove(&mut self, index: usize) -> u16 {
        self.0.remove(index)
    }
}

fn lookalike_method(mut items: Lookalike) {
    loop {
        items.remove(0);
    }
}

fn helper_call(mut items: Vec<u16>) {
    while !items.is_empty() {
        remove_first(&mut items);
    }
}

fn remove_first(items: &mut Vec<u16>) {
    items.remove(0);
}

fn closure_created_but_not_called(mut items: Vec<u16>) {
    while !items.is_empty() {
        let _remove = || items.remove(0);
    }
}

fn nested_closure_reads_persistent_vector(mut items: Vec<u16>) {
    while !items.is_empty() {
        let remaining_len = || items.len();
        let _observed_len = remaining_len();
        let _removed = items.remove(0);
    }
}

fn main() {}
