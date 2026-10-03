# reqwest-retry-invalid-max-extra-load

## What it does

Checks `reqwest::retry::Builder::max_extra_load` calls whose statically known
`f32` value is below `0.0`, above `1000.0`, NaN, or infinite. It resolves local
constants and supported `f32` arithmetic.

## Why is this bad?

`max_extra_load` panics when its argument is outside `0.0..=1000.0`. The
program crashes when it builds the retry policy, even though the source contains
the bad value.

## Known problems

The lint resolves local non-trait `f32` constants, unary negation, and built-in
`+`, `-`, `*`, `/`, or `%` expressions through 16 nested steps. It also
recognizes `f32::NAN`, `f32::INFINITY`, and `f32::NEG_INFINITY`. The evaluator
uses `f32` precision, so it follows the value after `f32` rounding. Runtime
values, function calls, casts, statics, trait or other external constants,
overloaded operators, control flow, and unsupported operators remain unknown.

## Example

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").max_extra_load(-0.1)
}
```

## Use instead

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").max_extra_load(0.2)
}
```
