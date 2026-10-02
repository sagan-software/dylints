# module_dependency_cycle

## What it does

Checks for top-level local modules that depend on each other in a cycle, and
emits one warning per cycle that names every module in it.

A dependency is a resolved path or method call to a definition in another
module of the same crate. Nested modules count as part of their top-level
ancestor. Items defined directly in the crate root count as one more module.
The chain `input -> policy -> output` is accepted. Adding `output -> input`
reports all three modules.

## Why is this bad?

A cycle removes the dependency direction between modules. No module in the
cycle can be read, tested, or moved without the others, and a change in one can
ripple around the whole cycle. Shared types tend to drift toward whichever
module is easiest to import, which adds more reverse edges over time.

## Known problems

The crate root takes part in cycles. A crate root that re-exports
`parser::parse` while `parser` uses a type defined in the crate root forms a
cycle. Cycles between nested modules under one top-level module are not
reported, so moving two cyclic modules under one parent silences the lint.
References inside macro invocations and dependencies created at run time, such
as trait objects or callbacks, are not counted.

## Example

```rust
mod parser {
    pub fn parse() {
        crate::model::validate();
    }

    pub fn token_limit() -> usize {
        64
    }
}

mod model {
    pub fn validate() {
        let _limit = crate::parser::token_limit();
    }
}
```

`parser` and `model` depend on each other.

## Use instead

Move the shared definition to a module that both sides can depend on.

```rust
mod limits {
    pub fn token_limit() -> usize {
        64
    }
}

mod parser {
    pub fn parse() {
        crate::model::validate();
    }
}

mod model {
    pub fn validate() {
        let _limit = crate::limits::token_limit();
    }
}
```
