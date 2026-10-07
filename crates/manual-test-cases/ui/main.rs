// compile-flags: --test
#![feature(register_tool)]
#![register_tool(test_case)]
#![register_tool(fixture)]

type CaseTuple = (&'static str, bool);

fn test_inline_array_cases() {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }
}

fn test_local_array_cases() {
    let cases = [("ok", true), ("no", false)];

    for (raw, expected) in cases {
        assert_eq!(parse(raw), expected);
    }
}

fn test_aliased_local_array_cases() {
    let cases: [CaseTuple; 2] = [("ok", true), ("no", false)];

    for (raw, expected) in cases {
        assert_eq!(parse(raw), expected);
    }
}

#[test]
fn status_maps_to_projection() {
    struct Case {
        raw: &'static str,
        expected: bool,
    }

    let cases = [
        Case {
            raw: "ok",
            expected: true,
        },
        Case {
            raw: "no",
            expected: false,
        },
    ];

    for case in cases {
        assert_eq!(parse(case.raw), case.expected);
    }
}

fn test_local_const_cases() {
    const CASES: [CaseTuple; 2] = [("ok", true), ("no", false)];

    for (raw, expected) in CASES.iter() {
        assert_eq!(parse(raw), *expected);
    }
}

fn test_local_vec_cases() {
    let cases = vec![("ok", true), ("no", false)];

    for (raw, expected) in cases.iter() {
        assert_eq!(parse(raw), *expected);
    }
}

fn test_dynamic_cases() {
    let cases = generated_cases();

    for (raw, expected) in cases {
        assert_eq!(parse(raw), expected);
    }
}

fn test_shadowed_dynamic_cases() {
    let cases = [("ok", true), ("no", false)];

    {
        let cases = generated_cases();

        for (raw, expected) in cases {
            assert_eq!(parse(raw), expected);
        }
    }

    assert_eq!(cases.len(), 2);
}

#[test_case::test_case("ok", true)]
fn already_uses_qualified_test_case(raw: &str, expected: bool) {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }

    assert_eq!(parse(raw), expected);
}

#[test_case::fixture]
fn test_test_case_tool_lookalike_attr_still_warns() {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }
}

#[fixture::test]
fn helper_with_lookalike_test_attr() {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }
}

fn helper_over_cases() {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }
}

const MODULE_CASES: [CaseTuple; 2] = [("ok", true), ("no", false)];

fn test_module_const_cases() {
    for (raw, expected) in MODULE_CASES {
        assert_eq!(parse(raw), expected);
    }
}

fn test_inline_for_each_cases() {
    [("ok", true), ("no", false)]
        .iter()
        .copied()
        .for_each(|(raw, expected)| assert_eq!(parse(raw), expected));
}

fn generated_cases() -> Vec<(&'static str, bool)> {
    vec![("ok", true), ("no", false)]
}

fn parse(raw: &str) -> bool {
    raw == "ok"
}

fn main() {}
