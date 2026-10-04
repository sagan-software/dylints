#![feature(rustc_private)]
//! UI cases for `tokio_spawn_blocking_async_closure`.

use std::future::{Ready, ready};

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use tokio_spawn_blocking_async_closure as _;
use tokio_support as _;

/// Returns a future for the direct async-function case.
async fn work() -> u8 {
    ready(1).await
}

/// Returns a ready future for the direct function-item case.
fn ready_value() -> Ready<u8> {
    ready(1)
}

/// Returns a future through another async function.
fn make_future() -> impl Future<Output = u8> {
    work()
}

/// Runs the closure and function-item UI cases.
fn main() {
    #[expect(
        closure_returning_async_block,
        reason = "The UI case covers a closure that returns an async block."
    )]
    drop(tokio::task::spawn_blocking(|| async {}));
    drop(tokio::task::spawn_blocking(async || 1_u8));
    let future_closure = || make_future();
    drop(tokio::task::spawn_blocking(future_closure));
    #[expect(
        clippy::redundant_closure,
        reason = "The UI case covers a closure that calls a future-returning function."
    )]
    drop(tokio::task::spawn_blocking(|| make_future()));
    drop(tokio::task::spawn_blocking(make_future));
    drop(tokio::task::spawn_blocking(ready_value));
    drop(tokio::task::spawn_blocking(|| {}));
    drop(tokio::task::spawn_blocking(|| 1_u8));
}
