# boxed_future_return

## What it does

Checks the declared return type of functions, methods, and trait methods for
a `Box<dyn Future>`, such as `Pin<Box<dyn Future<Output = T>>>` or a
`BoxFuture` alias. A boxed future nested inside another return type, such as
`Option<BoxFuture<'_, T>>` or a tuple, also triggers the lint.

## Why is this bad?

Each call allocates the future on the heap and calls it through a vtable. The
signature also hides that the function is async and loses auto traits such as
`Send` unless they are written out. An `async fn` or `-> impl Future` returns
the concrete future without these costs.

## Known problems

Boxing is sometimes required: recursive async functions, `dyn`-compatible
traits, and collections of different futures. The lint warns in these cases
too.

Trait impl methods are reported when the trait declares a boxed future. The
fix then belongs in the trait, which may live in another crate.

## Example

```rust
use std::future::Future;
use std::pin::Pin;

fn name_len(name: String) -> Pin<Box<dyn Future<Output = usize> + Send>> {
    Box::pin(async move { name.len() })
}
```

## Use instead

```rust
async fn name_len(name: String) -> usize {
    name.len()
}
```
