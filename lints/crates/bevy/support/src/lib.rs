#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for Bevy-specific private lints.

extern crate rustc_abi;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use core::ops::ControlFlow;

use rustc_abi::Size;
use rustc_hir::{
    Block, Body, Expr, ExprKind, FnDecl, ItemKind, Mutability, VariantData,
    def::Res,
    def_id::{DefId, LocalDefId},
    intravisit::{FnKind, Visitor},
};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Region, Ty, TyCtxt, TypeVisitable, TypeVisitor, layout::TyAndLayout};
use rustc_span::{Span, Symbol};

use dylint_linting as _;
use dylint_support as _;

/// One semantically resolved Bevy method call.
#[derive(Clone, Copy, Debug)]
pub struct BevyMethodCall<'hir> {
    /// Receiver expression.
    pub receiver: &'hir Expr<'hir>,
    /// Explicit arguments, excluding the receiver.
    pub arguments: &'hir [Expr<'hir>],
    /// Span of the method identifier.
    pub method_span: Span,
    /// Resolved method definition.
    pub def_id: DefId,
}

/// Field-level access performed through one mutable component query.
#[derive(Clone, Debug)]
pub struct ComponentFieldAccess {
    /// Local component definition.
    pub component: LocalDefId,
    /// Named component fields accessed by the function.
    pub fields: Vec<Symbol>,
}

/// Direct free systems registered by one `App::add_systems` call.
#[derive(Clone, Debug)]
pub struct RegisteredSystems {
    /// Resolved schedule-label type name.
    pub schedule: Symbol,
    /// Direct local free-function definitions.
    pub systems: Vec<LocalDefId>,
}

/// A standard trait expected on an empty marker component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkerTrait {
    /// The `Clone` trait.
    Clone,
    /// The `Copy` trait.
    Copy,
    /// The `Default` trait.
    Default,
}

impl MarkerTrait {
    /// Return the standard trait name.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = bevy_support::MarkerTrait::name(value);
    /// };
    /// ```
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clone => "Clone",
            Self::Copy => "Copy",
            Self::Default => "Default",
        }
    }

    /// Return the trait's rustc diagnostic item.
    const fn diagnostic_item(self) -> Symbol {
        match self {
            Self::Clone => rustc_span::sym::Clone,
            Self::Copy => rustc_span::sym::Copy,
            Self::Default => rustc_span::sym::Default,
        }
    }
}

/// A Bevy proxy type whose value can be cheaply reborrowed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reborrowable {
    /// `Commands`.
    Commands,
    /// `Deferred`.
    Deferred,
    /// `DeferredWorld`.
    DeferredWorld,
    /// `EntityCommands`.
    EntityCommands,
    /// `EntityMut`.
    EntityMut,
    /// `FilteredEntityMut`.
    FilteredEntityMut,
    /// Bevy's change-detecting `Mut`.
    Mut,
    /// Bevy's type-erased `MutUntyped`.
    MutUntyped,
    /// `NonSendMut`.
    NonSendMut,
    /// `PtrMut`.
    PtrMut,
    /// `Query`.
    Query,
    /// `ResMut`.
    ResMut,
}

impl Reborrowable {
    /// Return the user-facing type name.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = bevy_support::Reborrowable::name(value);
    /// };
    /// ```
    pub const fn name(self) -> &'static str {
        match self {
            Self::Commands => "Commands",
            Self::Deferred => "Deferred",
            Self::DeferredWorld => "DeferredWorld",
            Self::EntityCommands => "EntityCommands",
            Self::EntityMut => "EntityMut",
            Self::FilteredEntityMut => "FilteredEntityMut",
            Self::Mut => "Mut",
            Self::MutUntyped => "MutUntyped",
            Self::NonSendMut => "NonSendMut",
            Self::PtrMut => "PtrMut",
            Self::Query => "Query",
            Self::ResMut => "ResMut",
        }
    }
}
mod bundles;
mod helpers;
mod macros;
mod methods;
mod queries;
mod query_origins;
mod systems;
mod traits;

pub use self::bundles::{
    DuplicatePluginAddition, ElapsedSecsWidening, UnitBundleValue, duplicate_plugin_addition,
    duplicate_plugin_additions_in_block, elapsed_secs_widening, tuple_duplicate_plugin_addition,
    unit_bundle_values,
};
pub use self::helpers::{parameter_spans, parameter_types};
pub use self::methods::{
    bevy_method_call, expression_has_type, trait_is_named, type_is_named, world_method_call,
};
pub use self::queries::{
    borrowed_reborrowable_parameters, children_mutation_query_parameters,
    large_component_change_filter_parameters, local_large_components,
    mutable_component_field_access, mutable_query_component_parameters,
    narrow_exclusive_system_parameters, partially_used_query_data_parameters,
    presence_only_query_parameters, readonly_mut_query_parameters, shared_query_data_replacements,
    unfiltered_entity_access_query_parameters, wide_query_parameters, zst_query_parameters,
};
pub use self::systems::{
    camera_fixed_update_system_spans, directly_registered_repeating_systems,
    directly_registered_systems, disallowed_schedule_span, discarded_app_run_spans,
    inserted_message_resource_span, is_system_mutably_querying_camera,
    iter_current_update_messages_span,
};
pub use self::traits::{
    direct_bevy_facades, local_bevy_types_missing_reflect, local_unit_components_missing_trait,
    missing_trait_derive, unconventional_bevy_type_names,
};
