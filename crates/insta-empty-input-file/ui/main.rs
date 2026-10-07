//! Compiler UI cases for empty Insta input-file settings.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_input_file as _;
use insta_support as _;

/// An empty current-crate input-file constant.
const EMPTY_INPUT: &str = "";

/// Exercise literal, local-binding, and unknown input-file cases.
#[expect(
    clippy::needless_late_init,
    reason = "Preserves the uninitialized local input control"
)]
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_input_file("");
    settings.set_input_file("API response");

    let empty_input = "";
    let input_alias = empty_input;
    settings.set_input_file(input_alias);

    let const_alias = EMPTY_INPUT;
    settings.set_input_file(const_alias);

    let shadowed_input = "";
    settings.set_input_file(shadowed_input);
    {
        let shadowed_input = "API response";
        settings.set_input_file(shadowed_input);
    }
    {
        let shadowed_input = "";
        settings.set_input_file(shadowed_input);
    }

    let mut changed_input = "";
    settings.set_input_file(changed_input);
    changed_input = "API response";
    settings.set_input_file(changed_input);

    let late_input;
    late_input = "";
    settings.set_input_file(late_input);

    let depth0 = "";
    let depth1 = depth0;
    let depth2 = depth1;
    let depth3 = depth2;
    let depth4 = depth3;
    let depth5 = depth4;
    let depth6 = depth5;
    let depth7 = depth6;
    settings.set_input_file(depth7);
    let depth8 = depth7;
    settings.set_input_file(depth8);

    settings.set_input_file(dynamic_input());

    // External constants and statics remain unknown to literal extraction.
    settings.set_input_file(std::env::consts::OS);
    settings.set_input_file(EXTERNAL_INPUT);
}

/// Return a runtime-selected path that string extraction must leave unknown.
const fn dynamic_input() -> &'static str {
    if std::hint::black_box(false) {
        ""
    } else {
        "API response"
    }
}

/// An external static input remains unknown to literal extraction.
static EXTERNAL_INPUT: &str = "";
