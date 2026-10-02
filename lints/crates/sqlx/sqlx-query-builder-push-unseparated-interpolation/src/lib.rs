#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks interpolated unseparated `SQLx` query builder pushes.
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
    SQLX_QUERY_BUILDER_PUSH_UNSEPARATED_INTERPOLATION,
    SqlxQueryBuilderPushUnseparatedInterpolation,
    "Separated",
    "push_unseparated",
    FormattedSql,
    "formatted text is appended unseparated to a SQLx QueryBuilder",
    "`format!` inserts values directly into SQL text",
    "append fixed SQL and pass values with `push_bind`"
}
