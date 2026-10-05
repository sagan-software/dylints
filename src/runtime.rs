//! Compiler runtime and workspace configuration selected for the unified lint executable.
//!
//! Dylint 6.0.3 reads workspace configuration before registering lint passes.
//! Its `DYLINT_TOML` override accepts the original source text and tracks that
//! environment value in Cargo dependency information.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{cli::Cli, error::RunnerError};

/// Exact compiler assets used by Clippy and the embedded rustc driver.
#[derive(Clone, Debug)]
pub(super) struct Runtime {
    /// Cargo from the compiler sysroot that built this executable.
    pub(super) cargo: PathBuf,
    /// rustc from the compiler sysroot that built this executable.
    pub(super) rustc: PathBuf,
    /// Compiler sysroot containing `rustc_driver` and its matching standard library.
    pub(super) sysroot: PathBuf,
    /// Pinned toolchain identity shown in diagnostics and dry runs.
    pub(super) toolchain_name: &'static str,
}

/// Destination-host sysroot supplied by the skill launcher.
const SYSROOT_ENV: &str = "SAGAN_LINTS_SYSROOT";
/// Exact nightly used to build every native bundle executable.
const TOOLCHAIN_NAME: &str = env!("SAGAN_LINTS_TOOLCHAIN");
/// Build-host fallback retained for direct `cargo install` users.
const BUILD_SYSROOT: &str = env!("SAGAN_LINTS_BUILD_SYSROOT");

impl Runtime {
    /// Load one matching destination-host toolchain and reject missing assets.
    pub(super) fn load() -> Result<Self, RunnerError> {
        // The launcher overrides the build path after resolving the pinned local nightly.
        let sysroot = sysroot()?;
        // Both tools must come from the same sysroot as the linked rustc driver.
        let executable_suffix = env::consts::EXE_SUFFIX;
        let cargo = required_file(
            sysroot
                .join("bin")
                .join(format!("cargo{executable_suffix}")),
        )?;
        let rustc = required_file(
            sysroot
                .join("bin")
                .join(format!("rustc{executable_suffix}")),
        )?;

        // Keep the resolved paths owned so child phases cannot observe a later environment change.
        Ok(Self {
            cargo,
            rustc,
            sysroot,
            toolchain_name: TOOLCHAIN_NAME,
        })
    }

    /// Return the directory prepended for Cargo, rustc, Clippy, and rustdoc discovery.
    pub(super) fn toolchain_bin(&self) -> PathBuf {
        self.sysroot.join("bin")
    }
}

/// Return the compiler sysroot to internal compiler-wrapper processes.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its launcher boundary"
)]
pub(super) fn sysroot() -> Result<PathBuf, RunnerError> {
    // Keep one resolution rule for the parent runner and every re-entered compiler process.
    let path = selected_sysroot(env::var_os(SYSROOT_ENV));
    if path.is_dir() {
        Ok(path)
    } else {
        Err(RunnerError::MissingAsset { path })
    }
}

/// Prefer the destination-host path while retaining direct Cargo installation behavior.
fn selected_sysroot(runtime_sysroot: Option<OsString>) -> PathBuf {
    runtime_sysroot.map_or_else(|| PathBuf::from(BUILD_SYSROOT), PathBuf::from)
}

/// Return a required toolchain executable after validating it.
fn required_file(path: PathBuf) -> Result<PathBuf, RunnerError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(RunnerError::MissingAsset { path })
    }
}

/// Maximum configuration bytes copied into each compiler process environment.
const MAX_CONFIGURATION_BYTES: usize = 8 * 1024;

/// Retain inherited overrides and discovery selected by custom Cargo arguments.
pub(super) fn should_preload(
    args: &Cli,
    inherited: Option<&OsStr>,
    wrapper: Option<&OsStr>,
) -> bool {
    args.cargo_cmd.is_none()
        && args.extra_cargo_arg.iter().all(|argument| {
            matches!(
                argument.as_str(),
                "--tests" | "--lib" | "--bins" | "--locked" | "--offline"
            )
        })
        && args.list_private_lints.is_none()
        && args.skip_dylint.is_none()
        && inherited.is_none()
        && wrapper.is_none()
}

/// Resolve an existing workspace configuration with the selected Cargo
/// executable.
pub(super) fn load_configuration(
    cargo: &Path,
    repo: &Path,
    manifest: Option<&Path>,
) -> Option<String> {
    // Cargo handles workspace membership, including an explicit package.workspace path.
    // https://doc.rust-lang.org/cargo/commands/cargo-locate-project.html
    let mut command = Command::new(cargo);
    let _configured =
        command
            .current_dir(repo)
            .args(["locate-project", "--workspace", "--message-format=json"]);
    if let Some(manifest) = manifest {
        let _configured = command.arg("--manifest-path").arg(manifest);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    read_configuration(&output.stdout)
}

/// Preserve configuration source text, including malformed TOML, for Dylint to
/// parse.
fn read_configuration(location: &[u8]) -> Option<String> {
    let directory = manifest_directory(location)?;
    // Missing or unreadable files retain upstream discovery and its error behavior.
    let source = fs::read_to_string(directory.join("dylint.toml")).ok()?;
    // Bound environment growth and retain upstream parsing for text that cannot enter an environment.
    (source.len() <= MAX_CONFIGURATION_BYTES && !source.contains('\0')).then_some(source)
}

/// Decode Cargo's workspace manifest location without guessing its directory.
fn manifest_directory(location: &[u8]) -> Option<PathBuf> {
    // Decode Cargo's JSON before interpreting its absolute manifest path.
    let location: serde_json::Value = serde_json::from_slice(location).ok()?;
    Path::new(location.get("root")?.as_str()?)
        .parent()
        .map(Path::to_owned)
}

#[cfg(test)]
mod tests {
    use crate::cli::Cli;
    use clap::Parser as _;
    use std::{
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
    };
    use test_case::test_case;

    use super::{
        BUILD_SYSROOT, MAX_CONFIGURATION_BYTES, load_configuration, read_configuration,
        selected_sysroot, should_preload,
    };

    /// Prove a relocated bundle never retains the build host's absolute sysroot.
    #[test]
    fn destination_sysroot_overrides_build_sysroot() {
        let destination = OsString::from("/destination/nightly");

        // Preserve direct installs while ensuring the launcher can relocate a bundled executable.
        assert_eq!(
            selected_sysroot(Some(destination)),
            PathBuf::from("/destination/nightly")
        );
        assert_eq!(selected_sysroot(None), PathBuf::from(BUILD_SYSROOT));
    }
    /// Every opt-out retains discovery; known target-selection flags can share
    /// configuration.
    #[test_case(&[], None, None, true; "default")]
    #[test_case(&["--cargo-cmd", "cargo"], None, None, false; "custom cargo")]
    #[test_case(&["--extra-cargo-arg=--manifest-path=other/Cargo.toml"], None, None, false; "custom manifest")]
    #[test_case(&["--list-private-lints"], None, None, false; "listing")]
    #[test_case(&["--skip-dylint"], None, None, false; "skip")]
    #[test_case(&["--extra-cargo-arg=--tests"], None, None, true; "tests")]
    #[test_case(&["--extra-cargo-arg=--lib"], None, None, true; "library")]
    #[test_case(&["--extra-cargo-arg=--bins"], None, None, true; "binaries")]
    #[test_case(&["--extra-cargo-arg=--locked"], None, None, true; "locked")]
    #[test_case(&["--extra-cargo-arg=--offline"], None, None, true; "offline")]
    #[test_case(&[], Some(""), None, false; "empty override")]
    #[test_case(&[], Some("[lint]\nlimit = 5\n"), None, false; "configured override")]
    #[test_case(&[], None, Some("custom-wrapper"), false; "custom rustc wrapper")]
    fn preserves_discovery_selection_and_inherited_overrides(
        flags: &[&str],
        inherited: Option<&str>,
        wrapper: Option<&str>,
        is_expected: bool,
    ) {
        // Parse the same argument boundary used by the runner.
        let args = Cli::try_parse_from(std::iter::once("sagan-lints").chain(flags.iter().copied()))
            .unwrap();
        assert_eq!(
            should_preload(
                &args,
                inherited.map(std::ffi::OsStr::new),
                wrapper.map(std::ffi::OsStr::new)
            ),
            is_expected
        );
    }

    /// Environment limits preserve discovery for oversized and NUL-containing
    /// configurations.
    #[test_case(" ".repeat(MAX_CONFIGURATION_BYTES), true; "maximum")]
    #[test_case(" ".repeat(MAX_CONFIGURATION_BYTES + 1), false; "oversized")]
    #[test_case("\0".to_owned(), false; "nul")]
    fn bounds_environment_payload(source: String, is_accepted: bool) {
        // Write the exact payload at the resolved workspace boundary.
        let directory = tempfile::tempdir().unwrap();
        let location = configuration_location(directory.path());
        fs::write(directory.path().join("dylint.toml"), source).unwrap();
        assert_eq!(read_configuration(&location).is_some(), is_accepted);
    }

    /// Reading preserves source text for Dylint's own parser.
    #[test_case(""; "empty")]
    #[test_case("[lint]\nlimit = 7\n"; "configured")]
    #[test_case("[malformed\n"; "malformed")]
    fn preserves_configuration_source(source: &str) {
        // Compare bytes without normalizing malformed or valid TOML.
        let directory = tempfile::tempdir().unwrap();
        let location = configuration_location(directory.path());
        fs::write(directory.path().join("dylint.toml"), source).unwrap();
        assert_eq!(read_configuration(&location).as_deref(), Some(source));
    }

    /// Encode the same manifest location returned by Cargo.
    fn configuration_location(root: &Path) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"root": root.join("Cargo.toml")})).unwrap()
    }

    /// Missing, non-text, and unreadable configurations retain original discovery.
    #[test]
    fn preserves_configuration_read_failures() {
        let directory = tempfile::tempdir().unwrap();
        let location = serde_json::to_vec(&serde_json::json!({
            "root": directory.path().join("Cargo.toml")
        }))
        .unwrap();
        assert_eq!(read_configuration(&location), None);
        // Invalid UTF-8 must retain compiler-side reading.
        fs::write(directory.path().join("dylint.toml"), [0xff]).unwrap();
        assert_eq!(read_configuration(&location), None);
        // A directory cannot be read as configuration text.
        fs::remove_file(directory.path().join("dylint.toml")).unwrap();
        fs::create_dir(directory.path().join("dylint.toml")).unwrap();
        assert_eq!(read_configuration(&location), None);
    }

    /// Malformed Cargo locations cannot select a configuration file.
    #[test_case(b"invalid"; "invalid json")]
    #[test_case(b"{}"; "missing root")]
    #[test_case(b"{\"root\":4}"; "nontext root")]
    #[test_case(b"{\"root\":\"/\"}"; "no parent")]
    fn rejects_invalid_manifest_locations(location: &[u8]) {
        assert_eq!(read_configuration(location), None);
    }

    /// Cargo resolves a member's configuration from its workspace root and honors
    /// manifests.
    #[test]
    fn resolves_workspace_from_member_and_selected_manifest() {
        // Different root and member configurations expose incorrect root selection.
        let directory = workspace_fixture();
        let root = directory.path();
        let cargo = Path::new(env!("SAGAN_LINTS_BUILD_SYSROOT")).join("bin/cargo");
        assert_eq!(
            load_configuration(&cargo, &root.join("member"), None).as_deref(),
            Some("[lint]\nlimit = 7\n")
        );
        // Explicit manifests must still resolve the root, and missing manifests must fail.
        assert_eq!(
            load_configuration(&cargo, root, Some(Path::new("member/Cargo.toml"))).as_deref(),
            Some("[lint]\nlimit = 7\n")
        );
        assert_eq!(
            load_configuration(&cargo, root, Some(Path::new("missing/Cargo.toml"))),
            None
        );
    }
    /// Create a workspace whose member has a conflicting local configuration.
    fn workspace_fixture() -> tempfile::TempDir {
        // Cargo needs both manifests and a member source to resolve membership.
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("member/src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\nresolver = \"3\"\n",
        )
        .unwrap();
        fs::write(
            root.join("member/Cargo.toml"),
            "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(root.join("member/src/lib.rs"), "").unwrap();
        // Conflicting values distinguish root configuration from member-local configuration.
        fs::write(root.join("dylint.toml"), "[lint]\nlimit = 7\n").unwrap();
        fs::write(root.join("member/dylint.toml"), "[lint]\nlimit = 99\n").unwrap();
        directory
    }

    /// A missing Cargo executable retains original compiler-side discovery.
    #[test]
    fn preserves_process_start_failure() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        assert_eq!(
            load_configuration(&root.join("missing-cargo"), root, None),
            None
        );
    }
}
