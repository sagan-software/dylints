//! Follow tracing span guards through owned MIR places until coroutine suspension.
//!
//! The may-live worklist stores unique local and coroutine-field paths. It
//! follows at most sixteen aggregate fields and stops at unrecognized calls.
//! For B blocks and F distinct path or alias facts, it uses O(BF) flow space.
//! Vector-backed joins and transfers take O((M + E)F^3) worst-case time per
//! constructor, where M counts MIR transfer operations and E counts CFG edges.
//! Saved-type checks add O(BF^2T) time, where T bounds visited type fields.

use std::collections::{HashMap, HashSet, VecDeque};

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_errors::DiagDecorator;
use rustc_hir::{LangItem, def_id::LocalDefId};
use rustc_lint::{LateContext, LintContext as _};
use rustc_middle::{
    mir::{
        AggregateKind, BasicBlock, Local, Place, ProjectionElem, Rvalue, Statement, StatementKind,
        TerminatorKind,
    },
    ty::{Ty, TyKind},
};
use rustc_span::{Span, Spanned};

use crate::TRACING_AWAIT_HOLDING_SPAN_GUARD;

/// Maximum nested MIR fields followed while proving guard ownership.
const MAX_GUARD_FIELD_DEPTH: usize = 16;

/// Check tracing guard constructor results that may survive in owned wrappers.
pub(super) fn check_wrapped_guards<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    async_body: LocalDefId,
) {
    // Analyze each resolved tracing guard constructor in the async MIR body.
    let body = cx.tcx.optimized_mir(async_body);
    for (_, data) in body.basic_blocks.iter_enumerated() {
        check_guard_constructor(cx, coroutine, async_body, body, data.terminator());
    }
}

/// Check whether one MIR call's tracing guard reaches a pending coroutine state.
fn check_guard_constructor<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    async_body: LocalDefId,
    body: &rustc_middle::mir::Body<'tcx>,
    terminator: &rustc_middle::mir::Terminator<'tcx>,
) {
    // Only successful call edges have a result place that can receive the guard.
    let TerminatorKind::Call {
        func,
        destination,
        target: Some(target),
        ..
    } = &terminator.kind
    else {
        return;
    };
    // Resolve the method before interpreting its return type as a tracing guard.
    if !is_tracing_guard_constructor(cx, func) {
        return;
    }
    let guard_ty = destination.ty(body, cx.tcx).ty;
    // Keep this path limited to the two tracing guard ADTs diagnosed by the lint.
    if !is_tracing_span_guard_type(cx, guard_ty) {
        return;
    }

    // Continue from the successful call edge where MIR initializes the guard result.
    let origin = GuardOrigin {
        target: *target,
        destination: guard_place(body, async_body, *destination),
        guard_ty,
        span: terminator.source_info.span,
    };
    let variants = live_wrapper_variants(cx, coroutine, async_body, body, &origin);
    let await_points = variants
        .into_iter()
        .filter_map(|variant| {
            coroutine
                .variant_source_info
                .get(variant)
                .map(|source_info| source_info.span)
        })
        .collect::<Vec<_>>();
    if !await_points.is_empty() {
        emit_wrapper_guard_lint(cx, origin.span, await_points);
    }
}

/// Emit the existing tracing guard diagnostic at its constructor and await points.
fn emit_wrapper_guard_lint(cx: &LateContext<'_>, origin: Span, await_points: Vec<Span>) {
    cx.emit_span_lint(
        TRACING_AWAIT_HOLDING_SPAN_GUARD,
        origin,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic
                .primary_message("this tracing span guard is held across an `.await` point")
                .span_note(
                    await_points,
                    "the span remains entered through these suspension points",
                )
                .help("use `Span::in_scope`, `Future::instrument`, or `#[instrument]`");
        }),
    );
}

/// The result place and successful edge for one resolved guard constructor.
struct GuardOrigin<'tcx> {
    /// MIR block reached after the guard constructor returns.
    target: BasicBlock,
    /// Place initialized by the tracing constructor result.
    destination: GuardPlace,
    /// Exact tracing guard type returned by the constructor.
    guard_ty: Ty<'tcx>,
    /// Source span for the resolved tracing constructor call.
    span: Span,
}

/// Identify the two resolved tracing methods that construct span guards.
fn is_tracing_guard_constructor<'tcx>(
    cx: &LateContext<'tcx>,
    func: &rustc_middle::mir::Operand<'tcx>,
) -> bool {
    // Constant function definitions provide the resolved API identity.
    let Some((def_id, _)) = func.const_fn_def() else {
        return false;
    };
    if cx.tcx.crate_name(def_id.krate).as_str() != "tracing" {
        return false;
    }
    matches!(
        cx.tcx.def_path_str(def_id).as_str(),
        "tracing::Span::enter" | "tracing::Span::entered"
    )
}

/// Prove that a type is one of tracing's two span guard ADTs.
fn is_tracing_span_guard_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let TyKind::Adt(adt, _) = ty.kind() else {
        return false;
    };
    cx.tcx.crate_name(adt.did().krate).as_str() == "tracing"
        && matches!(
            cx.tcx.def_path_str(adt.did()).as_str(),
            "tracing::span::Entered" | "tracing::span::EnteredSpan"
        )
}

/// Track one constructor result until a pending poll saves it in the coroutine.
fn live_wrapper_variants<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    async_body: LocalDefId,
    body: &rustc_middle::mir::Body<'tcx>,
    origin: &GuardOrigin<'tcx>,
) -> Vec<VariantIdx> {
    // Store the may-live state at each block and revisit blocks only when it grows.
    let mut worklist = GuardWorklist {
        incoming_states: vec![None; body.basic_blocks.len()],
        pending_blocks: VecDeque::from([origin.target]),
        live_variants: Vec::new(),
    };
    let Some(origin_state) = worklist.incoming_states.get_mut(origin.target.index()) else {
        return Vec::new();
    };
    // Seed only the successful constructor edge where this guard first exists.
    *origin_state = Some(GuardFlowState {
        places: vec![origin.destination.clone()],
        references: HashMap::new(),
    });

    while let Some(block) = worklist.pending_blocks.pop_front() {
        process_guard_block(
            cx,
            coroutine,
            body,
            async_body,
            origin.guard_ty,
            block,
            &mut worklist,
        );
    }
    worklist.live_variants
}

/// Apply one block transfer and queue successors whose joined state changed.
fn process_guard_block<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: Ty<'tcx>,
    block: BasicBlock,
    worklist: &mut GuardWorklist,
) {
    // Each queue entry reads the joined state accumulated from reachable predecessors.
    let Some(data) = body.basic_blocks.get(block) else {
        return;
    };
    let mut state = worklist
        .incoming_states
        .get(block.index())
        .and_then(Option::as_ref)
        .cloned()
        .unwrap_or_default();
    // Apply statements in MIR order before testing this block's terminator.
    for statement in &data.statements {
        update_guard_state(body, async_body, statement, &mut state);
    }

    // A warning requires this guard in a saved wrapper at a Poll::Pending return.
    if let Some(variant) =
        pending_guard_variant(cx, coroutine, body, async_body, block, &state, guard_ty)
        && !worklist.live_variants.contains(&variant)
    {
        worklist.live_variants.push(variant);
    }

    consume_terminator_operands(
        cx,
        body,
        async_body,
        guard_ty,
        &data.terminator().kind,
        &mut state,
    );
    // Revisit successors only when their joined may-live state gains a fact.
    for successor in data.terminator().successors() {
        let Some(successor_state) = worklist.incoming_states.get_mut(successor.index()) else {
            continue;
        };
        if has_new_state_facts(successor_state, &state) {
            worklist.pending_blocks.push_back(successor);
        }
    }
}

/// Check that the tracked guard occupies a wrapper field in the pending coroutine state.
fn saved_wrapper_contains_guard<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    pending: VariantIdx,
    place: &GuardPlace,
    guard_ty: Ty<'tcx>,
) -> bool {
    let GuardBase::CoroutineField { variant, field } = place.base else {
        return false;
    };
    // A field from another saved state does not belong to this pending return.
    if variant != pending {
        return false;
    }
    let Some(saved_local) = coroutine
        .variant_fields
        .get(variant)
        .and_then(|fields| fields.get(field))
    else {
        return false;
    };
    let Some(saved_ty) = coroutine.field_tys.get(*saved_local).map(|cause| cause.ty) else {
        return false;
    };
    // Direct guard fields already use the established diagnostic path.
    saved_ty != guard_ty && type_contains_guard(cx, saved_ty, guard_ty)
}

/// Return the pending state when it saves this guard in an owning wrapper.
fn pending_guard_variant<'tcx>(
    cx: &LateContext<'tcx>,
    coroutine: &rustc_middle::mir::CoroutineLayout<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    block: BasicBlock,
    state: &GuardFlowState,
    guard_ty: Ty<'tcx>,
) -> Option<VariantIdx> {
    let data = body.basic_blocks.get(block)?;
    // Require a pending return before reading the saved-state discriminant.
    if !matches!(&data.terminator().kind, TerminatorKind::Return) || !returns_poll_pending(cx, data)
    {
        return None;
    }

    // The latest discriminant identifies the fields retained by this suspension.
    let variant = data.statements.iter().rev().find_map(|statement| {
        let StatementKind::SetDiscriminant {
            place,
            variant_index,
        } = &statement.kind
        else {
            return None;
        };
        matches!(
            place.ty(body, cx.tcx).ty.kind(),
            TyKind::Coroutine(def_id, _) if *def_id == async_body.to_def_id()
        )
        .then_some(*variant_index)
    })?;

    // Only a tracked place in this saved variant can trigger the wrapper diagnostic.
    state
        .places
        .iter()
        .any(|place| saved_wrapper_contains_guard(cx, coroutine, variant, place, guard_ty))
        .then_some(variant)
}

/// Check that the block assigns the language item's pending variant to its return place.
fn returns_poll_pending<'tcx>(
    cx: &LateContext<'tcx>,
    data: &rustc_middle::mir::BasicBlockData<'tcx>,
) -> bool {
    let span = data.terminator().source_info.span;
    // Resolve Poll and Pending through rustc's language-item table.
    let poll_def_id = cx.tcx.require_lang_item(LangItem::Poll, span);
    let pending_def_id = cx.tcx.require_lang_item(LangItem::PollPending, span);
    let pending_variant = cx
        .tcx
        .adt_def(poll_def_id)
        .variants()
        .iter_enumerated()
        .find_map(|(variant, definition)| (definition.def_id == pending_def_id).then_some(variant));
    let Some(pending_variant) = pending_variant else {
        return false;
    };
    // The return local must aggregate the exact Pending variant.
    data.statements.iter().any(|statement| {
        let StatementKind::Assign(assignment) = &statement.kind else {
            return false;
        };
        let (destination, value) = &**assignment;
        let Some(local) = destination.as_local() else {
            return false;
        };
        if local != Local::from_usize(0) {
            return false;
        }
        let Rvalue::Aggregate(kind, _) = value else {
            return false;
        };
        matches!(
            &**kind,
            AggregateKind::Adt(def_id, variant, ..)
                if *def_id == poll_def_id && *variant == pending_variant
        )
    })
}

/// A MIR place path that currently owns the tracing guard.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GuardPlace {
    /// Base local or compiler-generated saved field containing this guard.
    base: GuardBase,
    /// Aggregate fields traversed from the base to the guard value.
    fields: Vec<FieldIdx>,
}

/// Base identity for a local or coroutine state field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuardBase {
    /// A body-local MIR variable.
    Local(Local),
    /// A field saved by one state of the coroutine.
    CoroutineField {
        /// Coroutine state that contains the saved field.
        variant: VariantIdx,
        /// Field within the selected coroutine state.
        field: FieldIdx,
    },
}

/// Guard values and references that may reach a MIR block.
#[derive(Clone, Default)]
struct GuardFlowState {
    /// MIR places that may own the guard value or a wrapper containing it.
    places: Vec<GuardPlace>,
    /// Reference locals and the container places they borrow.
    references: HashMap<Local, Vec<GuardPlace>>,
}

/// Mutable state for the may-live control-flow worklist.
struct GuardWorklist {
    /// Joined ownership and alias facts for each MIR block.
    incoming_states: Vec<Option<GuardFlowState>>,
    /// MIR blocks whose joined state changed and needs transfer.
    pending_blocks: VecDeque<BasicBlock>,
    /// Pending coroutine variants where the tracked guard remains saved.
    live_variants: Vec<VariantIdx>,
}

/// Convert a MIR place into its local or coroutine-field identity.
fn guard_place<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    place: Place<'tcx>,
) -> GuardPlace {
    // A root local needs no aggregate projection path.
    if let Some(local) = place.as_local() {
        return local_guard_place(local);
    }

    let mut projection = place.projection.iter();
    let first = projection.next();
    let second = projection.next();
    let third = projection.next();
    let Some(local_decl) = body.local_decls.get(place.local) else {
        // Keep tracking rooted locally when rustc supplies no declaration metadata.
        return local_guard_place(place.local);
    };

    // Resolve the compiler's coroutine storage projection before interpreting local fields.
    if let (
        Some(ProjectionElem::Deref),
        Some(ProjectionElem::Downcast(_, variant)),
        Some(ProjectionElem::Field(field, _)),
        TyKind::Ref(_, pointee, _),
    ) = (first, second, third, local_decl.ty.kind())
        && matches!(pointee.kind(), TyKind::Coroutine(def_id, _) if *def_id == async_body.to_def_id())
    {
        return GuardPlace {
            base: GuardBase::CoroutineField { variant, field },
            fields: projection
                .filter_map(projection_field)
                .take(MAX_GUARD_FIELD_DEPTH + 1)
                .collect(),
        };
    }

    GuardPlace {
        base: GuardBase::Local(place.local),
        fields: place
            .projection
            .iter()
            .filter_map(projection_field)
            .take(MAX_GUARD_FIELD_DEPTH + 1)
            .collect(),
    }
}

/// Make a guard place rooted at a local variable.
const fn local_guard_place(local: Local) -> GuardPlace {
    GuardPlace {
        base: GuardBase::Local(local),
        fields: Vec::new(),
    }
}

/// Retain only aggregate field projections from a MIR path.
const fn projection_field(projection: ProjectionElem<Local, Ty<'_>>) -> Option<FieldIdx> {
    if let ProjectionElem::Field(field, _) = projection {
        Some(field)
    } else {
        None
    }
}

/// Check whether a tracked place is nested within a moved source place.
fn guard_place_is_within(tracked: &GuardPlace, moved: &GuardPlace) -> bool {
    tracked.base == moved.base && tracked.fields.starts_with(&moved.fields)
}

/// Move tracked child fields from a source place into a destination place.
fn transfer_guard_places(
    state: &[GuardPlace],
    source: &GuardPlace,
    destination: &GuardPlace,
    aggregate_field: Option<FieldIdx>,
) -> Vec<GuardPlace> {
    state
        .iter()
        .filter_map(|tracked| {
            if !guard_place_is_within(tracked, source) {
                return None;
            }
            let suffix = tracked.fields.get(source.fields.len()..)?;
            let mut moved = destination.clone();
            if let Some(field) = aggregate_field {
                moved.fields.push(field);
            }
            if moved.fields.len() + suffix.len() > MAX_GUARD_FIELD_DEPTH {
                return None;
            }
            moved.fields.extend_from_slice(suffix);
            Some(moved)
        })
        .collect()
}

/// Apply assignments and storage boundaries to tracked guard ownership.
fn update_guard_state<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    statement: &Statement<'tcx>,
    state: &mut GuardFlowState,
) {
    match &statement.kind {
        StatementKind::Assign(assignment) => {
            // Transfer moved fields before replacing the destination's old owner.
            let (destination, value) = &**assignment;
            let destination_place = *destination;
            let destination = guard_place(body, async_body, destination_place);
            let moved_to = moved_guard_places(body, async_body, value, &state.places);
            let mut carried_places = Vec::new();
            for (source, field) in moved_to {
                let moved = transfer_guard_places(&state.places, &source, &destination, field);
                if !moved.is_empty() {
                    clear_guard_place(&mut state.places, &source);
                    carried_places.extend(moved);
                }
            }
            clear_guard_place(&mut state.places, &destination);
            insert_guard_places(&mut state.places, carried_places);
            // Reference aliases must observe the ownership state after the assignment.
            update_reference_state(body, async_body, destination_place, value, state);
        }
        StatementKind::StorageDead(local) | StatementKind::StorageLive(local) => {
            // Storage boundaries end local ownership and any aliases rooted there.
            clear_guard_place(&mut state.places, &local_guard_place(*local));
            let _discarded_reference_targets = state.references.remove(local);
        }
        StatementKind::FakeRead(_)
        | StatementKind::SetDiscriminant { .. }
        | StatementKind::PlaceMention(_)
        | StatementKind::AscribeUserType(..)
        | StatementKind::Coverage(_)
        | StatementKind::Intrinsic(_)
        | StatementKind::ConstEvalCounter
        | StatementKind::Nop
        | StatementKind::BackwardIncompatibleDropHint { .. } => {
            // These metadata statements leave owner places and alias sets unchanged.
        }
    }
}

/// Remember a reference local's backing place for resolved container extraction calls.
fn update_reference_state<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    destination: Place<'tcx>,
    value: &Rvalue<'tcx>,
    state: &mut GuardFlowState,
) {
    // Only reference-typed locals can carry aliases to existing guard owners.
    let Some(local) = destination.as_local() else {
        return;
    };
    let Some(local_decl) = body.local_decls.get(local) else {
        return;
    };
    if !matches!(local_decl.ty.kind(), TyKind::Ref(..)) {
        let _discarded_reference_targets = state.references.remove(&local);
        return;
    }
    // Propagate known targets through reborrows and copies of reference locals.
    let targets = if let Rvalue::Ref(_, _, borrowed) = value {
        borrowed_guard_places(body, async_body, *borrowed, &state.references)
    } else if let Rvalue::Use(
        rustc_middle::mir::Operand::Copy(source) | rustc_middle::mir::Operand::Move(source),
        _,
    ) = value
    {
        source
            .as_local()
            .and_then(|source| state.references.get(&source).cloned())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // An empty set records no proven target, so remove any previous mapping.
    if targets.is_empty() {
        let _discarded_reference_targets = state.references.remove(&local);
    } else {
        let _replaced_reference_targets = state.references.insert(local, targets);
    }
}

/// Resolve a reborrow through any tracked reference local to its backing place.
fn borrowed_guard_places<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    borrowed: Place<'tcx>,
    references: &HashMap<Local, Vec<GuardPlace>>,
) -> Vec<GuardPlace> {
    let mut projection = borrowed.projection.iter();
    if matches!(projection.next(), Some(ProjectionElem::Deref))
        && let Some(targets) = references.get(&borrowed.local)
    {
        let suffix = projection
            .filter_map(projection_field)
            .take(MAX_GUARD_FIELD_DEPTH + 1)
            .collect::<Vec<_>>();
        // Apply fields projected after the dereference to each possible backing place.
        return targets
            .iter()
            .filter_map(|target| {
                if target.fields.len() + suffix.len() > MAX_GUARD_FIELD_DEPTH {
                    return None;
                }
                let mut target = target.clone();
                target.fields.extend_from_slice(&suffix);
                Some(target)
            })
            .collect();
    }
    vec![guard_place(body, async_body, borrowed)]
}

/// Find moved guard paths carried by a MIR assignment value.
fn moved_guard_places<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    value: &Rvalue<'tcx>,
    state: &[GuardPlace],
) -> Vec<(GuardPlace, Option<FieldIdx>)> {
    let operands: Vec<(&rustc_middle::mir::Operand<'_>, Option<FieldIdx>)> = match value {
        Rvalue::Use(operand, _) => vec![(operand, None)],
        Rvalue::Aggregate(kind, operands) => operands
            .iter()
            .enumerate()
            .filter_map(|(index, operand)| {
                let field = matches!(&**kind, AggregateKind::Tuple | AggregateKind::Adt(..))
                    .then(|| FieldIdx::from_usize(index));
                matches!(operand, rustc_middle::mir::Operand::Move(_)).then_some((operand, field))
            })
            .collect(),
        Rvalue::Repeat(..)
        | Rvalue::Ref(..)
        | Rvalue::ThreadLocalRef(_)
        | Rvalue::RawPtr(..)
        | Rvalue::Cast(..)
        | Rvalue::BinaryOp(..)
        | Rvalue::UnaryOp(..)
        | Rvalue::Discriminant(_)
        | Rvalue::CopyForDeref(_)
        | Rvalue::WrapUnsafeBinder(..)
        | Rvalue::Reborrow(..) => Vec::new(),
    };
    operands
        .into_iter()
        .filter_map(|(operand, field)| match operand {
            rustc_middle::mir::Operand::Move(place) => {
                Some((guard_place(body, async_body, *place), field))
            }
            rustc_middle::mir::Operand::Copy(_)
            | rustc_middle::mir::Operand::Constant(_)
            | rustc_middle::mir::Operand::RuntimeChecks(_) => None,
        })
        .filter(|(moved, _)| {
            state
                .iter()
                .any(|tracked| guard_place_is_within(tracked, moved))
        })
        .collect()
}

/// Remove consumed guard operands or transfer them through owned call results.
fn consume_terminator_operands<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: Ty<'tcx>,
    kind: &TerminatorKind<'tcx>,
    state: &mut GuardFlowState,
) {
    match kind {
        TerminatorKind::Call { .. } => {
            // Call handling transfers moved operands and invalidates opaque mutable aliases.
            consume_call_operands(cx, body, async_body, guard_ty, kind, state);
        }
        TerminatorKind::Drop { place, .. } => {
            // A drop removes both the owner and aliases whose backing place is that owner.
            let place = guard_place(body, async_body, *place);
            clear_guard_place(&mut state.places, &place);
            clear_reference_targets(&mut state.references, &place);
        }
        TerminatorKind::Goto { .. }
        | TerminatorKind::SwitchInt { .. }
        | TerminatorKind::UnwindResume
        | TerminatorKind::UnwindTerminate(_)
        | TerminatorKind::Return
        | TerminatorKind::Unreachable
        | TerminatorKind::TailCall { .. }
        | TerminatorKind::Assert { .. }
        | TerminatorKind::Yield { .. }
        | TerminatorKind::CoroutineDrop
        | TerminatorKind::FalseEdge { .. }
        | TerminatorKind::FalseUnwind { .. }
        | TerminatorKind::InlineAsm { .. } => {}
    }
}

/// Transfer moved guard state through a call only when its output can own the guard.
fn consume_call_operands<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: Ty<'tcx>,
    kind: &TerminatorKind<'tcx>,
    state: &mut GuardFlowState,
) {
    let TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = kind
    else {
        return;
    };
    let destination_path = guard_place(body, async_body, *destination);
    // Handle the recognized extraction before conservative opaque-call effects.
    let mut carried_places = transfer_option_take(cx, body, async_body, kind, state);

    // Opaque calls can consume a guard through a mutable reference; shared borrows remain tracked.
    if !is_mem_drop(cx, func) && !is_option_take(cx, func) {
        apply_opaque_mutable_borrow_policy(cx, body, args, state);
    }
    carried_places.extend(consume_moved_call_inputs(
        cx, body, async_body, guard_ty, kind, state,
    ));
    // Replace stale destination ownership only after all sources have been examined.
    clear_guard_place(&mut state.places, &destination_path);
    insert_guard_places(&mut state.places, carried_places);
}

/// Transfer a resolved `Option::take` result through its tracked reference target.
fn transfer_option_take<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    kind: &TerminatorKind<'tcx>,
    state: &mut GuardFlowState,
) -> Vec<GuardPlace> {
    let TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = kind
    else {
        return Vec::new();
    };
    if !is_option_take(cx, func) {
        return Vec::new();
    }
    // Resolve the receiver local to the wrappers that a tracked reborrow may target.
    let destination_path = guard_place(body, async_body, *destination);
    let receiver = args.first().and_then(|argument| match &argument.node {
        rustc_middle::mir::Operand::Copy(place) | rustc_middle::mir::Operand::Move(place) => {
            place.as_local()
        }
        rustc_middle::mir::Operand::Constant(_) | rustc_middle::mir::Operand::RuntimeChecks(_) => {
            None
        }
    });
    let Some(targets) = receiver.and_then(|local| state.references.get(&local).cloned()) else {
        return Vec::new();
    };

    // Clear only a proven single target; joined aliases lose branch correlation.
    let has_one_target = targets.len() == 1;
    let mut carried_places = Vec::new();
    // Transfer a possible result while preserving ambiguous source correlation.
    for source in targets {
        let moved = transfer_guard_places(&state.places, &source, &destination_path, None);
        if moved.is_empty() {
            continue;
        }
        // Keep may-live ownership when alias correlation leaves multiple possible targets.
        if has_one_target {
            clear_guard_place(&mut state.places, &source);
        }
        carried_places.extend(moved);
    }
    carried_places
}

/// Consume moved call inputs and transfer a guard through the resolved `Box::new` constructor.
fn consume_moved_call_inputs<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: Ty<'tcx>,
    kind: &TerminatorKind<'tcx>,
    state: &mut GuardFlowState,
) -> Vec<GuardPlace> {
    let TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = kind
    else {
        return Vec::new();
    };
    let destination_path = guard_place(body, async_body, *destination);
    let destination_ty = destination.ty(body, cx.tcx).ty;
    // Confirm the Box result can own this guard before preserving nested paths.
    let result_is_box = is_box_new(cx, func) && type_contains_guard(cx, destination_ty, guard_ty);
    let mut carried_places = Vec::new();

    // Only moved arguments can transfer tracked ownership to a call result.
    for argument in args {
        let rustc_middle::mir::Operand::Move(place) = &argument.node else {
            continue;
        };
        let source = guard_place(body, async_body, *place);
        if !state
            .places
            .iter()
            .any(|tracked| guard_place_is_within(tracked, &source))
        {
            continue;
        }
        if result_is_box {
            // Box::new moves each tracked descendant into the returned owning Box.
            // Preserve the tracked field suffix when `Box::new` owns a wrapper.
            carried_places.extend(transfer_guard_places(
                &state.places,
                &source,
                &destination_path,
                None,
            ));
        }
        clear_guard_place(&mut state.places, &source);
        clear_reference_targets(&mut state.references, &source);
    }
    carried_places
}

/// Clear guard paths exposed to an opaque mutable-reference argument.
fn apply_opaque_mutable_borrow_policy<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    args: &[Spanned<rustc_middle::mir::Operand<'tcx>>],
    state: &mut GuardFlowState,
) {
    // Shared and by-value arguments do not hide mutation of a tracked owner.
    for argument in args {
        let (rustc_middle::mir::Operand::Copy(place) | rustc_middle::mir::Operand::Move(place)) =
            &argument.node
        else {
            continue;
        };
        if !matches!(
            place.ty(body, cx.tcx).ty.kind(),
            TyKind::Ref(_, _, rustc_hir::Mutability::Mut)
        ) {
            continue;
        }
        // Only direct mutable-reference locals have a resolved alias mapping.
        let Some(local) = place.as_local() else {
            continue;
        };
        let Some(targets) = state.references.get(&local).cloned() else {
            continue;
        };
        // An opaque mutable call may remove a guard reachable through any such alias.
        for target in targets {
            clear_guard_place(&mut state.places, &target);
        }
    }
}

/// Identify `mem::drop`, which consumes a reference value without mutating its referent.
fn is_mem_drop(cx: &LateContext<'_>, func: &rustc_middle::mir::Operand<'_>) -> bool {
    let Some((def_id, _)) = func.const_fn_def() else {
        return false;
    };
    let path = cx.tcx.def_path_str(def_id);
    path.starts_with("core::mem::drop") || path.starts_with("std::mem::drop")
}

/// Identify the resolved standard Box constructor that owns its argument.
fn is_box_new(cx: &LateContext<'_>, func: &rustc_middle::mir::Operand<'_>) -> bool {
    let Some((def_id, _)) = func.const_fn_def() else {
        return false;
    };
    let path = cx.tcx.def_path_str(def_id);
    cx.tcx.crate_name(def_id.krate).as_str() == "alloc"
        && path.contains("::boxed::Box::<")
        && path.ends_with("::new")
}

/// Identify the standard resolved `Option::take` method.
fn is_option_take(cx: &LateContext<'_>, func: &rustc_middle::mir::Operand<'_>) -> bool {
    let Some((def_id, _)) = func.const_fn_def() else {
        return false;
    };
    let path = cx.tcx.def_path_str(def_id);
    cx.tcx.crate_name(def_id.krate).as_str() == "core"
        && path.contains("::option::Option::<")
        && path.ends_with("::take")
}

/// Check whether a type physically owns the tracked guard in a supported wrapper.
fn type_contains_guard<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, guard_ty: Ty<'tcx>) -> bool {
    type_contains_guard_with_seen(cx, ty, guard_ty, &mut HashSet::new())
}

/// Inspect stored fields and owning Box payloads under recursive-type protection.
fn type_contains_guard_with_seen<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    guard_ty: Ty<'tcx>,
    seen: &mut HashSet<Ty<'tcx>>,
) -> bool {
    if ty == guard_ty {
        return true;
    }
    if seen.len() >= MAX_GUARD_FIELD_DEPTH || !seen.insert(ty) {
        return false;
    }
    // Inspect only supported owning containers and explicitly excluded type kinds.
    let has_guard = match ty.kind() {
        TyKind::Adt(..) => type_contains_guard_in_adt(cx, ty, guard_ty, seen),
        TyKind::Tuple(elements) => elements
            .iter()
            .any(|element| type_contains_guard_with_seen(cx, element, guard_ty, seen)),
        TyKind::Array(..)
        | TyKind::Slice(..)
        | TyKind::Ref(..)
        | TyKind::RawPtr(..)
        | TyKind::Bool
        | TyKind::Char
        | TyKind::Int(_)
        | TyKind::Uint(_)
        | TyKind::Float(_)
        | TyKind::Foreign(_)
        | TyKind::Str
        | TyKind::Pat(..)
        | TyKind::FnDef(..)
        | TyKind::FnPtr(..)
        | TyKind::UnsafeBinder(_)
        | TyKind::Dynamic(..)
        | TyKind::Closure(..)
        | TyKind::CoroutineClosure(..)
        | TyKind::Coroutine(..)
        | TyKind::CoroutineWitness(..)
        | TyKind::Never
        | TyKind::Alias(..)
        | TyKind::Param(_)
        | TyKind::Bound(..)
        | TyKind::Placeholder(_)
        | TyKind::Infer(_)
        | TyKind::Error(_) => false,
    };
    // Remove this node from the active path so sibling fields can inspect it.
    let _removed_active_type = seen.remove(&ty);
    has_guard
}

/// Inspect ADT fields or the type parameter physically owned by `Box`.
fn type_contains_guard_in_adt<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    guard_ty: Ty<'tcx>,
    seen: &mut HashSet<Ty<'tcx>>,
) -> bool {
    let TyKind::Adt(adt, arguments) = ty.kind() else {
        return false;
    };
    let path = cx.tcx.def_path_str(adt.did());
    // Box owns its type argument; other ADTs expose ownership through declared fields.
    if cx.tcx.crate_name(adt.did().krate).as_str() == "alloc" && path.ends_with("boxed::Box") {
        arguments
            .types()
            .any(|argument| type_contains_guard_with_seen(cx, argument, guard_ty, seen))
    } else {
        // Instantiate each field with this ADT's arguments before checking its type.
        adt.all_fields().any(|field| {
            let field_ty = cx
                .tcx
                .type_of(field.did)
                .instantiate(cx.tcx, arguments)
                .skip_norm_wip();
            type_contains_guard_with_seen(cx, field_ty, guard_ty, seen)
        })
    }
}

/// Add new may-live places while keeping the flow state set-like.
fn insert_guard_places(target: &mut Vec<GuardPlace>, additions: Vec<GuardPlace>) {
    // Preserve insertion order while keeping the may-live places set-like.
    let mut additions = additions.into_iter();
    // Each search sees paths inserted by earlier iterations.
    while let Some(place) = additions.find(|place| !target.contains(place)) {
        target.push(place);
    }
}

/// Join a block's may-live places and aliases from a new predecessor.
fn has_new_state_facts(target: &mut Option<GuardFlowState>, incoming: &GuardFlowState) -> bool {
    let Some(target) = target else {
        // The first predecessor provides the initial state for this block.
        *target = Some(incoming.clone());
        return true;
    };
    let mut has_new_facts = false;
    // Ownership facts are joined monotonically until the worklist reaches a fixed point.
    for place in &incoming.places {
        if !target.places.contains(place) {
            target.places.push(place.clone());
            has_new_facts = true;
        }
    }
    // Reference aliases use the same may-live union as owned guard paths.
    for (local, places) in &incoming.references {
        let targets = target.references.entry(*local).or_default();
        for place in places {
            if !targets.contains(place) {
                targets.push(place.clone());
                has_new_facts = true;
            }
        }
    }
    // Revisit successors only when this join added a fact.
    has_new_facts
}

/// Remove a tracked place and all guard fields nested below it.
fn clear_guard_place(state: &mut Vec<GuardPlace>, place: &GuardPlace) {
    state
        .retain(|current| current.base != place.base || !current.fields.starts_with(&place.fields));
}

/// Clear aliases when their backing local is dropped or replaced.
fn clear_reference_targets(references: &mut HashMap<Local, Vec<GuardPlace>>, place: &GuardPlace) {
    if let GuardBase::Local(local) = place.base {
        let _discarded_reference_targets = references.remove(&local);
    }
}
