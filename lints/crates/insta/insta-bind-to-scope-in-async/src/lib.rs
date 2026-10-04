#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks whether an Insta scope guard may survive an async suspension.
//! The lint matches each resolved `bind_to_scope` call to its MIR result and
//! follows moves, aggregate fields, and owned wrappers through coroutine state.
//! It warns when a tracked value may occupy a saved field at `Poll::Pending`.
//! Mutable-container effects remain conservative because MIR does not identify
//! which collection element an opaque extraction method returned.

extern crate rustc_abi;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use insta_support::{SettingsMethodCondition, settings_method_violation};
use rustc_abi::{FieldIdx, VariantIdx};
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Closure, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, LangItem, Node,
    def_id::{DefId, LocalDefId},
};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_middle::mir::{
    AggregateKind, BasicBlock, Local, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::TyKind;
use rustc_span::{Span, Spanned};
use std::collections::{HashMap, HashSet, VecDeque};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_BIND_TO_SCOPE_IN_ASYNC,
    Warn,
    "an Insta Settings scope guard is live across an async suspension point",
    InstaBindToScopeInAsync
}

impl<'tcx> LateLintPass<'tcx> for InstaBindToScopeInAsync {
    /// Check resolved Insta calls against the enclosing coroutine's saved locals.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Require the crate-resolved settings API and async-body condition.
        let Some(call) = settings_method_violation(
            cx,
            expr,
            "bind_to_scope",
            SettingsMethodCondition::InAsyncBody,
        ) else {
            return;
        };
        let Some(async_body) = enclosing_async_body(cx, expr) else {
            return;
        };
        // Analyze the exact guard type returned by this HIR expression.
        let guard_ty = cx.typeck_results().expr_ty(expr);
        if !is_guard_live_at_suspension(cx, async_body, expr, guard_ty) {
            return;
        }

        cx.emit_span_lint(
            INSTA_BIND_TO_SCOPE_IN_ASYNC,
            call.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this thread-local guard can cross async scheduling points")
                    .help("use `Settings::bind_async`");
            }),
        );
    }
}

/// Return the nearest enclosing async body for a resolved method call.
fn enclosing_async_body(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<LocalDefId> {
    // The nearest closure boundary determines which coroutine owns this call.
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        let Node::Expr(parent) = node else {
            continue;
        };
        let ExprKind::Closure(Closure { def_id, kind, .. }) = parent.kind else {
            continue;
        };
        return is_async_closure(*kind).then_some(*def_id);
    }
    // Calls outside closures have no async owner.
    None
}

/// Report whether a closure kind creates an async coroutine body.
const fn is_async_closure(kind: ClosureKind) -> bool {
    matches!(
        kind,
        ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
            | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
    )
}

/// Identify a value flowing from this resolved call to a saved field at `Poll::Pending`.
fn is_guard_live_at_suspension<'tcx>(
    cx: &LateContext<'tcx>,
    async_body: LocalDefId,
    expr: &Expr<'_>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
) -> bool {
    // Optimized MIR preserves the runtime call shape, including optimized async bodies.
    let body = cx.tcx.optimized_mir(async_body);
    let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    let Some((block, target, result)) =
        body.basic_blocks
            .iter_enumerated()
            .find_map(|(block, data)| {
                // Match source span and resolved method to distinguish same-typed calls.
                let terminator = data.terminator();
                if !terminator.source_info.span.contains(expr.span) {
                    return None;
                }
                let TerminatorKind::Call {
                    func,
                    destination,
                    target: Some(target),
                    ..
                } = &terminator.kind
                else {
                    return None;
                };
                (func
                    .const_fn_def()
                    .is_some_and(|(def_id, _)| def_id == method_def_id))
                .then_some((block, *target, *destination))
            })
    else {
        return false;
    };
    let origin = GuardOrigin {
        block,
        target,
        destination: guard_place(body, async_body, result),
    };
    has_guard_crossed_pending_poll(cx, body, async_body, guard_ty, &origin)
}

/// Origin block, normal successor, and MIR place returned by the resolved Insta call.
#[derive(Clone)]
struct GuardOrigin {
    /// The block containing the matched Insta call.
    block: BasicBlock,
    /// The normal block reached after that call returns.
    target: BasicBlock,
    /// The tracked identity of the call's result place.
    destination: GuardPlace,
}

/// Follow the call result through the coroutine CFG until it drops or crosses a suspension.
fn has_guard_crossed_pending_poll<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    origin: &GuardOrigin,
) -> bool {
    // Start before the guard binding, then process each block after its state grows.
    let mut incoming = vec![None; body.basic_blocks.len()];
    let entry = BasicBlock::from_usize(0);
    let Some(entry_state) = incoming.get_mut(entry.index()) else {
        return false;
    };
    *entry_state = Some(GuardFlowState::default());
    let mut worklist = VecDeque::from([entry]);
    while let Some(block) = worklist.pop_front() {
        // Read the joined incoming state; unvisited blocks begin without this origin.
        let state = incoming
            .get(block.index())
            .and_then(Option::as_ref)
            .cloned()
            .unwrap_or_default();
        // A pending return with the place initialized proves the guard crosses a poll.
        let Some(edges) = advance_guard_state(cx, body, async_body, guard_ty, origin, block, state)
        else {
            return true;
        };
        for (successor, edge_state) in edges {
            let Some(successor_state) = incoming.get_mut(successor.index()) else {
                continue;
            };
            if has_guard_state_changed(successor_state, &edge_state) {
                // Reprocess a successor only when its may-live state grows.
                worklist.push_back(successor);
            }
        }
    }
    false
}

/// Update a block's guard state and return each successor's joined input state.
fn advance_guard_state<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    origin: &GuardOrigin,
    block: BasicBlock,
    mut state: GuardFlowState,
) -> Option<Vec<(BasicBlock, GuardFlowState)>> {
    // Apply statements in MIR order so assignments and drops affect the exit state.
    let data = body.basic_blocks.get(block)?;
    for statement in &data.statements {
        update_guard_state(cx, body, async_body, guard_ty, statement, &mut state);
    }
    if is_guard_live_at_pending(cx, body, async_body, block, &state.places) {
        return None;
    }

    let terminator = data.terminator();
    consume_terminator_operands(cx, body, async_body, guard_ty, &terminator.kind, &mut state);
    // Give each successor its own state and seed the matched call result on its return edge.
    Some(
        terminator
            .successors()
            .map(|successor| {
                let mut edge_state = state.clone();
                if block == origin.block && successor == origin.target {
                    clear_guard_place(&mut edge_state.places, &origin.destination);
                    edge_state.places.push(origin.destination.clone());
                }
                (successor, edge_state)
            })
            .collect(),
    )
}

/// Check whether a guard-containing coroutine field is initialized at `Poll::Pending`.
fn is_guard_live_at_pending<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    block: BasicBlock,
    places: &[GuardPlace],
) -> bool {
    // Witness fields alone overapproximate storage, so require a tracked initialized origin.
    let Some(coroutine) = cx.tcx.mir_coroutine_witnesses(async_body) else {
        return false;
    };
    pending_variant(cx, body, async_body, block).is_some_and(|variant| {
        places.iter().any(|place| {
            matches!(place, GuardPlace { base: GuardBase::CoroutineField { variant: field_variant, field }, .. }
                if *field_variant == variant
                    && coroutine.variant_fields.get(variant)
                        .and_then(|fields| fields.get(*field))
                        .is_some())
        })
    })
}

/// Identify a tracked binding and its field path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GuardPlace {
    /// The local or coroutine field that owns the tracked value.
    base: GuardBase,
    /// Fields inside that local or saved coroutine field.
    fields: Vec<FieldIdx>,
}

/// Storage location that owns a tracked guard value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuardBase {
    /// A MIR local.
    Local(Local),
    /// A field saved in one coroutine state variant.
    CoroutineField {
        /// The coroutine state variant that owns the field.
        variant: VariantIdx,
        /// The field index within that coroutine state variant.
        field: FieldIdx,
    },
}

/// Track the binding's guard through local moves and reference-backed containers.
#[derive(Clone, Default)]
struct GuardFlowState {
    /// Locals and coroutine fields that hold this guard or a container that retains it.
    places: Vec<GuardPlace>,
    /// MIR reference locals and the places they borrow.
    references: HashMap<Local, Vec<GuardPlace>>,
}

/// Convert a MIR place into the identity used by the guard flow analysis.
fn guard_place<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    place: Place<'tcx>,
) -> GuardPlace {
    // Keep locals distinct from fields saved in the coroutine state.
    if let Some(local) = place.as_local() {
        return local_guard_place(local);
    }

    let mut projection = place.projection.iter();
    let first = projection.next();
    let second = projection.next();
    let third = projection.next();
    let Some(local_decl) = body.local_decls.get(place.local) else {
        return local_guard_place(place.local);
    };

    // Recognize the compiler's saved coroutine-field projection before local projections.
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
            fields: projection.filter_map(projection_field).collect(),
        };
    }

    // Retain field indices while ignoring dereferences and dynamic projections.
    GuardPlace {
        base: GuardBase::Local(place.local),
        fields: place
            .projection
            .iter()
            .filter_map(projection_field)
            .collect(),
    }
}

/// Make the root identity for a MIR local.
const fn local_guard_place(local: Local) -> GuardPlace {
    GuardPlace {
        base: GuardBase::Local(local),
        fields: Vec::new(),
    }
}

/// Keep field projections and ignore other MIR projections.
const fn projection_field(
    projection: ProjectionElem<Local, rustc_middle::ty::Ty<'_>>,
) -> Option<FieldIdx> {
    if let ProjectionElem::Field(field, _) = projection {
        Some(field)
    } else {
        None
    }
}

/// Read the underlying local for reference-alias bookkeeping.
const fn guard_place_local(place: &GuardPlace) -> Option<Local> {
    match place.base {
        GuardBase::Local(local) => Some(local),
        GuardBase::CoroutineField { .. } => None,
    }
}

/// Check whether a moved parent place contains the tracked field path.
fn guard_place_is_within(tracked: &GuardPlace, moved: &GuardPlace) -> bool {
    tracked.base == moved.base && tracked.fields.starts_with(&moved.fields)
}

/// Transfer tracked descendants while retaining their path inside the moved source.
fn transfer_guard_places(
    state: &[GuardPlace],
    source: &GuardPlace,
    destination: &GuardPlace,
    aggregate_field: Option<FieldIdx>,
) -> Vec<GuardPlace> {
    // Only a tracked descendant of the moved source can be transferred.
    state
        .iter()
        .filter_map(|tracked| {
            if !guard_place_is_within(tracked, source) {
                return None;
            }
            let suffix = tracked.fields.get(source.fields.len()..)?;
            let mut moved = destination.clone();
            // Add the aggregate field, then retain the source-relative nested path.
            if let Some(field) = aggregate_field {
                moved.fields.push(field);
            }
            moved.fields.extend_from_slice(suffix);
            // Keep each tracked origin separate at its new owner.
            Some(moved)
        })
        .collect()
}

/// Apply a MIR assignment, storage end, or deinitialization to the tracked origin.
fn update_guard_state<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    statement: &rustc_middle::mir::Statement<'tcx>,
    state: &mut GuardFlowState,
) {
    // Follow move sources before replacing the destination's previous state.
    match &statement.kind {
        StatementKind::Assign(assignment) => {
            let (destination, value) = &**assignment;
            let destination_place = *destination;
            let destination = guard_place(body, async_body, destination_place);
            let moved_to = moved_guard_places(cx, body, async_body, guard_ty, value, &state.places);
            // Preserve nested fields when moving an aggregate into another aggregate.
            let mut carried_places = Vec::new();
            for (source, field) in moved_to {
                let moved = transfer_guard_places(&state.places, &source, &destination, field);
                if !moved.is_empty() {
                    clear_guard_place(&mut state.places, &source);
                    carried_places.extend(moved);
                }
            }
            // Remove overwritten values only after collecting every incoming descendant.
            clear_guard_place(&mut state.places, &destination);
            state.places.extend(carried_places);
            update_reference_state(body, async_body, destination_place, value, state);
        }
        StatementKind::StorageDead(local) | StatementKind::StorageLive(local) => {
            // MIR storage boundaries invalidate values and reference aliases for this local.
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
        | StatementKind::BackwardIncompatibleDropHint { .. } => {}
    }
}

/// Propagate aliases for MIR reference locals used as mutable container receivers.
fn update_reference_state<'tcx>(
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    destination: Place<'tcx>,
    value: &Rvalue<'tcx>,
    state: &mut GuardFlowState,
) {
    // Only direct locals can serve as alias keys for later container method calls.
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
    // Build alias targets from a new borrow or from an existing reference local.
    let targets = if let Rvalue::Ref(_, _, borrowed) = value {
        vec![guard_place(body, async_body, *borrowed)]
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
    // Remove stale aliases when a borrow no longer points to a tracked guard.
    if targets.is_empty() {
        let _discarded_reference_targets = state.references.remove(&local);
    } else {
        let _replaced_reference_targets = state.references.insert(local, targets);
    }
}

/// Find tracked values moved by a MIR rvalue.
fn moved_guard_places<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    value: &Rvalue<'tcx>,
    state: &[GuardPlace],
) -> Vec<(GuardPlace, Option<FieldIdx>)> {
    // Enumerate the operands consumed by each supported rvalue form.
    let operands: Vec<(&rustc_middle::mir::Operand<'tcx>, Option<FieldIdx>)> = match value {
        Rvalue::Use(operand, _)
        | Rvalue::Repeat(operand, _)
        | Rvalue::Cast(_, operand, _)
        | Rvalue::UnaryOp(_, operand)
        | Rvalue::WrapUnsafeBinder(operand, _) => vec![(operand, None)],
        Rvalue::BinaryOp(_, operands) => vec![(&operands.0, None), (&operands.1, None)],
        Rvalue::Aggregate(kind, operands) => operands
            .iter()
            .enumerate()
            .map(|(index, operand)| {
                // Direct aggregate fields preserve the nested source path.
                let field = matches!(&**kind, AggregateKind::Tuple | AggregateKind::Adt(..))
                    .then(|| FieldIdx::from_usize(index));
                (operand, field)
            })
            .collect(),
        Rvalue::Ref(..)
        | Rvalue::ThreadLocalRef(_)
        | Rvalue::RawPtr(..)
        | Rvalue::Discriminant(_)
        | Rvalue::CopyForDeref(_)
        | Rvalue::Reborrow(..) => Vec::new(),
    };
    // Ignore copies of unrelated types and values outside the tracked source path.
    operands
        .into_iter()
        .filter_map(|(operand, field)| match operand {
            rustc_middle::mir::Operand::Move(place) => {
                Some((guard_place(body, async_body, *place), field))
            }
            rustc_middle::mir::Operand::Copy(place) if place.ty(body, cx.tcx).ty == guard_ty => {
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

/// Remove moved values and transfer them to outputs or containers that may
/// retain a guard.
fn consume_terminator_operands<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    kind: &TerminatorKind<'tcx>,
    state: &mut GuardFlowState,
) {
    match kind {
        TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } => consume_call_operands(
            cx,
            body,
            async_body,
            guard_ty,
            (func, args),
            *destination,
            state,
        ),
        TerminatorKind::Drop { place, .. } => {
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

/// Propagate a moved guard through a call result or mutable container receiver.
fn consume_call_operands<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    call_operands: (
        &rustc_middle::mir::Operand<'tcx>,
        &[Spanned<rustc_middle::mir::Operand<'tcx>>],
    ),
    destination: Place<'tcx>,
    state: &mut GuardFlowState,
) {
    // Keep the function and argument list tied to the call being transferred.
    let (func, args) = call_operands;
    // Resolve the callee before propagating fields across an opaque call boundary.
    let identity_source = direct_identity_argument(cx, body, async_body, func, args);
    let has_guard_argument = args.iter().any(|argument| {
        let rustc_middle::mir::Operand::Move(place) = &argument.node else {
            return false;
        };
        let moved = guard_place(body, async_body, *place);
        state
            .places
            .iter()
            .any(|tracked| guard_place_is_within(tracked, &moved))
    });
    // Save mutable-container contents before clearing moved arguments or result origins.
    let has_mutable_receiver_guard =
        has_guard_in_mutable_receivers(cx, body, guard_ty, args, state);
    let has_guard_input = has_guard_argument || has_mutable_receiver_guard;
    let destination_path = guard_place(body, async_body, destination);
    let carried = identity_source
        .as_ref()
        .map(|source| transfer_guard_places(&state.places, source, &destination_path, None))
        .unwrap_or_default();
    // Remove moved sources before installing the call result, including same-place assignments.
    for argument in args {
        let rustc_middle::mir::Operand::Move(source) = &argument.node else {
            continue;
        };
        let source = guard_place(body, async_body, *source);
        clear_guard_place(&mut state.places, &source);
        clear_reference_targets(&mut state.references, &source);
    }
    update_call_destination(
        cx,
        body,
        guard_ty,
        (destination, destination_path),
        has_guard_input,
        carried,
        state,
    );
}

/// Resolve a direct one-argument identity function before preserving field paths.
fn direct_identity_argument<'tcx>(
    cx: &LateContext<'tcx>,
    caller_body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    func: &rustc_middle::mir::Operand<'tcx>,
    args: &[Spanned<rustc_middle::mir::Operand<'tcx>>],
) -> Option<GuardPlace> {
    // The transfer is exact only for one source argument and its own local body.
    let [argument] = args else { return None };
    let (callee, _) = func.const_fn_def()?;
    let local_callee = callee.as_local()?;
    // Only a by-value move can carry the tracked guard into the identity result.
    let rustc_middle::mir::Operand::Move(source) = &argument.node else {
        return None;
    };
    is_direct_identity_body(cx.tcx.optimized_mir(local_callee))
        .then(|| guard_place(caller_body, async_body, *source))
}

/// Prove the callee returns its sole argument without transforming it.
fn is_direct_identity_body(body: &rustc_middle::mir::Body<'_>) -> bool {
    let Some(block) = body.basic_blocks.get(BasicBlock::from_usize(0)) else {
        return false;
    };
    // A body with another argument, block, or exit can transform the value.
    (body.arg_count == 1
        && body.basic_blocks.len() == 1
        && matches!(&block.terminator().kind, TerminatorKind::Return))
        && returns_the_sole_argument(block)
}

/// Verify that only the argument-to-return assignment and storage markers remain.
fn returns_the_sole_argument(block: &rustc_middle::mir::BasicBlockData<'_>) -> bool {
    // Require one return assignment and permit compiler storage bookkeeping.
    let mut has_argument_return = false;
    // Reject a second assignment or any operation beyond the storage markers.
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Assign(_)
                if !has_argument_return && is_argument_return_assignment(statement) =>
            {
                has_argument_return = true;
            }
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            StatementKind::Assign(_)
            | StatementKind::FakeRead(_)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::PlaceMention(_)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(_)
            | StatementKind::Intrinsic(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::BackwardIncompatibleDropHint { .. } => return false,
        }
    }
    has_argument_return
}

/// Check whether one MIR assignment returns its only argument directly.
fn is_argument_return_assignment(statement: &rustc_middle::mir::Statement<'_>) -> bool {
    let StatementKind::Assign(assignment) = &statement.kind else {
        return false;
    };
    let (destination, value) = &**assignment;
    destination.as_local() == Some(Local::from_usize(0))
        && matches!(
            value,
            Rvalue::Use(
                rustc_middle::mir::Operand::Move(source)
                | rustc_middle::mir::Operand::Copy(source),
                _,
            ) if source.as_local() == Some(Local::from_usize(1))
        )
}

/// Preserve guard values retained through mutable container method calls.
fn has_guard_in_mutable_receivers<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    args: &[Spanned<rustc_middle::mir::Operand<'tcx>>],
    state: &mut GuardFlowState,
) -> bool {
    // Resolve each receiver local through the reference-alias map.
    let mut has_tracked_guard = false;
    for argument in args {
        let place = match &argument.node {
            rustc_middle::mir::Operand::Copy(place) | rustc_middle::mir::Operand::Move(place) => {
                place
            }
            rustc_middle::mir::Operand::Constant(_)
            | rustc_middle::mir::Operand::RuntimeChecks(_) => continue,
        };
        if !is_mutable_guard_reference(cx, place.ty(body, cx.tcx).ty, guard_ty) {
            continue;
        }
        let Some(local) = place.as_local() else {
            continue;
        };
        let Some(targets) = state.references.get(&local) else {
            continue;
        };
        if !targets.is_empty() {
            has_tracked_guard = true;
        }
        // Keep retained container contents live after their move arguments are cleared.
        let new_targets = targets
            .iter()
            .filter(|target| !state.places.contains(*target))
            .cloned()
            .collect::<Vec<_>>();
        state.places.extend(new_targets);
    }
    // Result types can receive guards removed from their mutable container.
    // The caller checks this flag before tracking a return value.
    has_tracked_guard
}

/// Update a call's output when its moved inputs can be retained there.
fn update_call_destination<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    output: (Place<'tcx>, GuardPlace),
    has_guard_input: bool,
    carried: Vec<GuardPlace>,
    state: &mut GuardFlowState,
) {
    // The MIR output place and its projected identity refer to the same result.
    let (place, destination) = output;
    // Replace destination state after all moved sources have been removed.
    clear_guard_place(&mut state.places, &destination);
    clear_reference_targets(&mut state.references, &destination);
    let destination_ty = place.ty(body, cx.tcx).ty;
    if has_guard_input && type_contains_guard(cx, destination_ty, guard_ty) {
        if carried.is_empty() {
            // Opaque calls may retain a guard in any field of their return value.
            state.places.push(destination);
        } else {
            state.places.extend(carried);
        }
    }
}

/// Remove reference aliases when their backing local or field is overwritten.
fn clear_reference_targets(references: &mut HashMap<Local, Vec<GuardPlace>>, place: &GuardPlace) {
    let Some(local) = guard_place_local(place) else {
        return;
    };
    let _discarded_reference_targets = references.remove(&local);
}

/// Return whether a type is a mutable reference to a guard or guard-containing value.
fn is_mutable_guard_reference<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_middle::ty::Ty<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
) -> bool {
    matches!(
        ty.kind(),
        TyKind::Ref(_, pointee, rustc_hir::Mutability::Mut)
            if type_contains_guard(cx, *pointee, guard_ty)
    )
}

/// Return whether a wrapper or aggregate type can retain this guard.
fn type_contains_guard<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_middle::ty::Ty<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
) -> bool {
    type_contains_guard_with_seen(cx, ty, guard_ty, &mut HashSet::new())
}

/// Inspect generic arguments and instantiated fields without revisiting recursive ADTs.
fn type_contains_guard_with_seen<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_middle::ty::Ty<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    seen: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
) -> bool {
    // Resolve the exact guard first before inspecting containers.
    if ty == guard_ty {
        return true;
    }
    // The standard marker carries `T` only in type information, never as a value.
    let is_phantom_data = matches!(
        ty.kind(),
        TyKind::Adt(adt, _) if cx.tcx.lang_items().get(LangItem::PhantomData) == Some(adt.did())
    );
    if is_phantom_data {
        return false;
    }
    if !seen.insert(ty) {
        return false;
    }

    // Inspect the type's structural contents without revisiting active types.
    let has_nested_guard = type_structure_contains_guard(cx, ty, guard_ty, seen);
    // Remove this type after each branch so repeated independent fields stay inspectable.
    let _ = seen.remove(&ty);
    has_nested_guard
}

/// Inspect guard-bearing wrappers and aggregates without changing type identity.
fn type_structure_contains_guard<'tcx>(
    cx: &LateContext<'tcx>,
    ty: rustc_middle::ty::Ty<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    seen: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
) -> bool {
    // Visit instantiated generic arguments and fields under type-level cycle protection.
    match ty.kind() {
        TyKind::Adt(adt, arguments) => {
            adt_contains_guard_with_seen(cx, *adt, arguments, guard_ty, seen)
        }
        TyKind::Array(element, _)
        | TyKind::Slice(element)
        | TyKind::Ref(_, element, _)
        | TyKind::RawPtr(element, _) => {
            has_guard_in_child_types(cx, std::iter::once(*element), guard_ty, seen)
        }
        TyKind::Tuple(elements) => has_guard_in_child_types(cx, elements.iter(), guard_ty, seen),
        // Closure and coroutine arguments encode captured values in their upvar tuple.
        TyKind::Closure(_, arguments) => {
            has_guard_in_capture(cx, arguments, CaptureKind::Closure, guard_ty, seen)
        }
        TyKind::CoroutineClosure(_, arguments) => {
            has_guard_in_capture(cx, arguments, CaptureKind::CoroutineClosure, guard_ty, seen)
        }
        TyKind::Coroutine(_, arguments) => {
            has_guard_in_capture(cx, arguments, CaptureKind::Coroutine, guard_ty, seen)
        }
        TyKind::Bool
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
        | TyKind::CoroutineWitness(..)
        | TyKind::Never
        | TyKind::Alias(..)
        | TyKind::Param(_)
        | TyKind::Bound(..)
        | TyKind::Placeholder(_)
        | TyKind::Infer(_)
        | TyKind::Error(_) => false,
    }
}

/// Inspect each child type carried by a wrapper or aggregate.
fn has_guard_in_child_types<'tcx>(
    cx: &LateContext<'tcx>,
    mut children: impl Iterator<Item = rustc_middle::ty::Ty<'tcx>>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    seen: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
) -> bool {
    // Stop at the first child that can retain this guard.
    children.any(|child| type_contains_guard_with_seen(cx, child, guard_ty, seen))
}

/// Select the compiler representation of a callable's capture tuple.
#[derive(Clone, Copy)]
enum CaptureKind {
    /// A regular closure.
    Closure,
    /// A coroutine closure.
    CoroutineClosure,
    /// An async or generator coroutine.
    Coroutine,
}

/// Inspect the selected closure or coroutine capture tuple for a retained guard.
fn has_guard_in_capture<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: rustc_middle::ty::GenericArgsRef<'tcx>,
    kind: CaptureKind,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    seen: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
) -> bool {
    // Decode captures from the variant-specific compiler arguments.
    let captures = match kind {
        CaptureKind::Closure => arguments.as_closure().tupled_upvars_ty(),
        CaptureKind::CoroutineClosure => arguments.as_coroutine_closure().tupled_upvars_ty(),
        CaptureKind::Coroutine => arguments.as_coroutine().tupled_upvars_ty(),
    };
    type_contains_guard_with_seen(cx, captures, guard_ty, seen)
}

/// Inspect generic arguments and instantiated fields in an ADT.
fn adt_contains_guard_with_seen<'tcx>(
    cx: &LateContext<'tcx>,
    adt: rustc_middle::ty::AdtDef<'tcx>,
    arguments: rustc_middle::ty::GenericArgsRef<'tcx>,
    guard_ty: rustc_middle::ty::Ty<'tcx>,
    seen: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
) -> bool {
    // Inspect generic arguments before instantiated fields to avoid duplicate traversal.
    let has_guard_in_arguments = arguments
        .types()
        .any(|argument| type_contains_guard_with_seen(cx, argument, guard_ty, seen));
    // Instantiated fields expose guards inside nongeneric wrapper structs.
    let has_guard_in_fields = !has_guard_in_arguments
        && adt.all_fields().any(|field| {
            let field_ty = cx
                .tcx
                .type_of(field.did)
                .instantiate(cx.tcx, arguments)
                .skip_norm_wip();
            type_contains_guard_with_seen(cx, field_ty, guard_ty, seen)
        });
    has_guard_in_arguments || has_guard_in_fields
}

/// Recognize a return of `Poll::Pending` and read its coroutine state variant.
fn pending_variant<'tcx>(
    cx: &LateContext<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    async_body: LocalDefId,
    block: BasicBlock,
) -> Option<VariantIdx> {
    let data = body.basic_blocks.get(block)?;
    if !matches!(&data.terminator().kind, TerminatorKind::Return) {
        return None;
    }

    let span = data.terminator().source_info.span;
    let (poll_def_id, pending_variant) = poll_pending_variant(cx, span)?;
    // Confirm the returned Poll value is Pending before reading its coroutine discriminant.
    let returns_pending = data.statements.iter().any(|statement| {
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
    });
    if !returns_pending {
        return None;
    }

    // The coroutine discriminant identifies the initialized saved-field variant.
    data.statements.iter().rev().find_map(|statement| {
        let StatementKind::SetDiscriminant {
            place,
            variant_index,
        } = &statement.kind
        else {
            return None;
        };
        matches!(
            place.ty(body, cx.tcx).ty.kind(),
            TyKind::Coroutine(def_id, _)
                if *def_id == async_body.to_def_id()
        )
        .then_some(*variant_index)
    })
}

/// Resolve the standard `Poll::Pending` variant from rustc's language items.
fn poll_pending_variant(cx: &LateContext<'_>, span: Span) -> Option<(DefId, VariantIdx)> {
    let poll_def_id = cx.tcx.require_lang_item(LangItem::Poll, span);
    let pending_def_id = cx.tcx.require_lang_item(LangItem::PollPending, span);
    let pending_variant = cx
        .tcx
        .adt_def(poll_def_id)
        .variants()
        .iter_enumerated()
        .find_map(|(variant, definition)| {
            (definition.def_id == pending_def_id).then_some(variant)
        })?;
    Some((poll_def_id, pending_variant))
}

/// Join a predecessor state into a successor state and report whether it grew.
fn has_guard_state_changed(target: &mut Option<GuardFlowState>, incoming: &GuardFlowState) -> bool {
    // First input seeds the block without merging empty state.
    let Some(target) = target else {
        *target = Some(incoming.clone());
        return true;
    };

    // The place set joins values initialized on any incoming path.
    let mut state_has_grown = false;
    for place in &incoming.places {
        if !target.places.contains(place) {
            target.places.push(place.clone());
            state_has_grown = true;
        }
    }

    // Reference aliases use the same may-live join by backing local.
    for (local, places) in &incoming.references {
        let targets = target.references.entry(*local).or_default();
        for place in places {
            if !targets.contains(place) {
                targets.push(place.clone());
                state_has_grown = true;
            }
        }
    }

    // The worklist needs another visit only when either set grew.
    state_has_grown
}

/// Remove a place and its nested fields from the current may-initialized set.
fn clear_guard_place(state: &mut Vec<GuardPlace>, place: &GuardPlace) {
    state
        .retain(|current| current.base != place.base || !current.fields.starts_with(&place.fields));
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
