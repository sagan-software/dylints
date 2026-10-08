#![feature(rustc_private)]

//! Documented wrappers around Dylint lint declaration macros.

extern crate rustc_driver as _;
extern crate rustc_span;

use dylint_linting as _;
use rustc_span::Symbol;
#[cfg(test)]
use tempfile as _;

mod optional;
#[cfg(feature = "rust-file-size")]
mod rust_file_size;

pub use self::optional::peel_standard_options;
#[cfg(feature = "rust-file-size")]
pub use self::rust_file_size::{RustFileSizeViolation, rust_file_size_violation};

/// Return whether a compiled crate is an internal helper or UI fixture.
///
/// Support and fixture crates expose public APIs to sibling lint crates, but
/// they are not application-facing surfaces. Repository-wide policy lints use
/// this predicate to avoid measuring those implementation-only contracts.
///
/// # Examples
///
/// ```rust
/// #![feature(rustc_private)]
/// # extern crate rustc_span;
/// use rustc_span::Symbol;
///
/// let classify_crate: fn(Symbol) -> bool = dylint_support::is_internal_support_crate;
/// let _ = classify_crate;
/// ```
#[must_use]
pub fn is_internal_support_crate(crate_name: Symbol) -> bool {
    let name = crate_name.as_str();
    name.ends_with("_support") || name.ends_with("_fixture")
}

/// Declare and register a pre-expansion lint with a documented pass type.
#[macro_export]
macro_rules! documented_pre_expansion_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $Pass:ident) => {
        dylint_linting::__maybe_exclude! {
            dylint_linting::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        dylint_linting::__maybe_mangle! {
            /// Helper for register lints analysis.
            #[doc = concat!("Register the `", stringify!($NAME), "` lint with Dylint.")]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                dylint_linting::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.register_pre_expansion_lint_pass(Box::new(|| Box::new($Pass)));
            }
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);

        #[doc = concat!("Pre-expansion lint pass for `", stringify!($NAME), "`.")]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $Pass;

        impl rustc_lint::LintPass for $Pass {
            /// Return the stable pass name used by rustc diagnostics.
            fn name(&self) -> &'static str {
                stringify!($Pass)
            }

            /// Return the lint declarations emitted by this pass.
            fn get_lints(&self) -> rustc_lint::LintVec {
                vec![$NAME]
            }
        }
    };
}

/// Declare and register an early lint with a documented pass type.
#[macro_export]
macro_rules! documented_early_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $Pass:ident) => {
        dylint_linting::__maybe_exclude! {
            dylint_linting::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        dylint_linting::__maybe_mangle! {
            /// Helper for register lints analysis.
            #[doc = concat!("Register the `", stringify!($NAME), "` lint with Dylint.")]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                dylint_linting::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.register_early_lint_pass(Box::new(|| Box::new($Pass)));
            }
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);

        #[doc = concat!("Early lint pass for `", stringify!($NAME), "`.")]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $Pass;

        impl rustc_lint::LintPass for $Pass {
            /// Return the stable pass name used by rustc diagnostics.
            fn name(&self) -> &'static str {
                stringify!($Pass)
            }

            /// Return the lint declarations emitted by this pass.
            fn get_lints(&self) -> rustc_lint::LintVec {
                vec![$NAME]
            }
        }
    };
}

/// Declare and register an early lint that constructs an explicitly named pass.
#[macro_export]
macro_rules! documented_early_lint_with_pass {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $Pass:ident, $pass:expr) => {
        dylint_linting::__maybe_exclude! {
            dylint_linting::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        dylint_linting::__maybe_mangle! {
            /// Helper for register lints analysis.
            #[doc = concat!("Register the `", stringify!($NAME), "` lint with Dylint.")]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                dylint_linting::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.register_early_lint_pass(Box::new(|| Box::new($pass)));
            }
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);

        #[doc = concat!("Early lint pass for `", stringify!($Pass), "`.")]
        impl rustc_lint::LintPass for $Pass {
            /// Return the stable pass name used by rustc diagnostics.
            fn name(&self) -> &'static str {
                stringify!($Pass)
            }

            /// Return the lint declarations emitted by this pass.
            fn get_lints(&self) -> rustc_lint::LintVec {
                vec![$NAME]
            }
        }
    };
}

/// Declare and register a late lint with a documented pass type.
#[macro_export]
macro_rules! documented_late_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $Pass:ident) => {
        dylint_linting::__maybe_exclude! {
            dylint_linting::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        dylint_linting::__maybe_mangle! {
            /// Helper for register lints analysis.
            #[doc = concat!("Register the `", stringify!($NAME), "` lint with Dylint.")]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                dylint_linting::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.register_late_lint_pass(Box::new(dylint_linting::__make_late_closure!($Pass)));
            }
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);

        #[doc = concat!("Late lint pass for `", stringify!($NAME), "`.")]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $Pass;

        impl rustc_lint::LintPass for $Pass {
            /// Return the stable pass name used by rustc diagnostics.
            fn name(&self) -> &'static str {
                stringify!($Pass)
            }

            /// Return the lint declarations emitted by this pass.
            fn get_lints(&self) -> rustc_lint::LintVec {
                vec![$NAME]
            }
        }
    };
}

/// Declare and register a late lint that constructs an explicitly named pass value.
#[macro_export]
macro_rules! documented_late_lint_with_pass {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $Pass:ident, $pass:expr) => {
        dylint_linting::__maybe_exclude! {
            dylint_linting::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        dylint_linting::__maybe_mangle! {
            /// Helper for register lints analysis.
            #[doc = concat!("Register the `", stringify!($NAME), "` lint with Dylint.")]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                dylint_linting::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.register_late_lint_pass(Box::new(dylint_linting::__make_late_closure!($pass)));
            }
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);

        impl rustc_lint::LintPass for $Pass {
            /// Return the stable pass name used by rustc diagnostics.
            fn name(&self) -> &'static str {
                stringify!($Pass)
            }

            /// Return the lint declarations emitted by this pass.
            fn get_lints(&self) -> rustc_lint::LintVec {
                vec![$NAME]
            }
        }
    };
}
