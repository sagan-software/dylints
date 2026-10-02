#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks zero-sized `SQLx` pools.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
use sqlx as _;

sqlx_support::declare_method_argument_lint! {
    SQLX_ZERO_MAX_CONNECTIONS,
    SqlxZeroMaxConnections,
    "PoolOptions",
    "max_connections",
    Zero,
    "a SQLx pool has zero maximum connections",
    "a zero-sized pool cannot acquire a connection",
    "configure at least one maximum connection"
}
