#![feature(rustc_private)]

//! A lint to check for bare rust-toolchain files.
//!
//! It walks from the crate root file's directory toward the file system root
//! and reports the first `rust-toolchain` file without the `.toml` extension.
//! The diagnostic points at the start of that file, or at the crate root when
//! rustc cannot load the file as text.

extern crate rustc_ast;

use cargo_support::{crate_root_path, emit_with_help, file_start_span};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub RUST_TOOLCHAIN_TOML,
    Warn,
    "`rust-toolchain` file should use the `.toml` extension",
    RustToolchainToml
}

impl EarlyLintPass for RustToolchainToml {
    /// Check the crate root's ancestors for a bare `rust-toolchain` file.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(toolchain) = crate_root_path(cx, krate).and_then(|crate_root| {
            // A toolchain file applies to every descendant directory.
            crate_root
                .ancestors()
                .skip(1)
                .map(|directory| directory.join("rust-toolchain"))
                .find(|candidate| candidate.is_file())
        }) {
            // A file that is not valid UTF-8 cannot be shown, so report at the crate root instead.
            let span = file_start_span(cx, &toolchain).unwrap_or(krate.spans.inner_span);

            emit_with_help(
                cx,
                RUST_TOOLCHAIN_TOML,
                span,
                "repository uses `rust-toolchain` instead of `rust-toolchain.toml`",
                "rename `rust-toolchain` to `rust-toolchain.toml`",
            );
        }
    }
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
