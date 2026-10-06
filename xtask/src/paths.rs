//! Canonical report paths and source selection for coverage.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use crate::error::Error;

/// Normalize existing symlinks and lexical parent components in an absolute
/// path.
pub(crate) fn canonical(path: &Path, root: &Path) -> Result<PathBuf, Error> {
    // Relative selections use the workspace as their only base.
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    // Resolve existing prefixes before appending components that do not exist yet.
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                let _is_removed = normalized.pop();
            }
            Component::CurDir => {}
            // Resolve each existing component before applying its following parent.
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
                if normalized.exists() {
                    normalized = normalized.canonicalize()?;
                }
            }
        }
    }
    Ok(normalized)
}

/// Exclude compiler dependencies, generated outputs and deliberate UI fixtures.
pub(crate) fn is_source(path: &Path) -> bool {
    // Component comparisons avoid excluding a legitimate filename containing these words.
    !path.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some(
                ".cargo"
                    | ".rustup"
                    | ".rustup-dylint"
                    | "rustc"
                    | "target"
                    | "examples"
                    | "ui"
                    | "fixtures"
                    | "fixture"
            )
        )
    }) && !path.starts_with(Path::new("/").join("nix").join("store"))
}

/// Expand selected directories to canonical Rust source paths before any
/// cleanup.
pub(crate) fn sources(paths: &[PathBuf], root: &Path) -> Result<BTreeSet<PathBuf>, Error> {
    let mut sources = BTreeSet::new();
    // Validate each requested scope before returning any selection for cleanup.
    for path in paths {
        add_sources(path, root, &mut sources)?;
    }
    Ok(sources)
}

/// Expand one directory and reject a scope with no eligible Rust source.
fn add_sources(path: &Path, root: &Path, sources: &mut BTreeSet<PathBuf>) -> Result<(), Error> {
    // A missing directory cannot silently expand coverage to the whole workspace.
    let directory = canonical(path, root)?;
    let display = path.display();
    if !directory.is_dir() {
        return Err(Error::Invalid(format!(
            "--path must name an existing directory: {display}"
        )));
    }
    let count = collect_sources(&directory, root, sources)?;
    // An empty selection must not trigger cleanup or an unrestricted report.
    if count == 0 {
        return Err(Error::Invalid(format!(
            "--path must contain at least one selected Rust source file: {display}"
        )));
    }
    Ok(())
}

/// Collect source files from one scope and count eligible entries, including
/// aliases.
fn collect_sources(
    directory: &Path,
    root: &Path,
    sources: &mut BTreeSet<PathBuf>,
) -> Result<usize, Error> {
    let mut count = 0;
    // Existing source aliases contribute their canonical identity once.
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        let path = entry.path();
        if is_rust_source(path) {
            // Resolve symlink source files before retaining their identities.
            let _is_inserted = sources.insert(canonical(path, root)?);
            count += 1;
        }
    }
    Ok(count)
}

/// Recognize eligible source files independently of directory traversal.
fn is_rust_source(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|extension| extension == "rs") && is_source(path)
}

/// Reject report directories that would replace workspace or root build
/// outputs.
pub(crate) fn coverage_target(path: &Path, root: &Path) -> Result<PathBuf, Error> {
    let target = canonical(path, root)?;
    if root.starts_with(&target) || target == root.join("target") {
        return Err(Error::Invalid(
            "coverage target must name a dedicated coverage directory".to_owned(),
        ));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    /// Locally mirrored compiler sources are dependencies, not workspace code.
    #[test]
    fn excludes_local_toolchain_sources() {
        assert!(!super::is_source(Path::new(
            "/workspace/.rustup-dylint/toolchains/nightly/lib/rustlib/src/rust/library/std/src/lib.rs"
        )));
        assert!(!super::is_source(Path::new(
            "/workspace/.rustup/toolchains/nightly/lib/rustlib/src/rust/library/std/src/lib.rs"
        )));
        assert!(super::is_source(Path::new("/workspace/src/toolchain.rs")));
    }
    use super::{canonical, coverage_target, sources};
    use std::path::Path;

    /// Coverage rejects ancestors and the ordinary target directory.
    #[test_case::test_case("/", false; "filesystem root")]
    #[test_case::test_case("/workspace", false; "workspace ancestor")]
    #[test_case::test_case("/workspace/repo", false; "workspace root")]
    #[test_case::test_case("/workspace/repo/target", false; "root build directory")]
    #[test_case::test_case("target/coverage", true; "dedicated coverage directory")]
    fn coverage_target_boundaries(path: &str, is_accepted: bool) {
        let root = Path::new("/workspace/repo");
        assert_eq!(coverage_target(Path::new(path), root).is_ok(), is_accepted);
    }

    /// Source selection rejects empty directories and follows source-file
    /// symlinks.
    #[test]
    fn source_symlink_retains_canonical_identity() {
        let directory = tempfile::tempdir().unwrap();
        // An empty scope must fail before any generated output is replaced.
        let scope = directory.path().join("scope");
        std::fs::create_dir(&scope).unwrap();
        assert!(sources(std::slice::from_ref(&scope), directory.path()).is_err());
        // A source-file symlink retains the target file's canonical identity.
        let source = directory.path().join("source.rs");
        std::fs::write(&source, "fn main() {}\n").unwrap();
        std::os::unix::fs::symlink(&source, scope.join("alias.rs")).unwrap();
        let selected = sources(&[scope], directory.path()).unwrap();
        assert_eq!(
            selected,
            std::collections::BTreeSet::from([source.canonicalize().unwrap()])
        );
    }

    /// Parent components resolve after following an existing directory alias.
    #[test]
    fn normalizes_existing_symlink_before_parent() {
        let directory = tempfile::tempdir().unwrap();
        // Resolve the directory alias before interpreting the following parent.
        let real = directory.path().join("real/child");
        std::fs::create_dir_all(&real).unwrap();
        std::os::unix::fs::symlink(&real, directory.path().join("alias")).unwrap();
        let result = canonical(Path::new("alias/../file.rs"), directory.path()).unwrap();
        // Lexical popping before alias resolution would produce another path.
        assert_eq!(result, directory.path().join("real/file.rs"));
    }
}
