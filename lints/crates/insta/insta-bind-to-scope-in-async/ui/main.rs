#![allow(dead_code)]

// compile-flags: -Copt-level=3

use insta::Settings;

async fn other_work() {}

fn observe<T: ?Sized>(_: &T) {}

async fn block_ends_before_await(settings: &Settings) {
    {
        let guard = settings.bind_to_scope();
        observe(&guard);
    }
    other_work().await;
}

async fn explicit_drop_before_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    observe(&guard);
    drop(guard);
    other_work().await;
}

async fn no_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    observe(&guard);
}

async fn mixed_live_and_dropped(settings: &Settings) {
    let dropped = settings.bind_to_scope();
    drop(dropped);
    let live = settings.bind_to_scope();
    other_work().await;
    drop(live);
}

async fn dropped_on_one_branch(settings: &Settings, should_drop: bool) {
    let guard = settings.bind_to_scope();
    if should_drop {
        drop(guard);
    }
    other_work().await;
}

async fn live_across_multiple_suspensions(settings: &Settings) {
    let guard = settings.bind_to_scope();
    other_work().await;
    observe(&guard);
    other_work().await;
    drop(guard);
}

async fn borrowed_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let borrowed = &guard;
    other_work().await;
    observe(borrowed);
}

async fn moved_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let moved = guard;
    other_work().await;
    drop(moved);
}

async fn boxed_guard_live_through_await(settings: &Settings) {
    let boxed = Box::new(settings.bind_to_scope());
    other_work().await;
    drop(boxed);
}

async fn vector_guard_live_through_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    other_work().await;
    drop(guards);
}

async fn popped_vector_guard_live_through_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    let popped = guards.pop();
    drop(guards);
    other_work().await;
    drop(popped);
}

async fn popped_vector_guard_dropped_before_await(settings: &Settings) {
    let mut guards = Vec::new();
    guards.push(settings.bind_to_scope());
    drop(guards.pop());
    other_work().await;
}

async fn boxed_guard_dropped_before_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let boxed = Box::new(guard);
    drop(boxed);
    other_work().await;
}

struct ConcreteGuardWrapper {
    guard: insta::internals::SettingsBindDropGuard,
}

#[inline(never)]
fn wrap_guard(guard: insta::internals::SettingsBindDropGuard) -> ConcreteGuardWrapper {
    ConcreteGuardWrapper { guard }
}

async fn concrete_field_wrapper_guard_live_through_await(settings: &Settings) {
    let wrapped = wrap_guard(settings.bind_to_scope());
    other_work().await;
    drop(wrapped);
}

async fn tuple_sibling_guard_after_first_drop(settings: &Settings) {
    let guards = (settings.bind_to_scope(), settings.bind_to_scope());
    drop(guards.0);
    other_work().await;
    drop(guards.1);
}

#[inline(never)]
fn identity<T>(value: T) -> T {
    value
}

async fn tuple_guard_moved_through_identity(settings: &Settings) {
    let guards = (settings.bind_to_scope(),);
    let moved = identity(guards);
    other_work().await;
    drop(moved);
}

async fn tuple_identity_sibling_guard_after_first_drop(settings: &Settings) {
    let guards = identity((settings.bind_to_scope(), settings.bind_to_scope()));
    drop(guards.0);
    other_work().await;
    drop(guards.1);
}

async fn nested_aggregate_sibling_guard_after_drop(settings: &Settings) {
    let old = ((settings.bind_to_scope(), settings.bind_to_scope()),);
    let moved = (old.0,);
    drop(moved.0.0);
    other_work().await;
    drop(moved.0.1);
}

async fn boxed_nested_future_guard_live_through_await(settings: &Settings) {
    let guard = settings.bind_to_scope();
    let boxed = Box::pin(async move { drop(guard) });
    other_work().await;
    drop(boxed);
}

struct OtherSettings;

impl OtherSettings {
    fn bind_to_scope(&self) {}
}

async fn same_named_unrelated_api(settings: &OtherSettings) {
    settings.bind_to_scope();
    other_work().await;
}

fn main() {}
