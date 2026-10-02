// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

#[rustfmt::skip]
mod nested {
    fn same_line_nested_first() {} fn same_line_nested_second() {}
    struct SameLineStructFirst; struct SameLineStructSecond;

    fn separated_nested_first() {}
    fn separated_nested_second() {}
}

#[rustfmt::skip]
mod attributes {
    fn first() {} #[inline] fn second() {}
    /// Documented first.
    fn third() {} /// Documented fourth.
    fn fourth() {}
    fn fifth() {} /* A block comment stays with the previous item. */ fn sixth() {}
}

#[rustfmt::skip]
macro_rules! generated_module {
    () => {
        mod generated { fn first() {} fn second() {} }
    };
}

generated_module!();

fn comment_gap() {} // The following item starts on a new line.
fn after_comment_gap() {}

fn attribute_gap() {}
#[inline]
fn after_attribute_gap() {}

trait AssociatedItemsMayShareLines {
    fn first(&self);
    fn second(&self);
}

impl AssociatedItemsMayShareLines for () {
    fn first(&self) {}
    fn second(&self) {}
}

fn local_items_are_not_module_items() {
    struct LocalFirst;
    struct LocalSecond;
    let _ = (LocalFirst, LocalSecond);
}

fn main() {}
