#![feature(rustc_private)]

//! This integration test constructs an iterator whose `size_hint` mutates shared
//! state. It compares the original loop with the collection rewrite and records
//! their different values. The fixture keeps iterator behavior local so the test
//! documents the rewrite boundary. The explicit loop and rewritten expression
//! consume equivalent source items otherwise.

use std::cell::Cell;

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use manual_iterator_loop as _;

struct SideEffectingItems {
    hinted: Cell<bool>,
    remaining: usize,
}

impl Iterator for SideEffectingItems {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // Stop before decrementing so exhaustion remains exact.
        if self.remaining == 0 {
            return None;
        }
        // Update the remaining count before reading whether size_hint ran.
        self.remaining -= 1;
        // A size_hint call changes every later item produced by this fixture.
        Some(self.remaining + if self.hinted.get() { 20 } else { 0 })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.hinted.set(true);
        (self.remaining, Some(self.remaining))
    }
}

const fn side_effecting_items() -> SideEffectingItems {
    SideEffectingItems {
        hinted: Cell::new(false),
        remaining: 3,
    }
}

#[test]
fn collect_can_change_values_when_size_hint_has_side_effects() {
    let mut original = Vec::new();
    // The explicit loop never queries size_hint before it consumes the source.
    for value in side_effecting_items() {
        Vec::push(&mut original, value * 2);
    }
    // collect queries size_hint after its first next call and changes later yielded values.
    let rewritten: Vec<_> = side_effecting_items().map(|value| value * 2).collect();

    // The differing vectors prove that the collection rewrite changes behavior.
    assert_eq!((original, rewritten), (vec![4, 2, 0], vec![4, 42, 40]));
}
