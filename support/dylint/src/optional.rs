//! Type helpers for optional semantic values used by lint crates.

extern crate rustc_middle;
extern crate rustc_span;

use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::sym;

/// Maximum number of nested standard `Option` types inspected at one boundary.
const MAX_STANDARD_OPTION_DEPTH: usize = 8;

/// Peel consecutive resolved standard `Option` layers from a type.
///
/// Returns `None` when another standard `Option` remains after eight layers.
#[must_use]
pub fn peel_standard_options<'tcx>(tcx: TyCtxt<'tcx>, mut ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    // Follow only the compiler-resolved standard `Option` ADT.
    for _ in 0..MAX_STANDARD_OPTION_DEPTH {
        let ty::Adt(adt, args) = ty.kind() else {
            return Some(ty);
        };
        if !tcx.is_diagnostic_item(sym::Option, adt.did()) {
            return Some(ty);
        }
        ty = args.types().next()?;
    }

    // Keep an over-depth Option opaque instead of treating it as its inner value.
    if matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(sym::Option, adt.did())) {
        None
    } else {
        Some(ty)
    }
}
