#![allow(dead_code)]

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    task::{Context, Poll},
};

use tracing::{Dispatch, Instrument, info_span, subscriber};

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

async fn invalid_span_scope() {
    let span = info_span!("span_scope");
    let future = span.in_scope(|| async { work().await });
    future.await;
}

async fn invalid_subscriber_scope() {
    let future = subscriber::with_default(subscriber::NoSubscriber::default(), || async {
        work().await;
    });
    future.await;
}

/// Return a future from a closure through an ordinary function call.
async fn invalid_span_scope_future_from_call() {
    let span = info_span!("span_scope_call");
    let future = span.in_scope(|| make_future());
    future.await;
}

/// Pass a local closure value that returns a future.
async fn invalid_span_scope_stored_closure() {
    let span = info_span!("span_scope_stored");
    let future_factory = || make_future();
    let future = span.in_scope(future_factory);
    future.await;
}

/// Return `Ready` from a function item.
async fn invalid_span_scope_ready_function_item() {
    let span = info_span!("span_scope_ready");
    span.in_scope(make_ready_future).await;
}

/// Return an opaque future from a subscriber-scope function item.
async fn invalid_subscriber_scope_function_item() {
    subscriber::with_default(subscriber::NoSubscriber::default(), make_future).await;
}

/// Return a boxed future from a dispatcher-scope function item.
async fn invalid_dispatcher_scope_boxed_function_item() {
    let dispatch = Dispatch::new(subscriber::NoSubscriber::default());
    tracing::dispatcher::with_default(&dispatch, make_boxed_future).await;
}

/// Resolve the future bound on a generic scope result.
async fn invalid_generic_span_future<T: Future<Output = ()>>(span: &tracing::Span, future: T) {
    span.in_scope(|| future).await;
}

/// Return a caller-defined `Future` implementation from a span-scope function item.
async fn invalid_custom_span_future() {
    let span = info_span!("span_scope_custom");
    span.in_scope(make_custom_future).await;
}

async fn valid_instrumented() {
    work().instrument(info_span!("instrumented")).await;
}

fn valid_synchronous_scope() {
    let span = info_span!("synchronous");
    span.in_scope(|| {});
}

/// Return synchronously from a subscriber scope.
fn valid_synchronous_subscriber_scope() {
    subscriber::with_default(subscriber::NoSubscriber::default(), || ());
}

/// Provide an unrelated method with the same name as `Span::in_scope`.
struct OtherSpan;

impl OtherSpan {
    /// Return this unrelated callable result unchanged.
    fn in_scope<F: FnOnce() -> T, T>(&self, function: F) -> T {
        function()
    }
}

/// Ensure an unrelated method does not match by name alone.
async fn same_name_is_not_tracing() {
    OtherSpan.in_scope(|| work()).await;
}

fn main() {}
