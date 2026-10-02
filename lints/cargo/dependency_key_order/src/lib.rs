#![feature(rustc_private)]

//! A lint to check for alphabetically sorted Cargo dependency entries.
//!
//! It parses the nearest `Cargo.toml` above the crate root with `toml_edit`,
//! orders each dependency table's entries by source position, and compares
//! adjacent entries in the same block. Blank lines and comment lines start a
//! new block, so manifests can keep independently sorted groups.

extern crate rustc_ast;

use std::{cmp::Ordering, ops::Range};

use cargo_support::{
    Dependency, Manifest, dependencies, dependency_tables, emit_with_help, package_manifest,
    toml_edit::TableLike, workspace_dependency_table,
};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub DEPENDENCY_KEY_ORDER,
    Warn,
    "Cargo dependency entries in the same block should be sorted alphabetically",
    DependencyKeyOrder
}

impl EarlyLintPass for DependencyKeyOrder {
    /// Check every dependency table of the crate's manifest for unsorted entries.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(manifest) = package_manifest(cx, krate) {
            check_manifest(cx, &manifest);
        }
    }
}

/// Report every out-of-order entry in the manifest's dependency tables.
fn check_manifest(cx: &EarlyContext<'_>, manifest: &Manifest) {
    // Workspace dependencies declare the versions members inherit, so order them too.
    let root = manifest.root();
    for table in workspace_dependency_table(root)
        .into_iter()
        .chain(dependency_tables(root))
    {
        for OrderIssue {
            name,
            previous,
            key,
        } in order_issues(manifest.text(), table)
        {
            emit_with_help(
                cx,
                DEPENDENCY_KEY_ORDER,
                manifest.span(key),
                &format!(
                    "dependency `{name}` appears after `{previous}` but should sort before it"
                ),
                "sort contiguous dependency entries alphabetically within each Cargo dependency table",
            );
        }
    }
}

/// One dependency entry with the source positions the ordering check needs.
struct Entry<'a> {
    /// Dependency name with TOML quoting removed.
    name: &'a str,
    /// Byte range of the entry's key.
    key: Range<usize>,
    /// Byte offset just past the entry's last value.
    end: usize,
}

/// An entry that sorts before the entry directly above it.
#[derive(Debug, PartialEq, Eq)]
struct OrderIssue<'a> {
    /// Name of the out-of-order dependency.
    name: &'a str,
    /// Name of the entry directly above it.
    previous: &'a str,
    /// Byte range of the out-of-order key.
    key: Range<usize>,
}

/// Return the out-of-order entries of one dependency table.
fn order_issues<'a>(text: &str, table: &'a dyn TableLike) -> Vec<OrderIssue<'a>> {
    // Compare entries in source order, which can differ from the table's key order.
    let mut entries: Vec<_> = dependencies(table).filter_map(entry).collect();
    entries.sort_by_key(|entry| entry.key.start);

    entries
        .iter()
        .zip(entries.iter().skip(1))
        .filter(|(previous, current)| {
            !starts_block(text, previous.end, current.key.start)
                && current
                    .name
                    .bytes()
                    .map(|byte| byte.to_ascii_lowercase())
                    .cmp(previous.name.bytes().map(|byte| byte.to_ascii_lowercase()))
                    == Ordering::Less
        })
        .map(|(previous, current)| OrderIssue {
            name: current.name,
            previous: previous.name,
            key: current.key.clone(),
        })
        .collect()
}

/// Return the ordering entry for a dependency written inside its table.
///
/// Standard subtables such as `[dependencies.serde]` live elsewhere in the
/// file, so they are not part of the table's ordering.
fn entry(dependency: Dependency<'_>) -> Option<Entry<'_>> {
    if dependency
        .item
        .as_table()
        .is_some_and(|table| !table.is_dotted())
    {
        return None;
    }

    Some(Entry {
        name: dependency.name,
        key: dependency.key.span()?,
        // A dotted table's own span covers only its first key, so use its last value.
        end: dependency
            .item
            .as_table_like()
            .filter(|fields| fields.is_dotted())
            .map_or_else(
                || dependency.item.span().map(|span| span.end),
                |fields| {
                    fields
                        .get_values()
                        .into_iter()
                        .filter_map(|(_, value)| value.span())
                        .map(|span| span.end)
                        .max()
                },
            )?,
    })
}

/// Return whether a blank or comment line separates two entries.
fn starts_block(text: &str, previous_end: usize, key_start: usize) -> bool {
    let Some(between) = text.get(previous_end..key_start) else {
        return false;
    };

    // Skip the rest of the previous entry's line, which may hold a trailing comment.
    let Some((_, after_previous)) = between.split_once('\n') else {
        return false;
    };

    // Drop the indentation in front of the key; what remains are whole lines.
    let Some((lines, _)) = after_previous.rsplit_once('\n') else {
        return false;
    };

    lines.split('\n').any(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with('#')
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests for the ordering rules.

    use cargo_support::toml_edit::Document;

    use super::{OrderIssue, order_issues, starts_block};

    /// Return `(name, previous)` pairs for every issue in the `[dependencies]` table.
    fn issues(manifest: &str) -> Vec<(String, String)> {
        let document = Document::parse(manifest.to_owned()).unwrap();
        let table = document
            .as_table()
            .get("dependencies")
            .and_then(|item| item.as_table_like())
            .unwrap();

        order_issues(document.raw(), table)
            .into_iter()
            .map(|OrderIssue { name, previous, .. }| (name.to_owned(), previous.to_owned()))
            .collect()
    }

    /// Adjacent unsorted entries warn; separators and case differences do not.
    #[test]
    fn compares_adjacent_entries_within_blocks() {
        let manifest = r#"
[dependencies]
Beta = "1"
alpha = "1" # trailing comments do not separate blocks
zeta.version = "1"
zeta.features = [
  # comments inside a value do not separate blocks
  "x",
]
gamma = { version = "1", features = ["a#b"] }

omega = "1"
# comment lines separate blocks
delta = "1"

[dependencies.aaa]
version = "1"
"#;

        assert_eq!(
            issues(manifest),
            [
                ("alpha".to_owned(), "Beta".to_owned()),
                ("gamma".to_owned(), "zeta".to_owned()),
            ]
        );
    }

    /// Inline dependency tables are one block.
    #[test]
    fn inline_tables_are_one_block() {
        assert_eq!(
            issues("dependencies = { c.path = \"c\", c.version = \"1\", b = \"1\", a = \"1\" }"),
            [
                ("b".to_owned(), "c".to_owned()),
                ("a".to_owned(), "b".to_owned())
            ]
        );
    }

    /// Out-of-range offsets never separate entries.
    #[test]
    fn invalid_ranges_do_not_start_blocks() {
        assert!(!starts_block("ab", 2, 1));
    }
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
