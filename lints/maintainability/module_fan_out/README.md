# module_fan_out

## What it does

Counts the distinct top-level local modules that each top-level module refers
to, and warns when there are more than 7.

A reference is a resolved path or method call to a definition in another
module of the same crate. Nested modules count as part of their top-level
ancestor. Items defined directly in the crate root count as one more module.
Repeated references to the same module count once. References to other crates
do not count. The limit of 7 is a local policy.

## Why is this bad?

Each outgoing dependency is one more reason the module may need to change. A
module that calls into parsing, storage, networking, formatting, and telemetry
breaks when any of them changes, and its tests need setup for all of them.

## Known problems

A composition root or `main` module can legitimately coordinate many modules.
A `pub use` facade does not lower the count, because paths resolve to the
original definition. A method call counts toward the module that defines the
method, which for a trait method is the module that defines the trait.
References inside macro invocations and dependencies created at run time, such
as trait objects or registration, are not counted. Coupling between nested
modules under one top-level module is not measured.

## Example

```rust
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
```

`coordinator` refers to 8 top-level modules.

## Use instead

Move related steps behind modules that each own one stage.

```rust
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
```
