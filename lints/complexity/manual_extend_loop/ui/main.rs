// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use std::{
    cell::Cell,
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

fn extend_vec(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        output.push(value);
    }
}

fn extend_array(output: &mut Vec<i32>) {
    for value in [1, 2, 3] {
        output.push(value);
    }
}

fn extend_deque(output: &mut VecDeque<i32>, values: Vec<i32>) {
    // Transformed items keep help-only output.
    for value in values {
        output.push_back(value * 2);
    }
}

fn keyword_like_name(output: &mut Vec<i32>, returned: Vec<i32>) {
    for value in returned {
        output.push(value);
    }
}

fn tail_position(output: &mut Vec<i32>, values: &[i32]) {
    for value in values.iter().copied() {
        output.push(value)
    }
}

fn labeled(output: &mut Vec<i32>, values: Vec<i32>) {
    'items: for value in values {
        output.push(value);
    }
}

fn reference_items<'a>(output: &mut Vec<&'a i32>, values: &'a [i32]) {
    for value in values {
        output.push(value);
    }
    let _ = output.len();
}

fn dereferenced_target(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        (*output).push(value);
    }
}

struct SideEffectDerefMut {
    values: Vec<i32>,
    calls: Cell<usize>,
}

impl Deref for SideEffectDerefMut {
    type Target = Vec<i32>;

    fn deref(&self) -> &Self::Target {
        self.calls.set(self.calls.get() + 1);
        &self.values
    }
}

impl DerefMut for SideEffectDerefMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.calls.set(self.calls.get() + 1);
        &mut self.values
    }
}

fn custom_deref_mut(output: &mut SideEffectDerefMut, values: Vec<i32>) {
    // Keep quiet when a custom dereference supplies the standard `push` method.
    for value in values {
        (*output).push(value);
    }
}

fn custom_deref_mut_implicit(output: &mut SideEffectDerefMut, values: Vec<i32>) {
    // Keep quiet when implicit dereference supplies the standard `push` method.
    for value in values {
        output.push(value);
    }
}

struct Holder {
    items: Vec<i32>,
}

struct DerefHolder {
    holder: Holder,
    calls: Cell<usize>,
}

impl Deref for DerefHolder {
    type Target = Holder;

    fn deref(&self) -> &Self::Target {
        self.calls.set(self.calls.get() + 1);
        &self.holder
    }
}

impl DerefMut for DerefHolder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.calls.set(self.calls.get() + 1);
        &mut self.holder
    }
}

fn custom_field_deref(output: &mut DerefHolder, values: Vec<i32>) {
    // Keep quiet when a custom field dereference supplies the standard `push` method.
    for value in values {
        output.items.push(value);
    }
}

impl Holder {
    fn add(&mut self, values: Vec<i32>) {
        for value in values {
            self.items.push(value);
        }
    }
}

#[rustfmt::skip]
fn match_arm(output: &mut Vec<i32>, values: Vec<i32>, flag: bool) {
    // A match arm cannot take the statement rewrite.
    match flag {
        true => for value in values {
            output.push(value);
        },
        false => {}
    }
}

fn pattern_item(output: &mut Vec<i32>, values: &[i32]) {
    // A destructuring pattern keeps help-only output.
    for &value in values {
        output.push(value);
    }
}

fn coerced_item<'a>(output: &mut Vec<&'a i32>, values: &'a mut [i32]) {
    // The reborrow coercion from `&mut i32` keeps help-only output.
    for value in values {
        output.push(value);
    }
}

struct Custom(Vec<i32>);

impl Custom {
    fn push(&mut self, value: i32) {
        self.0.push(value);
    }
}

fn custom(output: &mut Custom, values: Vec<i32>) {
    // A local `push` method is not a standard insertion.
    for value in values {
        output.push(value);
    }
}

fn assignment(last: &mut i32, values: Vec<i32>) {
    for value in values {
        *last = value;
    }
}

fn early_exit(output: &mut Vec<i32>, values: Vec<i32>) -> Option<()> {
    // `?` exits the function from inside the loop.
    for value in values {
        output.push(value.checked_add(1)?);
    }
    Some(())
}

fn reads_target(output: &mut Vec<usize>, values: Vec<usize>) {
    // Each insertion reads the length changed by the previous insertion.
    for value in values {
        output.push(output.len() + value);
    }
}

fn reads_target_in_closure(output: &mut Vec<usize>, values: Vec<usize>) {
    for value in values {
        output.push((|| output.len())() + value);
    }
}

fn source_reads_target(output: &mut Vec<i32>) {
    // Keep quiet when the loop source mutates the target before each insertion.
    for value in {
        output.clear();
        [1, 2]
    } {
        output.push(value);
    }
}

struct SizeHintItems {
    hinted: Cell<bool>,
    remaining: i32,
}

impl Iterator for SizeHintItems {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(self.remaining + if self.hinted.get() { 20 } else { 0 })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.hinted.set(true);
        let remaining = self.remaining as usize;
        (remaining, Some(remaining))
    }
}

fn custom_size_hint(output: &mut Vec<i32>) {
    for value in (SizeHintItems {
        hinted: Cell::new(false),
        remaining: 3,
    }) {
        output.push(value);
    }
}

fn custom_size_hint_transformed(output: &mut Vec<i32>) {
    for value in (SizeHintItems {
        hinted: Cell::new(false),
        remaining: 3,
    }) {
        output.push(value + 1);
    }
}

fn target(outputs: &mut [Vec<i32>; 2]) -> &mut Vec<i32> {
    &mut outputs[0]
}

fn computed_target(outputs: &mut [Vec<i32>; 2], values: Vec<i32>) {
    // The call that yields the target runs once per item.
    for value in values {
        target(outputs).push(value);
    }
}

macro_rules! generated {
    ($output:expr, $values:expr) => {
        for value in $values {
            $output.push(value);
        }
    };
}

fn macro_loop(output: &mut Vec<i32>, values: Vec<i32>) {
    generated!(output, values);
}

fn main() {
    extend_vec(&mut Vec::new(), vec![1]);
    extend_array(&mut Vec::new());
    extend_deque(&mut VecDeque::new(), vec![1]);
    custom(&mut Custom(Vec::new()), vec![1]);
    custom_deref_mut(
        &mut SideEffectDerefMut {
            values: Vec::new(),
            calls: Cell::new(0),
        },
        vec![1],
    );
    custom_deref_mut_implicit(
        &mut SideEffectDerefMut {
            values: Vec::new(),
            calls: Cell::new(0),
        },
        vec![1],
    );
    custom_field_deref(
        &mut DerefHolder {
            holder: Holder { items: Vec::new() },
            calls: Cell::new(0),
        },
        vec![1],
    );
    source_reads_target(&mut Vec::new());
    custom_size_hint(&mut Vec::new());
    custom_size_hint_transformed(&mut Vec::new());
}
