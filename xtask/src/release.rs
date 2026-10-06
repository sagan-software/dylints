//! Semver validation and immutable GitHub releases after validation gates.

use std::{fs, io::Write as _, path::Path, process::Command};

use crate::{error::Error, process};

/// Read the shared semver from Cargo's authoritative workspace manifest.
pub(crate) fn version(root: &Path) -> Result<semver::Version, Error> {
    manifest_version(&fs::read_to_string(root.join("Cargo.toml"))?)
}

/// Decode semver only at the Cargo manifest boundary.
fn manifest_version(source: &str) -> Result<semver::Version, Error> {
    let manifest: toml::Value = toml::from_str(source)?;
    let source = manifest
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            Error::Invalid("workspace.package.version is missing or is not a string".to_owned())
        })?;
    Ok(source.parse()?)
}

/// Validated full Git SHA used to compare a push with its previous version.
#[derive(Clone, Debug)]
pub(crate) struct GitCommit(Box<str>);

impl std::str::FromStr for GitCommit {
    type Err = String;
    /// Accept one full SHA-1 commit identifier and normalize hexadecimal
    /// casing.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        if source.len() == 40
            && source.bytes().all(|byte| byte.is_ascii_hexdigit())
            && source.bytes().any(|byte| byte != b'0')
        {
            Ok(Self(source.to_ascii_lowercase().into_boxed_str()))
        } else {
            Err("base must be a nonzero 40-digit Git commit SHA".to_owned())
        }
    }
}

/// Require a semver increase before any publication side effect.
pub(crate) fn check_increment(root: &Path, base: &GitCommit) -> Result<(), Error> {
    // Read the base manifest through Git without modifying the checkout.
    let revision = &base.0;
    let previous = process::output(
        Command::new("git")
            .current_dir(root)
            .args(["show", &format!("{revision}:Cargo.toml")]),
    )?;
    let previous = manifest_version(&String::from_utf8_lossy(&previous.stdout))?;
    // Compare complete semver values, including prerelease ordering.
    let current = version(root)?;
    if current <= previous {
        return Err(Error::Invalid(format!(
            "workspace version {current} must exceed base version {previous}"
        )));
    }
    Ok(())
}

/// Validate the previous version before printing or publishing the current
/// release.
pub(crate) fn run(root: &Path, base: Option<&GitCommit>, is_published: bool) -> Result<(), Error> {
    // Reject an unchanged version before any tag or release mutation.
    if let Some(base) = base {
        check_increment(root, base)?;
    }
    if is_published {
        return publish(root);
    }
    // The read-only command exposes the authoritative workspace version.
    let version = version(root)?;
    writeln!(std::io::stdout().lock(), "v{version}")?;
    Ok(())
}

/// Publish an immutable tag and release, retaining major and minor maintenance
/// branches.
pub(crate) fn publish(root: &Path) -> Result<(), Error> {
    let version = stable_version(root)?;
    // A stable workspace version determines immutable tag and maintenance names.
    let tag = format!("v{version}");
    let current = process::output(
        Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "HEAD"]),
    )?;
    // Tag identity comes from Git rather than an independently supplied release value.
    let current = String::from_utf8_lossy(&current.stdout);
    publish_artifacts(root, &tag, current.trim(), &version)
}

/// Reject prerelease and build identifiers before maintenance artifacts are
/// created.
fn stable_version(root: &Path) -> Result<semver::Version, Error> {
    let version = version(root)?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(Error::Invalid(
            "maintenance releases require a stable semver".to_owned(),
        ));
    }
    Ok(version)
}

/// Publish artifacts in tag, branch and release order after validating their
/// version.
fn publish_artifacts(
    root: &Path,
    tag: &str,
    current: &str,
    version: &semver::Version,
) -> Result<(), Error> {
    // Create each remote artifact only after its local identity has been checked.
    ensure_tag(root, tag, current, version)?;
    ensure_branches(root, version)?;
    ensure_release(root, tag)
}

/// Create a GitHub release only after a successful read proves that it is
/// absent.
fn ensure_release(root: &Path, tag: &str) -> Result<(), Error> {
    // A failed API read stays a failure; only a successful absence permits creation.
    let releases = process::output(Command::new("gh").current_dir(root).args([
        "api",
        "repos/{owner}/{repo}/releases",
        "--paginate",
        "--jq",
        ".[].tag_name",
    ]))?;
    if String::from_utf8_lossy(&releases.stdout)
        .lines()
        .any(|existing| existing == tag)
    {
        return Ok(());
    }
    process::run(Command::new("gh").current_dir(root).args([
        "release",
        "create",
        tag,
        "--verify-tag",
        "--generate-notes",
    ]))
}

/// Preserve existing tag identity and retry an interrupted tag push safely.
fn ensure_tag(
    root: &Path,
    tag: &str,
    current: &str,
    version: &semver::Version,
) -> Result<(), Error> {
    let existing = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify"])
        .arg(format!("refs/tags/{tag}^{{commit}}"))
        .output()?;
    // A tag may be reused only for the exact commit it already authenticates.
    if existing.status.success() {
        if String::from_utf8_lossy(&existing.stdout).trim() != current {
            return Err(Error::Invalid(format!(
                "tag {tag} already identifies another commit"
            )));
        }
    } else {
        process::run(
            Command::new("git")
                .current_dir(root)
                .args(["tag", "-a"])
                .arg(tag)
                .args(["-m", &format!("Release {version}")]),
        )?;
    }
    // Always push the matching tag, including a locally created tag from an interrupted run.
    process::run(
        Command::new("git")
            .current_dir(root)
            .args(["push", "origin"])
            .arg(tag),
    )
}

/// Create major and minor maintenance branches without moving existing
/// branches.
fn ensure_branches(root: &Path, version: &semver::Version) -> Result<(), Error> {
    let major = version.major;
    let minor = version.minor;
    // Retain separate major and minor maintenance histories.
    for branch in [
        format!("release/{major}"),
        format!("release/{major}.{minor}"),
    ] {
        // Create maintenance branches once. Later releases never retarget existing branches.
        let remote = process::output(
            Command::new("git")
                .current_dir(root)
                .args(["ls-remote", "--heads", "origin"])
                .arg(format!("refs/heads/{branch}")),
        )?;
        // Existing branches retain their owner-selected commit.
        if remote.stdout.is_empty() {
            process::run(
                Command::new("git")
                    .current_dir(root)
                    .arg("push")
                    // An empty expected value requires absence at push time.
                    // https://git-scm.com/docs/git-push#Documentation/git-push.txt---force-with-leaseltrefnamegtltexpectgt
                    .arg(format!("--force-with-lease=refs/heads/{branch}:"))
                    .arg("origin")
                    .arg(format!("HEAD:refs/heads/{branch}")),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::version;

    /// Missing, malformed and non-semver workspace versions remain distinct
    /// failures.
    #[test]
    fn rejects_missing_manifest() {
        let root = tempfile::tempdir().expect("version fixture");
        assert!(version(root.path()).is_err());
    }

    /// Each malformed manifest or version fails at its own parsing boundary.
    #[test_case::test_case("invalid = ["; "invalid TOML")]
    #[test_case::test_case("[workspace.package]\n"; "missing version")]
    #[test_case::test_case("[workspace.package]\nversion = 1"; "nontext version")]
    fn rejects_invalid_versions(source: &str) {
        let root = tempfile::tempdir().expect("version fixture");
        std::fs::write(root.path().join("Cargo.toml"), source).expect("manifest fixture");
        assert!(version(root.path()).is_err());
    }

    /// Invalid semver retains the parser's typed failure and diagnostic source.
    #[test]
    fn invalid_semver_preserves_source() {
        // Keep the parser failure available through the development command's error chain.
        let source = "[workspace.package]\nversion = \"invalid\"";
        let error = super::manifest_version(source).expect_err("invalid semver");
        assert!(std::error::Error::source(&error).is_some());
        assert!(matches!(error, crate::error::Error::Semver(_)));
    }

    /// A present nontext version reports its type restriction instead of claiming absence.
    #[test]
    fn nontext_version_reports_type_requirement() {
        // Distinguish an invalid value from a missing version in the diagnostic contract.
        let source = "[workspace.package]\nversion = 1";
        let error = super::manifest_version(source).expect_err("nontext version");
        assert_eq!(
            error.to_string(),
            "workspace.package.version is missing or is not a string"
        );
    }

    /// Full hexadecimal commit identifiers normalize case and reject malformed
    /// input.
    #[test]
    fn commit_identity_boundaries() {
        use std::str::FromStr as _;
        let uppercase = "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD";
        assert_eq!(
            &*super::GitCommit::from_str(uppercase).expect("full SHA").0,
            "abcdefabcdefabcdefabcdefabcdefabcdefabcd"
        );
    }

    /// Rejected identities cannot identify a release comparison commit.
    #[test_case::test_case(""; "empty")]
    #[test_case::test_case("main"; "branch name")]
    #[test_case::test_case("0000000000000000000000000000000000000000"; "zero SHA")]
    #[test_case::test_case("111111111111111111111111111111111111111g"; "nonhexadecimal")]
    fn rejects_invalid_commit_identity(source: &str) {
        use std::str::FromStr as _;
        assert!(super::GitCommit::from_str(source).is_err());
    }

    /// Release validation retains all semver components instead of parsing a
    /// numeric prefix.
    #[test]
    fn parses_workspace_semver() {
        let root = tempfile::tempdir().expect("version fixture");
        // Full semver parsing preserves prerelease and build metadata.
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[workspace.package]\nversion = \"1.2.3-rc.1+build\"",
        )
        .expect("manifest fixture");
        let parsed = version(root.path()).expect("valid version");
        assert_eq!(parsed.to_string(), "1.2.3-rc.1+build");
        assert!(super::publish(root.path()).is_err());
    }
}
