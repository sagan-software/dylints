//! Compiler UI cases for Insta guard lifetimes across async suspension points.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "compiler UI fixture items are intentionally not executed"
)]

// compile-flags: -Copt-level=3

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta::Settings;
use insta_bind_to_scope_in_async as _;
use insta_support as _;

/// Await pending work to create an async suspension point.
async fn other_work() {
    std::future::pending::<()>().await;
}

/// Borrow a value so the fixture can keep it live across an await.
const fn observe<T: ?Sized>(_: &T) {}

/// Release a guard at the end of a block before awaiting.
async fn block_ends_before_await(settings: &Settings) {
    {
        let guard = settings.bind_to_scope();
        observe(&guard);
    }
    other_work().await;
}

/// Release a guard explicitly before awaiting.
async fn explicit_drop_before_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    observe(&guard);
    drop(guard);
    other_work().await;
}

/// Keep the async function body without an await as a control.
#[expect(
    clippy::unused_async,
    reason = "this fixture preserves an async body with no suspension"
)]
async fn no_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    observe(&guard);
}

/// Keep one guard live while dropping another before awaiting.
async fn mixed_live_and_dropped(settings: &Settings) {
    let dropped = settings.bind_to_scope();
    drop(dropped);
    let live = settings.bind_to_scope();
    other_work().await;
    drop(live);
}

/// Keep a guard live on one branch across an await.
async fn dropped_on_one_branch(settings: &Settings, should_drop: bool) {
    let guard = settings.bind_to_scope();
    if should_drop {
        drop(guard);
    }
    other_work().await;
}

/// Keep a guard live through multiple async suspension points.
async fn live_across_multiple_suspensions(settings: &Settings) {
    let guard = settings.bind_to_scope();
    other_work().await;
    observe(&guard);
    other_work().await;
    drop(guard);
}

/// Keep a borrow of a guard live across an await.
async fn borrowed_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let borrowed = &guard;
    other_work().await;
    observe(borrowed);
}

/// Move a guard and keep it live across an await.
async fn moved_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let moved = guard;
    other_work().await;
    drop(moved);
}

/// Keep a boxed guard live across an await.
async fn boxed_guard_live_through_await(settings: &Settings) {
    let boxed = Box::new(settings.bind_to_scope());
    other_work().await;
    drop(boxed);
}

/// Keep an actual guard inside an `Option` across an async suspension.
async fn optional_guard_live_through_await(settings: &Settings) {
    let guard = Some(settings.bind_to_scope());
    other_work().await;
    drop(guard);
}

/// Keep a vector containing a guard live across an await.
#[expect(
    clippy::vec_init_then_push,
    reason = "explicit Vec push preserves the MIR ownership regression"
)]
async fn vector_guard_live_through_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    other_work().await;
    drop(guards);
}

/// Keep a popped guard live while dropping the remaining vector before await.
async fn popped_vector_guard_live_through_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    let popped = guards.pop();
    drop(guards);
    other_work().await;
    drop(popped);
}

/// Drop a popped guard and vector before awaiting.
async fn popped_vector_guard_dropped_before_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    drop(guards.pop());
    other_work().await;
}

/// Drop a boxed guard before awaiting.
async fn boxed_guard_dropped_before_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let boxed = Box::new(guard);
    drop(boxed);
    other_work().await;
}

/// Store the guard in a concrete field.
struct ConcreteGuardWrapper {
    /// The stored settings guard.
    guard: insta::internals::SettingsBindDropGuard,
}

/// Wrap a guard in the concrete fixture type.
#[inline(never)]
const fn wrap_guard(guard: insta::internals::SettingsBindDropGuard) -> ConcreteGuardWrapper {
    ConcreteGuardWrapper { guard }
}

/// Keep a guard stored in a wrapper field across an await.
async fn concrete_field_wrapper_guard_live_through_await(settings: &Settings) {
    let wrapped = wrap_guard(settings.bind_to_scope());
    other_work().await;
    drop(wrapped);
}

/// Keep the second tuple guard live after dropping the first.
async fn tuple_sibling_guard_after_first_drop(settings: &Settings) {
    let guards = (settings.bind_to_scope(), settings.bind_to_scope());
    drop(guards.0);
    other_work().await;
    drop(guards.1);
}

/// Return the value unchanged to retain a tuple-move control.
#[inline(never)]
const fn identity<T>(value: T) -> T {
    value
}

/// Keep a tuple guard live across an await after an identity call.
async fn tuple_guard_moved_through_identity(settings: &Settings) {
    let guards = (settings.bind_to_scope(),);
    let moved = identity(guards);
    other_work().await;
    drop(moved);
}

/// Keep the second tuple guard live after identity and a partial drop.
async fn tuple_identity_sibling_guard_after_first_drop(settings: &Settings) {
    let guards = identity((settings.bind_to_scope(), settings.bind_to_scope()));
    drop(guards.0);
    other_work().await;
    drop(guards.1);
}

/// Keep a nested aggregate sibling guard live after a partial drop.
async fn nested_aggregate_sibling_guard_after_drop(settings: &Settings) {
    let old = ((settings.bind_to_scope(), settings.bind_to_scope()),);
    let moved = (old.0,);
    drop(moved.0.0);
    other_work().await;
    drop(moved.0.1);
}

/// Keep a nested future containing a guard live across an await.
async fn boxed_nested_future_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let boxed = Box::pin(async move { drop(guard) });
    other_work().await;
    drop(boxed);
}

/// Drop a value and retain only its type in a `PhantomData` marker.
fn discard_into_marker<T>(value: T) -> std::marker::PhantomData<T> {
    drop(value);
    std::marker::PhantomData
}

/// Keep only the `PhantomData` type marker across an await.
async fn dropped_guard_type_marker_across_await(settings: &Settings) {
    let marker = discard_into_marker(settings.bind_to_scope());
    other_work().await;
    observe(&marker);
}

/// Provide an unrelated method with the same name as Insta's API.
struct OtherSettings;

impl OtherSettings {
    /// Observe the receiver without creating an Insta guard.
    const fn bind_to_scope(&self) {
        observe(self);
    }
}

/// Keep the unrelated same-named method as a negative control.
async fn same_named_unrelated_api(settings: &OtherSettings) {
    settings.bind_to_scope();
    other_work().await;
}

fn main() {}
