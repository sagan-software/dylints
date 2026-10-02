//! This executable fixture exercises public documentation thresholds across modules, functions,
//! types, fields, variants, traits, and methods. It also keeps private definitions, missing docs,
//! and sufficiently detailed comments in view so the lint proves its reachability and ownership
//! boundaries. The examples use ordinary source definitions because generated APIs have no local
//! documentation that an author can edit directly.

#![allow(dead_code)]

/// Public helpers.
pub mod short_module {}

/// This module owns stable value access for the fixture and groups the public operations that
/// callers use to read state. Its documentation names the responsibility, expected caller path,
/// durable behavior, and relationship between the exposed type and function so a reader can
/// decide where to begin without opening the implementation source.
pub mod documented_module {
    /// Return a stable snapshot of the current value so callers can compare later updates without
    /// retaining a lock or mutable reference to internal state.
    pub fn snapshot() -> u64 {
        0
    }
}

/// Return the current value.
pub fn current_value() -> u64 {
    0
}

/// Run this operation.
///
/// ```rust
/// let many_code_words = do_not_count_code_as_explanatory_prose();
/// ```
pub fn code_heavy_docs() {}

/// Brief public type.
pub struct Brief;

/// This type stores the stable configuration used by callers when they create a service. It
/// documents the role, construction boundary, retained state, and interpretation of each exposed
/// value so changes remain reviewable across crate boundaries.
pub struct Configuration {
    /// Timeout value.
    pub timeout_ms: u64,
}

/// Result states returned after an operation finishes. Each variant records one stable outcome so
/// callers can handle completion without parsing display text or depending on implementation
/// details that may change independently.
pub enum Outcome {
    /// Operation finished.
    Complete,
}

/// Service used by callers to read stable state. The type owns no external resources and keeps
/// each returned value independent from later mutations, which lets callers retain snapshots
/// without extending an internal borrow.
pub struct Service;

impl Service {
    /// Read state.
    pub fn read(&self) -> u64 {
        0
    }
}

/// Public behavior implemented by services that can describe their current stable state. The
/// trait keeps the returned description independent from the implementation so callers can log or
/// compare it without retaining a service borrow.
pub trait Describe {
    /// Describe state.
    fn describe(&self) -> String;
}

impl Describe for Service {
    fn describe(&self) -> String {
        String::new()
    }
}

mod private_parent {
    /// Short docs are acceptable because private ancestry keeps this function inside the crate.
    pub fn unreachable() {}
}

pub fn missing_documentation_is_owned_by_rustc() {}

fn main() {
    private_parent::unreachable();
}

/// Hidden helper.
#[doc(hidden)]
pub fn hidden_helper() {}

/// Hidden module.
#[doc(hidden)]
pub mod hidden_module {
    /// Hidden item.
    pub struct InsideHidden;
}

/// Short constant.
pub const LIMIT: u32 = 1;

/// Short trait.
pub trait Measure {
    /// Short associated constant.
    const UNIT: u32;

    /// Short associated type.
    type Output;
}

impl Brief {
    /// Short associated constant.
    pub const ZERO: u32 = 0;
}

unsafe extern "C" {
    /// Short foreign function.
    pub fn foreign_value() -> u32;
}
