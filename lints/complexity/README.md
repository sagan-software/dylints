# complexity

Rust review lints for code that can usually become simpler.

## Lints

- [`collect_return`](collect_return): flags `pub` functions that end with
  `.collect()` into a standard collection return type.
- [`complicated_conditional`](complicated_conditional): flags `bool`
  conditions and initializers with too much inline work.
- [`consecutive_iterator_loops`](consecutive_iterator_loops): flags two adjacent
  `for` loops with the same body that can use `Iterator::chain`.
- [`contains_key_before_any`](contains_key_before_any): flags a separate
  `contains_key` call that can join the array searched with `any`.
- [`let_some_return_err`](let_some_return_err): flags
  `let Some(..) = option else { return Err(..) };` that can use `ok_or` and `?`.
- [`manual_adjacent_window_loop`](manual_adjacent_window_loop): flags index
  loops over adjacent pairs that can use `slice::windows(2)`.
- [`manual_arg_parsing`](manual_arg_parsing): flags calls to `std::env::args`
  and `std::env::args_os`.
- [`manual_entry_update`](manual_entry_update): flags map `Entry` matches that
  can use `and_modify(...).or_insert(...)`.
- [`manual_extend_loop`](manual_extend_loop): flags push loops that can use
  `Extend::extend`.
- [`manual_fallible_collect_loop`](manual_fallible_collect_loop): flags loops
  that push `?` results and can collect into `Result` or `Option`.
- [`manual_filter_for_each_loop`](manual_filter_for_each_loop): flags loops with
  one `if` that can use `filter(...).for_each(...)`.
- [`manual_filter_map_for_each_loop`](manual_filter_map_for_each_loop): flags
  loops with one `if let Some(..)` that can use `filter_map(...).for_each(...)`.
- [`manual_iterator_loop`](manual_iterator_loop): flags loops that fill a `Vec`,
  count matches, or set a `bool` flag where `collect`, `count`, `any`, or `all`
  fits.
- [`manual_option_take_if`](manual_option_take_if): flags conditional
  `Option::take` that can use `Option::take_if`.
- [`manual_partition_loop`](manual_partition_loop): flags loops that split items
  into two new collections and can use `Iterator::partition`.
- [`manual_passthrough_inspect`](manual_passthrough_inspect): flags an
  `Option` or `Result` observed and returned unchanged that can use `inspect`
  or `inspect_err`.
- [`manual_try_for_each_loop`](manual_try_for_each_loop): flags fallible loops
  that can use `Iterator::try_for_each`.
- [`manual_unzip_loop`](manual_unzip_loop): flags loops that split pairs into
  two new collections and can use `Iterator::unzip`.
- [`one_use_predicate_binding`](one_use_predicate_binding): flags generic
  `bool` bindings used once as the next `if` condition.
- [`one_use_private_helper`](one_use_private_helper): flags private
  one-expression functions called once in the same file.
- [`trivial_public_constructor`](trivial_public_constructor): flags `new`
  functions that only forward arguments into a struct with `pub` fields.
- [`unnecessary_map_err`](unnecessary_map_err): flags `.map_err` calls that only
  apply the `From` conversion that `?` already applies.
