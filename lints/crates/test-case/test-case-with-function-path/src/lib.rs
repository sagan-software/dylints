#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks function paths passed to the test-case `with` validator.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves test-case validator paths, reports unsupported
//! function forms, and recommends a callable path with the expected signature.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_WITH_FUNCTION_PATH,
    TestCaseWithFunctionPath,
    WithFunctionPath,
    "a test-case with validator receives a function path",
    "`with` communicates an inline assertion closure",
    "use `using` for a named reusable validation function"
}
