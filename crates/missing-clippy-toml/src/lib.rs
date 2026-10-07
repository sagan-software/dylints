#![feature(rustc_private)]

//! A lint to check for missing clippy.toml files.
//!
//! It loads the package manifest for the current crate and follows Clippy's
//! configuration search from its selected directory through the filesystem
//! root. A missing file is reported at the package manifest's `[package]` header.

extern crate rustc_ast;
extern crate rustc_span;

use std::{env, path::PathBuf};

use cargo_support::{emit_with_help, package_manifest};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub MISSING_CLIPPY_TOML,
    Warn,
    "crates should have a Clippy configuration file",
    MissingClippyToml
}

/// Configuration file names Clippy checks in each directory, in priority order.
const CONFIG_FILE_NAMES: &[&str] = &[".clippy.toml", "clippy.toml"];

impl EarlyLintPass for MissingClippyToml {
    /// Check that Clippy's configuration search path contains a configuration file.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(header) = missing_config_header(cx, krate) {
            emit_with_help(
                cx,
                MISSING_CLIPPY_TOML,
                header,
                "crate has no Clippy configuration file",
                "add `clippy.toml` or `.clippy.toml` to a directory Clippy searches",
            );
        }
    }
}

/// Return the `[package]` header span when no Clippy configuration applies to the package.
fn missing_config_header(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Span> {
    let package = package_manifest(cx, krate)?;
    let header = package.entry_span("package")?;

    (!has_clippy_config()).then_some(header)
}

/// Return whether Clippy can find a configuration file from its selected start directory.
fn has_clippy_config() -> bool {
    // Clippy prefers CLIPPY_CONF_DIR, then CARGO_MANIFEST_DIR, then the current directory.
    let start = env::var_os("CLIPPY_CONF_DIR")
        .or_else(|| env::var_os("CARGO_MANIFEST_DIR"))
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok());
    has_clippy_config_from(start)
}

/// Return whether Clippy can find a configuration file from `start` through its ancestors.
fn has_clippy_config_from(start: Option<PathBuf>) -> bool {
    // Canonicalization makes parent traversal start from an absolute directory.
    let Some(mut directory) = start.and_then(|path| path.canonicalize().ok()) else {
        return false;
    };

    // Clippy checks each ancestor through the filesystem root and prefers .clippy.toml per directory.
    loop {
        if CONFIG_FILE_NAMES
            .iter()
            .any(|name| directory.join(name).is_file())
        {
            return true;
        }

        if !directory.pop() {
            return false;
        }
    }
}

/// Run the UI tests.
#[test]
fn ui() {
    let _lock = ui_tests::lock_ui_tests();
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod ui_tests {
    use std::{
        env,
        fs::File,
        io::Write,
        path::{Path, PathBuf},
        sync::Mutex,
    };
    use tempfile::TempDir;
    use tokio::process::Command;

    /// Environment variable that tells the child test process which package to compile.
    const FIXTURE_ENV: &str = "MISSING_CLIPPY_TOML_UI_FIXTURE";

    /// Expected output for a package that has no applicable Clippy configuration.
    const MISSING_CONFIG_STDERR: &str = "\
warning: crate has no Clippy configuration file
  --> $DIR/Cargo.toml:1:1
   |
LL | [package]
   | ^^^^^^^^^
   |
   = help: add `clippy.toml` or `.clippy.toml` to a directory Clippy searches
   = note: `#[warn(missing_clippy_toml)]` on by default

warning: 1 warning emitted

";

    /// Serializes UI tests because each one builds or runs the Dylint driver.
    pub(super) static UI_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Lock the UI test sequence and recover after an earlier assertion failure.
    pub(super) fn lock_ui_tests() -> std::sync::MutexGuard<'static, ()> {
        UI_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// One isolated package used to test Clippy configuration discovery.
    struct Fixture {
        /// Cargo package directory compiled by the UI test runner.
        package: TempDir,
        /// Temporary parent directory for the package and optional Clippy configuration.
        root: TempDir,
    }

    impl Default for Fixture {
        /// Create a fixture whose ancestors cannot satisfy Clippy config discovery.
        fn default() -> Self {
            // Keep the fixture outside the repository so its ancestors cannot satisfy discovery.
            let root = tempfile::tempdir().unwrap();
            let package = tempfile::tempdir_in(root.path()).unwrap();

            write_file(
                package.path().join("Cargo.toml"),
                "[package]\nname = \"missing-clippy-toml-env-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n\n[[bin]]\nname = \"missing-clippy-toml-env-fixture\"\npath = \"main.rs\"\n",
            );
            write_file(package.path().join("main.rs"), "fn main() {}\n");

            Self { package, root }
        }
    }

    impl Fixture {
        /// Return the package directory used as the compiler's `CARGO_MANIFEST_DIR`.
        fn package(&self) -> &Path {
            self.package.path()
        }

        /// Create a fixture directory for a Clippy config file.
        fn config_directory(&self) -> TempDir {
            tempfile::tempdir_in(self.root.path()).unwrap()
        }

        /// Write a Clippy configuration file into `directory`.
        fn write_configuration(directory: &Path, file_name: &str) {
            write_file(directory.join(file_name), "msrv = \"1.85.0\"\n");
        }

        /// Write the expected warning for this package into its UI output file.
        fn expect_missing_configuration_warning(&self) {
            write_file(
                self.package.path().join("main.stderr"),
                MISSING_CONFIG_STDERR,
            );
        }

        /// Assert that `directory` and every parent lack a Clippy configuration file.
        fn assert_no_configuration_ancestor(directory: &Path) {
            // Every ancestor must be empty to prove that this fixture lacks a config.
            for ancestor in directory.ancestors() {
                for file_name in super::CONFIG_FILE_NAMES {
                    let config = ancestor.join(file_name);
                    let config_path = config.display();
                    assert!(
                        !config.is_file(),
                        "isolated UI fixture found unexpected config at {config_path}"
                    );
                }
            }
        }
    }

    /// Write one fixture file through a file handle.
    fn write_file(path: PathBuf, contents: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
    }

    /// Run one isolated UI test process with Clippy environment values for `package`.
    fn run_isolated_fixture(package: &Path, clippy_conf_dir: Option<&Path>) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        // Set variables on the child so this test cannot affect other Rust test threads.
        let output = runtime.block_on(async {
            let executable = env::current_exe().unwrap();
            let mut command = Command::new(executable);
            let command = command
                .arg("--exact")
                .arg("ui_tests::run_fixture")
                .arg("--nocapture")
                .env(FIXTURE_ENV, package)
                .env("CARGO_MANIFEST_DIR", package)
                .env_remove("CLIPPY_CONF_DIR");
            let command = match clippy_conf_dir {
                Some(directory) => command.env("CLIPPY_CONF_DIR", directory),
                None => command,
            };
            command.output().await.unwrap()
        });
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "isolated UI fixture failed\n{stdout}\n{stderr}"
        );
    }

    /// Compile the requested UI package in the child process.
    #[test]
    fn run_fixture() {
        if let Some(package) = env::var_os(FIXTURE_ENV) {
            let package = PathBuf::from(package);
            dylint_testing::ui_test(env!("CARGO_PKG_NAME"), package);
        }
    }

    /// Verify that `CLIPPY_CONF_DIR` selects a config directory without a package-local
    /// file.
    #[test]
    fn clippy_conf_dir_finds_configuration() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();
        let config_dir = fixture.config_directory();

        // The config in the selected directory should satisfy discovery.
        Fixture::write_configuration(config_dir.path(), "clippy.toml");
        run_isolated_fixture(fixture.package(), Some(config_dir.path()));
    }

    /// Verify that Clippy searches parent directories of `CLIPPY_CONF_DIR`.
    #[test]
    fn clippy_conf_dir_searches_ancestors() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();
        let config_dir = fixture.config_directory();
        // Select a nested directory so Clippy must inspect its parent.
        let selected_directory = tempfile::tempdir_in(config_dir.path()).unwrap();

        // The config belongs in the parent, not the selected start directory.
        Fixture::write_configuration(config_dir.path(), "clippy.toml");
        run_isolated_fixture(fixture.package(), Some(selected_directory.path()));
    }

    /// Verify that Clippy searches parent directories of `CARGO_MANIFEST_DIR`.
    #[test]
    fn cargo_manifest_dir_searches_ancestors() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();
        let config_file = fixture.root.path().join("clippy.toml");

        // Only the package's parent contains a configuration file.
        write_file(config_file, "msrv = \"1.85.0\"\n");
        run_isolated_fixture(fixture.package(), None);
    }

    /// Verify that Clippy accepts a package-local `.clippy.toml` file.
    #[test]
    fn package_dot_clippy_toml_satisfies_lint() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();

        // A package-local `.clippy.toml` should satisfy the default search path.
        Fixture::write_configuration(fixture.package(), ".clippy.toml");
        run_isolated_fixture(fixture.package(), None);
    }

    /// Verify that the lint warns when the selected directory and its parents lack
    /// a config.
    #[test]
    fn clippy_conf_dir_without_configuration_warns() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();
        let config_dir = fixture.config_directory();

        // Ensure no ancestor can accidentally satisfy the selected search path.
        Fixture::assert_no_configuration_ancestor(config_dir.path());
        // The compiletest expectation must exist before the child compiler runs.
        fixture.expect_missing_configuration_warning();
        run_isolated_fixture(fixture.package(), Some(config_dir.path()));
    }

    /// Verify that a selected Clippy directory does not fall back to a package config.
    #[test]
    fn clippy_conf_dir_does_not_fall_back_to_package() {
        let _lock = lock_ui_tests();
        let fixture = Fixture::default();
        let config_dir = fixture.config_directory();

        // Keep a config in the lower-priority package directory as a control.
        Fixture::write_configuration(fixture.package(), "clippy.toml");
        // The selected directory and its ancestors must not provide a config.
        Fixture::assert_no_configuration_ancestor(config_dir.path());
        fixture.expect_missing_configuration_warning();
        run_isolated_fixture(fixture.package(), Some(config_dir.path()));
    }

    /// Verify that an invalid selected directory does not find a configuration file.
    #[test]
    fn invalid_selected_directory_has_no_configuration() {
        let fixture = Fixture::default();

        assert!(!super::has_clippy_config_from(Some(
            fixture.root.path().join("missing-directory")
        )));
    }
}
