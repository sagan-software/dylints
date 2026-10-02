#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks lossy widening of Bevy elapsed seconds.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;

#[cfg(test)]
use bevy_time as _;

bevy_support::declare_expression_span_suggestion_lint! {
    BEVY_TIME_ELAPSED_SECS_CAST_F64,
    BevyTimeElapsedSecsCastF64,
    Warn,
    bevy_support::elapsed_secs_cast_f64_span,
    "checks lossy widening of Bevy elapsed seconds",
    "casting `elapsed_secs()` to `f64` cannot recover its lost precision",
    "call `elapsed_secs_f64()` instead",
    "elapsed_secs_f64"
}
