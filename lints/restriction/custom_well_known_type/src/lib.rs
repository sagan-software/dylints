#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the exhaustive type catalog stays visible beside its targeted rustc matching"
)]
//! A lint to check for local types that duplicate well-known semantic types.
//!
//! It recognizes exact names for common HTTP, URL, identity, filesystem, time,
//! and address concepts, then resolves aliases before deciding whether the
//! local declaration already uses an established type. The diagnostic preserves
//! project-specific names while guiding primitive or duplicate declarations to
//! a typed boundary representation.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, def_id::DefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CUSTOM_WELL_KNOWN_TYPE,
    Warn,
    "local type declaration duplicates a well-known semantic type",
    CustomWellKnownType
}

impl<'tcx> LateLintPass<'tcx> for CustomWellKnownType {
    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Restrict classification to named structs, enums, and type aliases.
        let Some(declaration_kind) = declaration_kind(item) else {
            return;
        };

        let Some(ident) = item.kind.ident() else {
            return;
        };

        // Classify exact well-known names before resolving alias identity.
        let type_name = ident.name.to_ident_string();
        let Some(well_known_type) = well_known_type(&type_name) else {
            return;
        };

        if aliases_actual_well_known_type(cx, item, well_known_type) {
            return;
        }

        emit_custom_type_lint(
            cx,
            ident.span,
            declaration_kind,
            &type_name,
            well_known_type.replacement,
        );
    }
}

/// State used by the well known type analysis.
#[derive(Clone, Copy)]
struct WellKnownType {
    /// replacement stored for this lint's analysis.
    replacement: &'static str,
    /// Definition paths, starting with the defining crate, of the established types.
    canonical_paths: &'static [&'static str],
}

/// Closed set of established semantic type families recognized by this lint.
#[derive(Clone, Copy, Eq, PartialEq)]
enum WellKnownKind {
    /// HTTP status code types.
    StatusCode,
    /// HTTP method types.
    Method,
    /// URL types.
    Url,
    /// URI types.
    Uri,
    /// UUID types.
    Uuid,
    /// Email address types.
    Email,
    /// Borrowed filesystem path types.
    Path,
    /// Owned filesystem path types.
    PathBuf,
    /// Duration types.
    Duration,
    /// Monotonic instant types.
    Instant,
    /// Calendar date-time types.
    DateTime,
}

/// Helper for declaration kind analysis.
const fn declaration_kind(item: &Item<'_>) -> Option<&'static str> {
    match item.kind {
        ItemKind::Struct(..) => Some("struct"),
        ItemKind::Enum(..) => Some("enum"),
        ItemKind::TyAlias(..) => Some("type alias"),
        _ => None,
    }
}

/// Emit the custom type lint diagnostic.
fn emit_custom_type_lint(
    cx: &LateContext<'_>,
    span: Span,
    declaration_kind: &'static str,
    type_name: &str,
    replacement: &'static str,
) {
    // Name the local declaration kind and duplicated semantic type.
    // Reuse the selected established type guidance as diagnostic help.
    emit_span_lint_with_help(
        cx,
        CUSTOM_WELL_KNOWN_TYPE,
        span,
        format!("local {declaration_kind} `{type_name}` duplicates a well-known semantic type"),
        replacement,
    );
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's diagnostic decorator so the message and help match the rest of the suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return type information for aliases actual well known.
fn aliases_actual_well_known_type(
    cx: &LateContext<'_>,
    item: &Item<'_>,
    well_known_type: WellKnownType,
) -> bool {
    let ItemKind::TyAlias(..) = item.kind else {
        return false;
    };

    // Resolve aliases through rustc's type system so renamed imports and fully qualified standard
    // paths are treated as the same established type.
    let alias_ty = cx
        .tcx
        .type_of(item.owner_id.def_id)
        .instantiate_identity()
        .skip_norm_wip();
    ty_matches_known_paths(cx, alias_ty, well_known_type.canonical_paths)
}

/// Helper for ty matches known paths analysis.
fn ty_matches_known_paths(
    cx: &LateContext<'_>,
    ty: ty::Ty<'_>,
    canonical_paths: &'static [&'static str],
) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => def_id_matches_known_paths(cx, adt.did(), canonical_paths),
        _ => false,
    }
}

/// Helper for def id matches known paths analysis.
fn def_id_matches_known_paths(
    cx: &LateContext<'_>,
    def_id: DefId,
    canonical_paths: &'static [&'static str],
) -> bool {
    // The definition path starts with the defining crate, so imports and re-exports such as
    // `reqwest::Url` or `std::time::Duration` resolve to `url::Url` and `core::time::Duration`.
    let def_path = cx
        .get_def_path(def_id)
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::");
    canonical_paths.contains(&def_path.as_str())
}

/// Return type information for well known.
fn well_known_type(type_name: &str) -> Option<WellKnownType> {
    // Keep exact names so project-specific declarations such as `PaymentMethod` stay quiet.
    let kind = WELL_KNOWN_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == type_name)
        .map(|(_, kind)| *kind)?;
    WELL_KNOWN_TYPES
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .map(|(_, known_type)| *known_type)
}

/// Replacement guidance and canonical paths for each recognized family.
const WELL_KNOWN_TYPES: &[(WellKnownKind, WellKnownType)] = &[
    (
        WellKnownKind::StatusCode,
        WellKnownType {
            replacement: "use `http::StatusCode` or `reqwest::StatusCode`",
            canonical_paths: &["http::status::StatusCode"],
        },
    ),
    (
        WellKnownKind::Method,
        WellKnownType {
            replacement: "use `http::Method` or `reqwest::Method`",
            canonical_paths: &["http::method::Method"],
        },
    ),
    (
        WellKnownKind::Url,
        WellKnownType {
            replacement: "use `url::Url`, `reqwest::Url`, or another established URL type",
            canonical_paths: &["url::Url"],
        },
    ),
    (
        WellKnownKind::Uri,
        WellKnownType {
            replacement: "use `http::Uri` or another established URI type",
            canonical_paths: &["http::uri::Uri"],
        },
    ),
    (
        WellKnownKind::Uuid,
        WellKnownType {
            replacement: "use `uuid::Uuid`",
            canonical_paths: &["uuid::Uuid"],
        },
    ),
    (
        WellKnownKind::Email,
        WellKnownType {
            replacement: "use an established email address type or a validated boundary newtype with a project-specific name",
            canonical_paths: &[],
        },
    ),
    (
        WellKnownKind::Path,
        WellKnownType {
            replacement: "use `std::path::Path`",
            canonical_paths: &["std::path::Path"],
        },
    ),
    (
        WellKnownKind::PathBuf,
        WellKnownType {
            replacement: "use `std::path::PathBuf`",
            canonical_paths: &["std::path::PathBuf"],
        },
    ),
    (
        WellKnownKind::Duration,
        WellKnownType {
            replacement: "use `std::time::Duration`",
            canonical_paths: &["core::time::Duration"],
        },
    ),
    (
        WellKnownKind::Instant,
        WellKnownType {
            replacement: "use `std::time::Instant`",
            canonical_paths: &["std::time::Instant"],
        },
    ),
    (
        WellKnownKind::DateTime,
        WellKnownType {
            replacement: "use `chrono::DateTime`, `time::OffsetDateTime`, or another established datetime type",
            canonical_paths: &[
                "chrono::datetime::DateTime",
                "time::offset_date_time::OffsetDateTime",
            ],
        },
    ),
];

/// Canonical and compatibility names mapped to their semantic type family.
const WELL_KNOWN_NAMES: &[(&str, WellKnownKind)] = &[
    ("StatusCode", WellKnownKind::StatusCode),
    ("HttpStatusCode", WellKnownKind::StatusCode),
    ("Method", WellKnownKind::Method),
    ("HttpMethod", WellKnownKind::Method),
    ("Url", WellKnownKind::Url),
    ("URL", WellKnownKind::Url),
    ("Uri", WellKnownKind::Uri),
    ("URI", WellKnownKind::Uri),
    ("Uuid", WellKnownKind::Uuid),
    ("UUID", WellKnownKind::Uuid),
    ("Email", WellKnownKind::Email),
    ("EmailAddress", WellKnownKind::Email),
    ("Path", WellKnownKind::Path),
    ("PathBuf", WellKnownKind::PathBuf),
    ("Duration", WellKnownKind::Duration),
    ("Instant", WellKnownKind::Instant),
    ("Timestamp", WellKnownKind::DateTime),
    ("DateTime", WellKnownKind::DateTime),
];

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
