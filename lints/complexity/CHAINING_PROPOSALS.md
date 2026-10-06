# Rust method-chaining lint proposals

- Standard library: Rust 1.97.1, the current stable release documented on
  2026-08-05.
- Scope: stable standard-library methods that replace local imperative loops,
  mutable accumulators, branches, and pass-through matches.
- Existing baseline: this repository enables Clippy `all`, `complexity`,
  `nursery`, `pedantic`, `perf`, `style`, and `suspicious`. It also enables
  `option_if_let_else` explicitly.
- Existing lints: `manual_iterator_loop` already covers a new `Vec`
  filled with `push`, predicate counting, `any`, and `all`.
  `let_some_return_err` already covers one `Option::ok_or_else` case.
- Implementation status: all 12 ranked proposals have implementations. The
  suitability labels below preserve the pre-implementation research ranking.

## Documentation read

The research covered these official Rust sources:

- [`std::iter`](https://doc.rust-lang.org/stable/std/iter/index.html),
  [`Iterator`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html),
  [`IntoIterator`](https://doc.rust-lang.org/stable/std/iter/trait.IntoIterator.html),
  [`FromIterator`](https://doc.rust-lang.org/stable/std/iter/trait.FromIterator.html),
  and [`Extend`](https://doc.rust-lang.org/stable/std/iter/trait.Extend.html).
- [`Option`](https://doc.rust-lang.org/stable/std/option/index.html) and
  [`Result`](https://doc.rust-lang.org/stable/std/result/index.html), including
  their method overviews, iterator implementations, and collection behavior.
- [`bool`](https://doc.rust-lang.org/stable/std/primitive.bool.html),
  [`ControlFlow`](https://doc.rust-lang.org/stable/std/ops/enum.ControlFlow.html),
  [`slice`](https://doc.rust-lang.org/stable/std/primitive.slice.html), and
  [`str`](https://doc.rust-lang.org/stable/std/primitive.str.html).
- [`HashMap::Entry`](https://doc.rust-lang.org/stable/std/collections/hash_map/enum.Entry.html)
  and
  [`BTreeMap::Entry`](https://doc.rust-lang.org/stable/std/collections/btree_map/enum.Entry.html).
- [`Future`](https://doc.rust-lang.org/stable/std/future/trait.Future.html) and
  [`IntoFuture`](https://doc.rust-lang.org/stable/std/future/trait.IntoFuture.html).
- The current official
  [Clippy lint index](https://rust-lang.github.io/rust-clippy/master/index.html)
  for overlap analysis.

Coverage gaps: the review covered the public method families that can support
the lint-specific style and every API used by a proposal. It did not inventory
every numeric, path, I/O, platform-specific, or synchronization method because
those APIs do not form a general control-flow chaining vocabulary.

## Chaining vocabulary

### Iterator sources and ownership

`iter`, `iter_mut`, and `into_iter` select shared borrowing, mutable borrowing,
or ownership. `IntoIterator` lets arrays, slices, collections, `Option`,
`Result`, ranges, and custom collection types enter the same pipeline.
`iter::once`, `once_with`, `empty`, `repeat`, `repeat_with`, `from_fn`, and
`successors` create pipelines without a source collection.

Ownership is part of the rewrite contract. A lint must preserve whether the
loop moves items, yields shared references, or yields mutable references.

### Lazy Iterator adapters

- Transform: `map`, `filter`, `filter_map`, `flat_map`, `flatten`, `scan`,
  `map_while`, `inspect`, `copied`, and `cloned`.
- Bound or select: `skip`, `take`, `skip_while`, `take_while`, and `step_by`.
- Compose: `chain`, `zip`, `enumerate`, `rev`, `cycle`, `peekable`, `fuse`,
  and `by_ref`.

These adapters are lazy. A terminal operation must consume the pipeline before
its closures run. A suggestion must not move side effects from loop execution
into pipeline construction or leave a new pipeline unused.

### Iterator terminal operations

- Build or mutate collections: `collect`, `partition`, `unzip`, and the
  `Extend::extend` implementations on standard collections.
- Aggregate: `fold`, `reduce`, `sum`, `product`, `count`, `min`, and `max`,
  including their comparator and key variants.
- Search or validate: `find`, `find_map`, `position`, `any`, and `all`.
- Apply the side effects: `for_each` and `try_for_each`.
- Fallible aggregation: `try_fold`, plus stable `collect` through
  `FromIterator<Result<_, _>>` or `FromIterator<Option<_>>`.
- Compare: `eq`, `ne`, `cmp`, `partial_cmp`, and sortedness checks.

`any`, `all`, `find`, `find_map`, `position`, `try_fold`, and `try_for_each`
short-circuit. A rewrite must preserve the original evaluation order and early
exit point.

The stable rustdoc also displays nightly methods. The proposals do not depend
on nightly-only APIs such as `next_chunk`, `array_chunks`, `try_collect`,
`try_find`, or `try_reduce`. Stable fallible collection must use `collect`
with an explicit `Result` or `Option` target.

### Option, Result, bool, and ControlFlow

`Option` and `Result` give pipelines over zero or one successful value:

- Borrow: `as_ref`, `as_mut`, `as_deref`, and `as_deref_mut`.
- Transform: `map`, `map_err`, `filter`, `flatten`, `transpose`, `inspect`,
  and `inspect_err` where applicable.
- Continue or recover: `and_then`, `or_else`, `and`, and `or`.
- Collapse to a value: `map_or`, `map_or_else`, `unwrap_or`,
  `unwrap_or_else`, and `unwrap_or_default`.
- Query with a predicate: `is_some_and`, `is_none_or`, `is_ok_and`, and
  `is_err_and`.
- Mutate `Option` in place: `insert`, `get_or_insert`,
  `get_or_insert_with`, `take`, and `take_if`.

`bool::then` lazily creates an `Option`; `bool::then_some` evaluates its value
eagerly. The documented `bool::ok_or` and `bool::ok_or_else` methods are still
nightly in Rust 1.97.1, so stable lint suggestions must not use them.

`ControlFlow` supplies a typed early-exit value for `try_for_each` and
`try_fold`. It can use `?` to propagate `Break` while `Continue` proceeds.

### Collections, slices, strings, and futures

Standard `Entry` values from a map chain `and_modify` with `or_insert`,
`or_insert_with`, `or_insert_with_key`, or `or_default`. This covers an
occupied update and vacant insertion with one lookup.

Slices give `iter`, `windows`, `chunks`, `chunks_exact`, `split`, and their
mutable or reverse variants. Strings give iterator-producing methods such
as `chars`, `char_indices`, `bytes`, `lines`, `split`, `matches`, and
`match_indices`. These methods can feed ordinary Iterator chains.

The stable `Future` trait only provides `poll`. `IntoFuture` only provides
`into_future`. Standard Rust therefore has no general `Future::map` or
`Future::and_then` target for a lint. Async chaining requires a named
ecosystem extension trait and belongs in a separate crate-specific proposal.

## Existing Clippy coverage

Do not create private duplicates for these current Clippy lints:

- `manual_filter`, `manual_map`, `manual_find`, `manual_flatten`,
  `manual_inspect`, `manual_option_zip`, `manual_try_fold`, and
  `manual_retain`.
- `manual_filter_map` and `manual_find_map` for redundant adapter pairs.
- `option_if_let_else`, `manual_unwrap_or`, `manual_unwrap_or_default`,
  `manual_ok_or`, and `manual_is_variant_and`.
- `map_entry`, `explicit_counter_loop`, `needless_range_loop`, and
  `while_let_on_iterator`.
- `if_then_some_else_none` for `bool::then` or `bool::then_some`.

Clippy's `needless_for_each` intentionally permits `for_each` after a
transformer such as `filter`. The transformed-loop proposals below target that
documented exception and remain personal style lints.

## Ranked proposals

### 1. `lints/complexity/manual_try_for_each_loop`

- Misuse: a `for` loop contains one fallible action using `?`, and the
  enclosing expression returns the matching success value after the loop.
- Why this matters: `try_for_each` states both repeated application and
  short-circuiting in one expression.
- Evidence:
  [`Iterator::try_for_each`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.try_for_each)
  and its official `Result` and `ControlFlow` examples.
- Detection: resolve the standard desugaring for an `IntoIterator` loop. need a
  single call under the `?` desugaring, no `break`, `continue`, or `return`,
  and the same loop-call and enclosing residual types. need the following
  tail to be `Ok(())`, `Some(())`, or `ControlFlow::Continue(())`.
- Suggested fix: replace `for item in items { write(item)?; } Ok(())` with
  `items.into_iter().try_for_each(write)`. Use help-only when the closure or
  error type needs annotation.
- False-positive risk: low after exact residual-type and body-shape checks.
  Different error types can rely on `?` conversion, which a direct terminal
  call would not preserve.
- Suitability: do now.

### 2. `lints/complexity/manual_fallible_collect_loop`

- Misuse: a new standard collection fills with `transform(item)?` in a
  loop and returned inside the matching success variant.
- Why this matters: fallible `collect` preserves source order and stops at the
  first `Err` or `None` without a mutable collection binding.
- Evidence: the official
  [`Result` collection documentation](https://doc.rust-lang.org/stable/std/result/index.html#collecting-into-result),
  [`FromIterator`](https://doc.rust-lang.org/stable/std/iter/trait.FromIterator.html),
  and [`Iterator::collect`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.collect).
- Detection: extend the current `manual_iterator_loop` analysis. need a new
  `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `HashMap`, or `BTreeMap`; one
  insertion containing `?`; a following `Ok(collection)` or
  `Some(collection)` tail; and the same residual types.
- Suggested fix: use
  `items.into_iter().map(transform).collect::<Result<Vec<_>, _>>()` or the
  inferred equal. Use help-only unless all snippets and target types are
  exact.
- False-positive risk: low for one-statement bodies. Skip loops that inspect a
  partial collection, keep it on failure, or convert the residual type.
- Suitability: do now.

### 3. `lints/complexity/manual_extend_loop`

- Misuse: a loop only pushes or inserts each source item, or one pure
  transformation of it, into an existing standard collection.
- Why this matters: `extend` states bulk addition and removes manual iteration.
  It also lets the collection use the iterator's size hint.
- Evidence: [`Extend`](https://doc.rust-lang.org/stable/std/iter/trait.Extend.html)
  and
  [`Iterator::map`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.map).
- Detection: resolve both the standard loop source and a standard collection
  mutation method. need a one-statement body and prove that the target is
  not the source. Start with `Vec::push` and `VecDeque::push_back`; add map and
  set insertion only after semantic fixtures.
- Suggested fix: replace the loop with `output.extend(items)` or
  `output.extend(items.into_iter().map(transform))`.
- False-positive risk: low for standard sequence collections. Skip
  `String::extend(chars)` because Clippy correctly prefers `push_str` for a
  string source. Skip custom `Extend` implementations.
- Suitability: do now.

### 4. `lints/complexity/manual_partition_loop`

- Misuse: two new collections receive opposite branches of one predicate in a
  loop.
- Why this matters: `partition` names the two-way split and constructs both
  outputs without two mutable bindings.
- Evidence:
  [`Iterator::partition`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.partition).
- Detection: need two immediately preceding empty standard collections.
  need an `if` and `else` whose only operations insert the same loop item
  into opposite targets. Prove the returned tuple order matches the predicate's
  true and false outputs.
- Suggested fix: use
  `let (accepted, rejected): (Vec<_>, Vec<_>) = items.into_iter().partition(predicate);`.
- False-positive risk: low with exact item and target matching. Closure
  borrowing often prevents a source-only machine suggestion, so default to
  help-only.
- Suitability: do now.

### 5. `lints/complexity/manual_unzip_loop`

- Misuse: a loop destructures each pair and pushes its two members into two new
  collections.
- Why this matters: `unzip` names the pairwise projection and supports any two
  `Default + Extend` output types.
- Evidence:
  [`Iterator::unzip`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.unzip).
- Detection: need two immediately preceding empty standard collections, a
  two-field tuple loop pattern, and exactly one insertion into each matching
  target in source field order.
- Suggested fix: use
  `let (left, right): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();`.
- False-positive risk: low. Skip transformed fields, swapped output order, and
  loop bodies with other effects in the first implementation.
- Suitability: do now.

### 6. `lints/complexity/manual_filter_for_each_loop`

- Misuse: a loop contains only `if predicate(item) { action(item) }` with no
  `else` branch.
- Why this matters: `filter(...).for_each(...)` separates selection from the
  terminal action and matches the chain style style.
- Evidence:
  [`Iterator::filter`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.filter),
  [`Iterator::for_each`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.for_each),
  and Clippy's documented transformed-pipeline exception in `needless_for_each`.
- Detection: need a standard iterator loop, a one-branch `if`, and one
  terminal call or assignment. Reject control flow, `await`, `?`, macro
  expansion, and predicates that mutate values also used by the action.
- Suggested fix: use
  `items.into_iter().filter(|item| predicate(item)).for_each(action)`.
- False-positive risk: medium because closure argument borrowing can need
  dereference or destructuring changes. Keep the first version help-only.
- Suitability: do now as an opt-in personal style lint.

### 7. `lints/complexity/manual_filter_map_for_each_loop`

- Misuse: a loop calls an `Option`-returning transformation. Next, it runs one
  action only for `Some(mapped)`.
- Why this matters: `filter_map(...).for_each(...)` states conditional
  transformation without nested `if let` control flow.
- Evidence:
  [`Iterator::filter_map`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.filter_map)
  and
  [`Iterator::for_each`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.for_each).
- Detection: need `if let Some(mapped) = transform(item)` as the complete
  loop body, with one use of `mapped` in a single action. Resolve standard
  `Option` semantically and reject a converted `Result::ok` because that would
  hide errors.
- Suggested fix: use
  `items.into_iter().filter_map(transform).for_each(action)`.
- False-positive risk: low for the narrow shape. Use help-only when either
  closure cannot support exact rendering exactly.
- Suitability: do now as an opt-in personal style lint.

### 8. `lints/complexity/manual_passthrough_inspect`

- Misuse: code stores an `Option` or `Result`, performs a side effect for one
  variant by reference, and immediately returns the unchanged value.
- Why this matters: `inspect` and `inspect_err` keep observation inside the
  pipeline and make unchanged pass-through explicit.
- Evidence:
  [`Option::inspect`](https://doc.rust-lang.org/stable/std/option/enum.Option.html#method.inspect),
  [`Result::inspect`](https://doc.rust-lang.org/stable/std/result/enum.Result.html#method.inspect),
  and
  [`Result::inspect_err`](https://doc.rust-lang.org/stable/std/result/enum.Result.html#method.inspect_err).
- Detection: need an initializer, one adjacent `if let` or two-arm match,
  and the same binding as the block tail. need the observation arm to borrow
  its payload and return no replacement value. Prove the binding has no other
  uses.
- Suggested fix: replace the three-part sequence with
  `operation().inspect_err(log_error)` or the matching `inspect` call.
- False-positive risk: low. Drop timing remains sensitive when the branch
  creates temporaries, so machine suggestions must cover only a single
  direct call.
- Suitability: do now.

### 9. `lints/complexity/manual_option_take_if`

- Misuse: an `if` tests the current `Option` payload and calls `take` only when
  that predicate passes.
- Why this matters: `take_if` combines the conditional mutation and extraction
  without repeating the `Option` receiver.
- Evidence:
  [`Option::take_if`](https://doc.rust-lang.org/stable/std/option/enum.Option.html#method.take_if).
- Detection: resolve standard `Option`, the same mutable binding. A shape
  equal to
  `if option.as_ref().is_some_and(predicate) { option.take() } else { None }`.
  need that the predicate cannot mutate the `Option` binding through another
  path.
- Suggested fix: use `option.take_if(predicate)` with a closure adjusted for
  `&mut T`.
- False-positive risk: low for behavior, but the predicate's argument changes
  from a shared to mutable reference. Use help-only until type adjustment is
  proven exact.
- Suitability: do now.

### 10. `lints/complexity/manual_entry_update`

- Misuse: a direct `HashMap::entry` or `BTreeMap::entry` match modifies an
  occupied value and inserts a default into a vacant entry.
- Why this matters: `and_modify(...).or_insert_with(...)` expresses the full
  one-lookup update policy as a method chain.
- Evidence:
  [`HashMap::Entry::and_modify`](https://doc.rust-lang.org/stable/std/collections/hash_map/enum.Entry.html#method.and_modify)
  and
  [`BTreeMap::Entry::and_modify`](https://doc.rust-lang.org/stable/std/collections/btree_map/enum.Entry.html#method.and_modify).
- Detection: resolve the exact standard `Entry` enum and variants. need an
  occupied arm with one mutation through `get_mut` or `into_mut`, plus a vacant
  arm with one `insert`. Distinguish eager `or_insert` from lazy
  `or_insert_with` by initializer purity.
- Suggested fix: use
  `map.entry(key).and_modify(update).or_insert_with(default)`.
- False-positive risk: medium. Entry guards expose keys and occupied-entry
  operations that the short chain cannot always represent. Start with value-
  only arms and help-only diagnostics.
- Suitability: needs prototype.

### 11. `lints/complexity/consecutive_iterator_loops`

- Misuse: two adjacent loops apply the same body to compatible item types from
  two sources.
- Why this matters: `chain` expresses one ordered stream and lets later
  adapters or one terminal operation apply once.
- Evidence:
  [`Iterator::chain`](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.chain).
- Detection: compare resolved, expansion-free HIR for both bodies after mapping
  their loop bindings. need local-path or simple-borrow sources. Reject a
  first body that mutates, moves, or drops the second source.
- Suggested fix: use
  `first.into_iter().chain(second).for_each(|item| body(item))`.
- False-positive risk: medium. `chain` converts the second source before it
  consumes the first, while the second `for` loop converts its source later.
  This changes observable source-expression side effects and some borrow
  lifetimes. Keep the lint help-only and need inert source expressions.
- Suitability: needs prototype.

### 12. `lints/complexity/manual_adjacent_window_loop`

- Misuse: an index loop reads only `slice[index]` and `slice[index + 1]` over
  every valid adjacent pair.
- Why this matters: `windows(2)` removes index arithmetic and states that the
  algorithm consumes overlapping adjacent pairs.
- Evidence:
  [`slice::windows`](https://doc.rust-lang.org/stable/std/primitive.slice.html#method.windows).
- Detection: resolve a slice, array, or `Vec` receiver. Prove the range is
  `0..len.saturating_sub(1)` or has a guard for an empty slice. need all
  loop-index uses to be the two adjacent subscripts. Reject mutation and any
  retained window reference.
- Suggested fix: use
  `slice.windows(2).for_each(|window| action(&window[0], &window[1]))`.
- False-positive risk: medium. Rewriting `0..len - 1` without an existing
  nonempty proof would remove an empty-input panic, so that unguarded form must
  not receive the suggestion.
- Suitability: needs prototype.

## Parked ideas

- General `for` to `for_each`: park. Clippy's `needless_for_each` documents the
  opposite preference for an untransformed terminal loop. A blanket private
  lint would create permanent tool disagreement without adding structure.
- Arbitrary accumulator to `fold` or `scan`: park. The rewrite can change
  ownership, final-state visibility, borrow lifetimes. The meaning of
  `break` or `continue`. Clippy already covers common unnecessary folds and
  manual `try_fold` adapter chains.
- Nested loops to `flat_map`: park. Moving or borrowing an outer item into the
  inner iterator often makes a source rewrite fail to compile. A useful lint
  needs a proven local subset first.
- Index loops to `zip`: park. `zip` stops at the shorter input. An indexed loop
  can instead panic or use an explicit bound, so most apparent rewrites are not
  behavior preserving. Clippy already handles many simple range loops.
- `if let Some(value) { action(value) }` to
  `option.into_iter().for_each(action)`: park. The standard `Option`
  documentation explicitly presents pattern matching for one-off actions, and
  Clippy warns on `Option::map` or `Result::map` returning unit.
- Async loops to future combinators: park for the standard-library suite.
  `Future` has no stable combinator methods. A proposal must first name the
  ecosystem trait and version whose methods it would need.

## Recommended implementation order

do `manual_try_for_each_loop` first. Its exact body and residual-type
checks make it the best test of fallible loop analysis. do
`manual_fallible_collect_loop` next by reusing the existing
`manual_iterator_loop` accumulator and loop-desugaring helpers. Then do
`manual_extend_loop`, `manual_partition_loop`, and `manual_unzip_loop` as one
lint at a time with focused positive and negative UI fixtures.

Keep `manual_filter_for_each_loop` and
`manual_filter_map_for_each_loop` separate from the structural accumulator
lints. They encode a personal terminal-style preference and must remain easy
to enable or disable independently.
