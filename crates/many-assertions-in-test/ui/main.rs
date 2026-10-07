// compile-flags: --test

#[test]
fn test_renders_report() {
    let report = "Summary Total Details Footer";

    assert!(report.contains("Summary"));
    assert!(report.contains("Total"));
    assert!(report.contains("Details"));
    assert!(report.contains("Footer"));
}

#[test]
fn focused_assertion() {
    assert_eq!(1, 1);
}

#[test]
fn test_assertions_nested_in_statements() {
    let report = ("Summary", 4, true, "Footer");

    // Nested statement layout must not hide assertion invocations.
    if report.2 {
        assert_eq!(report.0, "Summary");
        assert_eq!(report.1, 4);
    }
    if report.2 {
        assert!(report.1 > 0);
        assert_eq!(report.3, "Footer");
    }
}

fn test_named_helper_is_not_a_test() {
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(1, 2);
    debug_assert!(true);
}

fn main() {}
