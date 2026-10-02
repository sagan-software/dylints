#![allow(dead_code)]

fn parse(raw: &str) -> bool {
    raw == "ok"
}

fn test_vec_cases() {
    for (raw, expected) in vec![("ok", true), ("bad", false), ("", false)] {
        assert_eq!(parse(raw), expected);
    }
}

fn test_nested_case_data() {
    let cases = [("ok", true), ("bad", false)];
    for (raw, expected) in cases {
        assert_eq!(parse(raw), expected);
    }
}

fn test_generated_cases() {
    for raw in ["ok", "bad"].iter().copied().filter(|raw| !raw.is_empty()) {
        let _ = parse(raw);
    }
}

static STATIC_CASES: [(&str, bool); 2] = [("ok", true), ("bad", false)];

fn test_static_cases() {
    for (raw, expected) in STATIC_CASES {
        assert_eq!(parse(raw), expected);
    }
}

struct CaseSet;

impl CaseSet {
    const CASES: [(&'static str, bool); 2] = [("ok", true), ("bad", false)];
}

fn test_associated_const_cases() {
    for (raw, expected) in CaseSet::CASES {
        assert_eq!(parse(raw), expected);
    }
}

fn test_repeated_literal_cases() {
    for (raw, expected) in [("ok", true); 2] {
        assert_eq!(parse(raw), expected);
    }
}

fn test_local_vec_for_each_cases() {
    let cases = vec![("ok", true), ("bad", false)];
    cases
        .into_iter()
        .for_each(|(raw, expected)| assert_eq!(parse(raw), expected));
}

fn test_dynamic_for_each_cases() {
    dynamic_cases()
        .into_iter()
        .for_each(|(raw, expected)| assert_eq!(parse(raw), expected));
}

fn dynamic_cases() -> Vec<(&'static str, bool)> {
    vec![("ok", true), ("bad", false)]
}

trait ArrayForEach {
    fn for_each(self, action: impl FnMut((&'static str, bool)));
}

impl ArrayForEach for [(&'static str, bool); 2] {
    fn for_each(self, mut action: impl FnMut((&'static str, bool))) {
        for case in self {
            action(case);
        }
    }
}

fn test_custom_for_each_lookalike() {
    [("ok", true), ("bad", false)].for_each(|(raw, expected)| assert_eq!(parse(raw), expected));
}

fn test_borrowed_alias_cases() {
    let base = [("ok", true), ("bad", false)];
    let cases = &base;
    for &(raw, expected) in cases {
        assert_eq!(parse(raw), expected);
    }
}

fn main() {}
