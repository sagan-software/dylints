# perf

Rust review lints for avoidable performance costs.

## Lints

- [`boxed_future_return`](boxed_future_return): flags functions, methods, and
  trait methods that return boxed futures instead of `async fn`.
- [`expensive_as_method`](expensive_as_method): flags inherent `as_*` methods
  that allocate, clone, parse, decode, or return `Option` or `Result`.
- [`owned_input_field_clones`](owned_input_field_clones): flags parameters
  when code clones their fields into a returned struct literal.
- [`ownership_at_boundaries`](ownership_at_boundaries): flags by-value `String`
  and `Vec` parameters of `pub` functions.
