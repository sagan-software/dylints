//! Catalog, book and report assembly for one Pages artifact.

use std::{fs, path::Path, process::Command};

use crate::{error::Error, process};

/// Generate public documentation and copy reports that were produced by
/// validation.
pub(crate) fn generate(root: &Path, destination: &Path) -> Result<(), Error> {
    fs::create_dir_all(destination)?;
    // Generate both documentation surfaces before copying validation reports.
    catalog(root, destination)?;
    book(root, destination)?;
    reports(root, destination)
}

/// Discover native lint names and render the catalog from the registered
/// inventory.
fn catalog(root: &Path, destination: &Path) -> Result<(), Error> {
    // Discovery rejects an unloadable category instead of publishing stale names.
    let registered = process::output(
        Command::new("cargo")
            .current_dir(root)
            .args(["dylint", "list", "--all"]),
    )?;
    // Store the inventory for the catalog generator and later inspection.
    let list = root.join("target/registered-lints.txt");
    fs::create_dir_all(root.join("target"))?;
    fs::write(&list, registered.stdout)?;
    // Render the complete HTML catalog through its typed Askama contexts.
    process::run(
        Command::new("cargo")
            .current_dir(root)
            .args(["run", "-p", "sagan-lints-web", "--", "--lint-list"])
            .arg(&list)
            .arg("--out-dir")
            .arg(destination),
    )
}

/// Build the repository guide under the catalog's book path.
fn book(root: &Path, destination: &Path) -> Result<(), Error> {
    let mut command = Command::new("mdbook");
    let _configured = command
        .current_dir(root)
        .arg("build")
        .arg(root)
        .arg("--dest-dir")
        .arg(destination.join("book"));
    process::run(&mut command)
}

/// Include existing reports without following paths outside their generated
/// directories.
fn reports(root: &Path, destination: &Path) -> Result<(), Error> {
    // Reports are included only when their validation commands produced them.
    for (source, name) in [
        ("target/coverage/report", "coverage"),
        ("target/criterion", "benches"),
    ] {
        // Each copied tree remains rooted in its generated source directory.
        let source = root.join(source);
        if source.is_dir() {
            copy_tree(&source, &destination.join(name))?;
        }
    }
    Ok(())
}

/// Copy a generated directory without following symlinks into unrelated files.
fn copy_tree(source: &Path, destination: &Path) -> Result<(), Error> {
    // WalkDir excludes symlink contents unless explicitly configured to follow them.
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        // Preserve relative directory names while skipping every symlink entry.
        let output = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&output)?;
        } else if entry.file_type().is_file() {
            let _bytes = fs::copy(entry.path(), output)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Build a nested report with an external symlink that copying must ignore.
    fn source_tree(root: &std::path::Path) -> std::path::PathBuf {
        // The regular file is copyable; the symlink must never escape this tree.
        let source = root.join("source");
        std::fs::create_dir_all(source.join("nested")).expect("nested fixture");
        std::fs::write(source.join("nested/index.html"), "report").expect("report file");
        std::os::unix::fs::symlink(root, source.join("outside")).expect("external link");
        source
    }

    /// Copies files and nested directories while excluding symlink content.
    #[test]
    fn report_copy_does_not_follow_symlinks() {
        // Copy one isolated tree before observing its files and excluded link.
        let root = tempfile::tempdir().expect("report fixture");
        let source = source_tree(root.path());
        let output = root.path().join("destination");
        super::copy_tree(&source, &output).expect("copy report");
        assert_eq!(
            std::fs::read_to_string(output.join("nested/index.html")).expect("copied report"),
            "report"
        );
        // Missing input remains an error rather than an empty successful report.
        assert!(!output.join("outside").exists());
        assert!(super::copy_tree(&root.path().join("missing"), &output).is_err());
    }

    /// Optional reports are omitted until generated, then copied to public
    /// routes.
    #[test]
    fn report_assembly_tracks_generated_artifacts() {
        // Missing reports create no public report directories.
        let directory = tempfile::tempdir().expect("report assembly fixture");
        let root = directory.path();
        let destination = root.join("public");
        super::reports(root, &destination).expect("missing reports are optional");
        assert!(!destination.exists());
        // Generated coverage and benchmark trees must both become public routes.
        fixture_generated_reports(root);
        super::reports(root, &destination).expect("copy generated reports");
        assert!(destination.join("coverage/index.html").exists());
        // The benchmark route has an independent artifact assertion.
        assert!(destination.join("benches/index.html").exists());
    }

    /// Populate both generated report inputs without placing a case loop in a
    /// test.
    fn fixture_generated_reports(root: &std::path::Path) {
        for (source, route) in [
            ("target/coverage/report", "coverage"),
            ("target/criterion", "benches"),
        ] {
            std::fs::create_dir_all(root.join(source)).expect("generated report directory");
            std::fs::write(root.join(source).join("index.html"), route).expect("generated report");
        }
    }
}
