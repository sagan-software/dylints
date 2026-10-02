#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty `SQLx` tuple lists.
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
    SQLX_EMPTY_PUSH_TUPLES,
    SqlxEmptyPushTuples,
    "QueryBuilder",
    "push_tuples",
    EmptyCollection,
    "SQLx QueryBuilder receives an empty tuple list",
    "an empty tuple list produces invalid SQL",
    "handle the empty collection before building the query"
}
