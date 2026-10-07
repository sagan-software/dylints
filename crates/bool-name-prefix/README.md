# bool-name-prefix

## What it does

Checks the names of `bool` struct fields, local variables, function and method
parameters, functions and methods that return `bool`, and `bool` constants,
statics, and associated constants. It warns when the name does not identify a
predicate.

A snake_case name passes when it starts with a predicate word such as `is_`,
`has_`, `can_`, `should_`, `does_`, `contains_`, `needs_`, or `uses_`, or
contains one such as `_is_`, `_has_`, or `_are_`. A constant or static name
passes when it starts with `IS_` or `HAS_`.

## Why is this bad?

A name like `ready` or `enabled` can be a flag, a count, a state enum, or a
callback. Reading `if config.enabled` does not tell the reader whether the
value is a `bool`. A predicate name such as `is_enabled` makes the type and the
meaning clear at the call site.

## Known problems

This repository uses a long, repository-specific list of accepted names. Names
that start with words such as `default_`, `from_`, `in_`, or `path_` pass. Names
that end with `_name`, `_type`, `_value`, or `_path`, and a few exact names such
as `value`, `seen`, and `escaped` pass without a predicate word.

The lint skips method and associated constant names in trait implementations,
closure parameters, destructuring patterns, `_` placeholders, and names that
macros create.

## Example

```rust
struct Job {
    ready: bool,
}

fn ready(enabled: bool) -> bool {
    let cached = enabled;
    cached
}

const ENABLED: bool = true;
```

## Use instead

```rust
struct Job {
    is_ready: bool,
}

fn is_ready(is_enabled: bool) -> bool {
    let is_cached = is_enabled;
    is_cached
}

const IS_ENABLED: bool = true;
```
