//! Compiler UI cases for `PhantomData` and stored guards at optimization level zero.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "compiler UI fixture items are intentionally not executed"
)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_bind_to_scope_in_async as _;
use insta_support as _;

// compile-flags: -Copt-level=0

use insta::Settings;

/// Await a pending operation to make a suspension point in the generated future.
async fn other_work() {
    std::future::pending::<()>().await;
}

/// Consume a reference so the marker remains live through the awaited operation.
const fn observe<T: ?Sized>(_: &T) {}

/// Drop a guard and return a marker that stores only its type.
fn discard_into_marker<T>(value: T) -> std::marker::PhantomData<T> {
    drop(value);
    std::marker::PhantomData
}

/// Keep a type-only marker across an async suspension after dropping its guard.
async fn dropped_guard_type_marker_across_await(settings: &Settings) {
    let marker = discard_into_marker(settings.bind_to_scope());
    other_work().await;
    observe(&marker);
}

/// Keep an actual guard across an async suspension as the positive control.
async fn stored_guard_control(settings: &Settings) {
    let guard = settings.bind_to_scope();
    other_work().await;
    drop(guard);
}

fn main() {}
