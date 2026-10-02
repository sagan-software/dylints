# tokio-sleep-in-loop

## What it does

Checks for calls to `tokio::time::sleep` inside the body of a `loop`, `while`,
or `for` expression.

## Why is this bad?

Each sleep measures its delay from the end of the previous work. The time the
work takes is added to every cycle, so a periodic schedule drifts.
`tokio::time::interval` tracks fixed deadlines and lets you choose how missed
ticks are handled.

## Known problems

Retry backoff, rate-limit recovery, and polling loops often need a delay
measured from the previous attempt. The lint cannot tell those loops from
periodic ones and warns on both.

The lint warns on any `sleep` call that is lexically inside a loop, including
one inside a closure or async block created in the loop. For example, a task
spawned once per iteration that sleeps once still triggers the lint.

The lint checks only `tokio::time::sleep`. Calls to `tokio::time::sleep_until`
do not trigger it.

## Example

```rust
async fn run(period: std::time::Duration) {
    loop {
        work().await;
        tokio::time::sleep(period).await;
    }
}
```

## Use instead

```rust
async fn run(period: std::time::Duration) {
    let mut ticker = tokio::time::interval(period);
    loop {
        ticker.tick().await;
        work().await;
    }
}
```
