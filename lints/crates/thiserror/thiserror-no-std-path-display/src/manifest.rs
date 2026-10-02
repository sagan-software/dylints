//! Decide from `Cargo.toml` whether the compiled package disables thiserror's `std`.
//!
//! Cargo sets `CARGO_MANIFEST_DIR` for every `rustc` it runs, so the lint reads
//! exactly the manifest of the package being compiled. The manifest is parsed
//! as TOML. Every `thiserror` declaration in `[dependencies]`,
//! `[dev-dependencies]`, and their `[target.*]` forms counts, including renamed
//! declarations and `workspace = true` declarations, which take their
//! `default-features` and `features` from the workspace root manifest.

use std::{
    fs::File,
    io::Read as _,
    path::{Path, PathBuf},
};
use toml::{Table, Value};

/// Return whether every thiserror declaration of the compiled package disables `std`.
///
/// The answer is `false` when the manifest cannot be read or declares no thiserror.
pub(crate) fn is_thiserror_std_disabled() -> bool {
    // Cargo configuration supplies the package directory at the boundary.
    let Some(directory) = load_config_manifest_directory() else {
        return false;
    };
    // A malformed or unreadable manifest cannot establish the feature state.
    let Some(manifest) = read_table(&directory.join("Cargo.toml")) else {
        return false;
    };
    let workspace = workspace_manifest(&directory);
    is_std_disabled_in(&manifest, workspace.as_ref())
}

/// Return the package manifest directory supplied by Cargo.
fn load_config_manifest_directory() -> Option<PathBuf> {
    // Cargo identifies the package manifest through this environment variable.
    let directory = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
    // Pass a typed path into manifest parsing and keep environment access at the boundary.
    directory
}

/// Find the nearest manifest at or above `directory` that declares `[workspace]`.
fn workspace_manifest(directory: &Path) -> Option<Table> {
    directory
        .ancestors()
        .filter_map(|ancestor| read_table(&ancestor.join("Cargo.toml")))
        .find(|manifest| manifest.contains_key("workspace"))
}

/// Read and parse one TOML file.
fn read_table(path: &Path) -> Option<Table> {
    let mut text = String::new();
    let _ = File::open(path).ok()?.read_to_string(&mut text).ok()?;
    text.parse().ok()
}

/// Return whether a manifest declares thiserror and no declaration enables `std`.
fn is_std_disabled_in(manifest: &Table, workspace: Option<&Table>) -> bool {
    let workspace_entry = workspace
        .and_then(|root| root.get("workspace")?.get("dependencies")?.as_table())
        .and_then(thiserror_entry);
    let declarations = declarations(manifest);
    !declarations.is_empty()
        && declarations
            .iter()
            .all(|declaration| !has_std_enabled(declaration, workspace_entry))
}

/// Collect thiserror declarations from every dependency table that builds the crate.
fn declarations(manifest: &Table) -> Vec<&Value> {
    // Target-specific tables use the same keys under `[target.<cfg>]`.
    let targets = manifest
        .get("target")
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(Table::values)
        .filter_map(Value::as_table);
    std::iter::once(manifest)
        .chain(targets)
        .flat_map(|table| ["dependencies", "dev-dependencies"].map(|key| table.get(key)))
        .flatten()
        .filter_map(Value::as_table)
        .filter_map(thiserror_entry)
        .collect()
}

/// Return the declaration for the `thiserror` package in one dependency table.
fn thiserror_entry(dependencies: &Table) -> Option<&Value> {
    dependencies.iter().find_map(|(key, value)| {
        let package = value.get("package").and_then(Value::as_str).unwrap_or(key);
        (package == "thiserror").then_some(value)
    })
}

/// Return whether one declaration enables thiserror's `std` feature.
fn has_std_enabled(declaration: &Value, workspace_entry: Option<&Value>) -> bool {
    // A plain version string keeps the default features, which include `std`.
    let Some(table) = declaration.as_table() else {
        return true;
    };
    let uses_workspace_dependency = table.get("workspace").and_then(Value::as_bool) == Some(true);
    // Workspace declarations inherit their feature defaults from the root entry.
    let base = if uses_workspace_dependency {
        workspace_entry
    } else {
        Some(declaration)
    };
    let default_features = base.is_none_or(|base| {
        base.get("default-features")
            .or_else(|| base.get("default_features"))
            .and_then(Value::as_bool)
            != Some(false)
    });
    default_features
        || has_std_feature(Some(declaration))
        || (uses_workspace_dependency && has_std_feature(workspace_entry))
}

/// Return whether a declaration lists the `std` feature.
fn has_std_feature(declaration: Option<&Value>) -> bool {
    declaration
        .and_then(|declaration| declaration.get("features")?.as_array())
        .is_some_and(|features| {
            features
                .iter()
                .any(|feature| feature.as_str() == Some("std"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a manifest fixture.
    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    #[test]
    fn reads_plain_and_feature_flags() {
        let disabled = |text: &str| is_std_disabled_in(&table(text), None);
        assert!(!disabled("[dependencies]\nserde = \"1\""));
        assert!(!disabled("[dependencies]\nthiserror = \"2\""));
        assert!(disabled(
            "[dependencies]\nthiserror = { version = \"2\", default-features = false }"
        ));
    }

    #[test]
    fn reads_table_forms() {
        let disabled = |text: &str| is_std_disabled_in(&table(text), None);
        assert!(disabled(
            "[dependencies.thiserror]\nversion = \"2\"\ndefault_features = false"
        ));
        assert!(!disabled(
            "[dependencies]\nthiserror = { version = \"2\", default-features = false, features = [\"std\"] }"
        ));
    }

    #[test]
    fn reads_target_and_dev_declarations() {
        let disabled = |text: &str| is_std_disabled_in(&table(text), None);
        assert!(disabled(
            "[target.'cfg(unix)'.dev-dependencies]\nerr = { package = \"thiserror\", version = \"2\", default-features = false }"
        ));
        assert!(!disabled(
            "[dependencies]\nthiserror = { version = \"2\", default-features = false }\n[dev-dependencies]\nthiserror = \"2\""
        ));
    }

    #[test]
    fn reads_inherited_workspace_declaration() {
        let workspace = table(
            "[workspace.dependencies]\nthiserror = { version = \"2\", default-features = false }",
        );
        let inherited = "[dependencies]\nthiserror.workspace = true";
        assert!(is_std_disabled_in(&table(inherited), Some(&workspace)));
        assert!(!is_std_disabled_in(&table(inherited), None));
    }

    #[test]
    fn reads_workspace_feature_override() {
        let workspace = table(
            "[workspace.dependencies]\nthiserror = { version = \"2\", default-features = false }",
        );
        assert!(!is_std_disabled_in(
            &table("[dependencies]\nthiserror = { workspace = true, features = [\"std\"] }"),
            Some(&workspace)
        ));
    }

    #[test]
    fn reads_workspace_std_feature() {
        let with_std = table(
            "[workspace.dependencies]\nthiserror = { version = \"2\", default-features = false, features = [\"std\"] }",
        );
        let inherited = "[dependencies]\nthiserror.workspace = true";
        assert!(!is_std_disabled_in(&table(inherited), Some(&with_std)));
    }

    #[test]
    fn reads_manifest_file() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(read_table(&manifest.join("Cargo.toml")).is_some());
    }

    #[test]
    fn rejects_missing_manifest_file() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(read_table(&manifest.join("missing.toml")).is_none());
    }

    #[test]
    fn rejects_non_manifest_file() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(read_table(&manifest.join("README.md")).is_none());
    }

    #[test]
    fn finds_workspace_manifest() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(workspace_manifest(manifest).is_some());
    }
}
