# module_dependency_cycle

## What it does

Checks for top-level local modules that depend on each other in a cycle, and
emits one warning per cycle that names every module in it.

A dependency is a resolved path or method call to a definition in another
module of the same crate. Nested modules count as part of their top-level
ancestor. The crate root is not part of the graph, because it declares and re-exports
every module.
The lint accepts the chain `input -> policy -> output`. Adding `output -> input`
makes it report all three modules.

## Why is this bad?

A cycle removes the dependency direction between modules. Every module in the
cycle depends on the others for reading, testing, and moving, and a change in
one can ripple around the whole cycle. Shared types tend to drift toward whichever
module is easiest to import, which adds more reverse edges over time.

## Known problems

The lint does not report a cycle that passes through items defined in the crate
root. It does not report cycles between nested modules under one top-level
module, so moving two cyclic modules under one parent silences the lint. The
lint does not count references inside macro invocations or dependencies created
at run time, such as trait objects or callbacks.

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
