// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    task::{Context, Poll},
};

use insta::Settings;

async fn work() {}

/// Return a future through an opaque type for callable-result checks.
fn make_future() -> impl Future<Output = ()> {
    work()
}

/// Return a standard ready future for callable-result checks.
fn make_ready_future() -> Ready<()> {
    ready(())
}

/// Return a pinned trait-object future for callable-result checks.
fn make_boxed_future() -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(work())
}

/// A caller-defined future used to check semantic trait resolution.
struct CustomFuture;

impl Future for CustomFuture {
    type Output = ();

    /// Complete immediately when the test polls this future.
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

/// Return the caller-defined future from a function item.
fn make_custom_future() -> CustomFuture {
    CustomFuture
}

async fn invalid() {
    let settings = Settings::clone_current();
    let future = settings.bind(|| async { work().await });
    future.await;
}

async fn invalid_async_move() {
    let settings = Settings::clone_current();
    let name = String::from("name");
    settings
        .bind(|| async move {
            drop(name);
            work().await;
        })
        .await;
}

async fn invalid_move_closure_borrowing_block() {
    let settings = Settings::clone_current();
    settings.bind(move || async { work().await }).await;
}

async fn invalid_async_closure() {
    let settings = Settings::clone_current();
    settings.bind(async || work().await).await;
}

/// Return a future from a closure through an ordinary function call.
async fn invalid_future_from_call() {
    let settings = Settings::clone_current();
    settings.bind(|| work()).await;
}

/// Return an `impl Future` value from a function item.
async fn invalid_impl_future_function_item() {
    let settings = Settings::clone_current();
    settings.bind(make_future).await;
}

/// Pass a local closure value that returns a future.
async fn invalid_stored_closure() {
    let settings = Settings::clone_current();
    let future_factory = || make_future();
    settings.bind(future_factory).await;
}

/// Return `Ready` from a function item.
async fn invalid_ready_function_item() {
    let settings = Settings::clone_current();
    settings.bind(make_ready_future).await;
}

/// Return a boxed future from a function item.
async fn invalid_boxed_function_item() {
    let settings = Settings::clone_current();
    settings.bind(make_boxed_future).await;
}

/// Keep a side effect in the callable before it returns a future.
async fn invalid_side_effect_closure() {
    let settings = Settings::clone_current();
    let mut called = false;
    settings
        .bind(|| {
            called = true;
            work()
        })
        .await;
    assert!(called);
}

/// Move a captured value into the callable before it returns a future.
async fn invalid_capturing_closure() {
    let settings = Settings::clone_current();
    let name = String::from("name");
    let future_factory = move || {
        drop(name);
        make_future()
    };
    settings.bind(future_factory).await;
}

/// Resolve the future bound on a generic callable result.
async fn invalid_generic_future<T: Future<Output = ()>>(settings: &Settings, future: T) {
    settings.bind(|| future).await;
}

/// Return a caller-defined `Future` implementation from a function item.
async fn invalid_custom_future() {
    let settings = Settings::clone_current();
    settings.bind(make_custom_future).await;
}

async fn valid_async_binding() {
    let settings = Settings::clone_current();
    settings.bind_async(async { work().await }).await;
}

fn valid_synchronous_binding() {
    let settings = Settings::clone_current();
    settings.bind(|| {});
}

struct OtherSettings;

impl OtherSettings {
    fn bind<T>(&self, function: impl FnOnce() -> T) -> T {
        function()
    }
}

async fn same_name_is_not_insta() {
    let settings = OtherSettings;
    settings.bind(|| async { work().await }).await;
}

fn main() {}
