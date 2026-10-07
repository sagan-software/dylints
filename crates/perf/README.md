# perf

Rust review lints for avoidable performance costs.

## Lints

- [`boxed_future_return`](boxed_future_return): flags functions, methods, and
  trait methods that return boxed futures instead of `async fn`.
- [`expensive_as_method`](expensive_as_method): flags inherent `as_*` methods
  that allocate, clone, parse, decode, or return `Option` or `Result`.
- [`expensive_sort_key`](expensive_sort_key): flags supported allocating key
  closures passed to stable `sort_by_key`.
- [`owned_input_field_clones`](owned_input_field_clones): flags parameters
  when code clones their fields into a returned struct literal.
- [`ownership_at_boundaries`](ownership_at_boundaries): flags by-value `String`
  and `Vec` parameters of `pub` functions.
- [`vec_front_removal_in_loop`](vec_front_removal_in_loop): flags `Vec::remove(0)`
  on a persistent vector in supported repeating loops.
