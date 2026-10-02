#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks panicking `SQLx` statement column access.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
use sqlx as _;

sqlx_support::declare_method_argument_lint! {
    SQLX_PANICKING_STATEMENT_COLUMN,
    SqlxPanickingStatementColumn,
    "Statement",
    "column",
    Any,
    "SQLx Statement::column can panic",
    "`Statement::column` panics for an invalid index",
    "use `try_column`"
}
