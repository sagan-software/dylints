# module-fan-out

## What it does

Counts the distinct top-level local modules that each top-level module refers
to, and warns when there are more than 7.

A reference is a resolved path or method call to a definition in another
module of the same crate. Nested modules count as part of their top-level
ancestor. Items defined directly in the crate root count as one more module.
Repeated references to the same module count once. References to other crates
do not count. The limit of 7 is a local policy.

## Why is this bad?

Each outgoing dependency is one more reason for a module to change. A
module that calls into parsing, storage, networking, formatting, and telemetry
breaks when any of them changes. Its tests need setup for all of them.

## Known problems

A composition root or `main` module can legitimately coordinate many modules.
A `pub use` facade does not lower the count, because paths resolve to the
original definition. A method call counts toward the module that defines the
method, which for a trait method is the module that defines the trait.
The lint does not count references inside macro invocations or dependencies created at run time, such as trait objects or registration. The lint does not measure coupling between nested modules under one top-level module.

## Example

```rust
# mod input { pub fn read() {} }
# mod policy { pub fn check() {} }
# mod storage { pub fn load() {} }
# mod auth { pub fn authorize() {} }
# mod schedule { pub fn plan() {} }
# mod output { pub fn write() {} }
# mod telemetry { pub fn record() {} }
# mod recovery { pub fn checkpoint() {} }
mod coordinator {
    pub fn run() {
        crate::input::read();
        crate::policy::check();
        crate::storage::load();
        crate::auth::authorize();
        crate::schedule::plan();
        crate::output::write();
        crate::telemetry::record();
        crate::recovery::checkpoint();
    }
}
# fn main() {}
```

`coordinator` refers to 8 top-level modules.

## Use instead

Move related steps behind modules that each own one stage.

```rust
# mod input { pub fn read() {} }
# mod policy { pub fn check() {} }
# mod storage { pub fn load() {} }
# mod auth { pub fn authorize() {} }
# mod schedule { pub fn plan() {} }
# mod output { pub fn write() {} }
# mod telemetry { pub fn record() {} }
# mod recovery { pub fn checkpoint() {} }
mod intake {
    pub fn run() {
        crate::input::read();
        crate::policy::check();
        crate::storage::load();
        crate::auth::authorize();
    }
}

mod delivery {
    pub fn run() {
        crate::schedule::plan();
        crate::output::write();
        crate::telemetry::record();
        crate::recovery::checkpoint();
    }
}

mod coordinator {
    pub fn run() {
        crate::intake::run();
        crate::delivery::run();
    }
}
# fn main() {}
```

## Interpretation and sources

Module fan-out counts distinct outgoing module dependencies. A high value can identify an orchestration module, while generated adapters and facades need review. Compare changes with accepted code before interpreting a threshold as architectural evidence. [Code Maat](https://github.com/adamtornhill/code-maat) supplies separate history-based coupling measurements; this compiler lint measures static dependencies.
