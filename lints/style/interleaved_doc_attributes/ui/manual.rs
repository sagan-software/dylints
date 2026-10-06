/// An unrecognized attribute requires an order review.
#[allow(dead_code)]
/// This diagnostic must not suggest an automatic reorder.
fn reviewed() {}

/// Multiple interruptions require an order review.
#[inline]
/// More documentation.
#[cold]
/// Final documentation.
fn multiple() {}

fn main() {
    reviewed();
    multiple();
}
