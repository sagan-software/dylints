//! Compiler UI cases for tracing guards retained across suspension points.
#![feature(rustc_private)]
#![allow(
    dead_code,
    reason = "UI functions are compiled to inspect diagnostics and are not called."
)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use tracing::{Instrument, info_span};
use tracing_await_holding_span_guard as _;

/// Provide a small await point for each UI case.
async fn other_work() {
    std::future::ready(()).await;
}

/// Keep a borrowed tracing guard across an await point.
async fn invalid_borrowed_guard() {
    let span = info_span!("borrowed");
    let _guard = span.enter();
    other_work().await;
}

/// Keep an owned tracing guard across an await point.
async fn invalid_owned_guard() {
    let _guard = info_span!("owned").entered();
    other_work().await;
}

/// Release synchronous span entry before awaiting.
async fn valid_scoped_work() {
    let span = info_span!("scoped");
    span.in_scope(|| {});
    other_work().await;
}

/// Instrument the future instead of holding an entered guard.
async fn valid_instrumented_work() {
    other_work().instrument(info_span!("instrumented")).await;
}

/// Keep an `Option` containing a guard across an await point.
async fn invalid_option_guard(span: tracing::Span) {
    let _guard = Some(span.entered());
    other_work().await;
}

/// Consume a moved guard through an opaque helper returning `None`.
#[inline(never)]
fn discard_guard(guard: Option<tracing::span::EnteredSpan>) -> Option<tracing::span::EnteredSpan> {
    drop(guard);
    None
}

/// Take and drop a guard through a mutable reference.
#[inline(never)]
fn discard_guard_through_mut_ref(guard: &mut Option<tracing::span::EnteredSpan>) {
    drop(guard.take());
}

/// Read a borrowed guard without changing its owner.
#[inline(never)]
const fn inspect_guard_read_only(_guard: Option<&tracing::span::EnteredSpan>) {}

/// Leave mutable helper effects unknown to the lint.
#[inline(never)]
const fn opaque_no_op_mut_ref(_guard: &mut Option<tracing::span::EnteredSpan>) {}

/// Consume a boolean through an indirect-call control.
#[inline(never)]
const fn ignore_flag(_flag: bool) {}

/// Avoid warning after an opaque helper consumes the guard and returns `None`.
async fn valid_opaque_helper_discards_guard(span: tracing::Span) {
    let guard = discard_guard(Some(span.entered()));
    other_work().await;
    drop(guard);
}

/// Avoid warning after a mutable helper takes and drops the guard.
async fn valid_opaque_mut_ref_helper_discards_guard(span: tracing::Span) {
    let mut guard = Some(span.entered());
    discard_guard_through_mut_ref(&mut guard);
    other_work().await;
}

/// Show that an opaque mutable helper can hide a retained guard.
async fn unknown_opaque_mut_ref_state(span: tracing::Span) {
    let mut guard = Some(span.entered());
    opaque_no_op_mut_ref(&mut guard);
    other_work().await;
    drop(guard);
}

/// Warn while unresolved helper calls borrow a separate external option.
async fn invalid_guard_with_unresolved_mutable_borrow(
    span: tracing::Span,
    external: &mut Option<tracing::span::EnteredSpan>,
) {
    let guard = Some(span.entered());
    let indirect_ignore_flag: fn(bool) = ignore_flag;
    indirect_ignore_flag(true);
    let indirect_mutator: fn(&mut Option<tracing::span::EnteredSpan>) = opaque_no_op_mut_ref;
    indirect_mutator(external);
    drop(external.take());
    other_work().await;
    drop(guard);
}

/// Warn after a shared read because the guard remains in its wrapper.
async fn invalid_guard_after_shared_borrow(span: tracing::Span) {
    let guard = Some(span.entered());
    inspect_guard_read_only(guard.as_ref());
    other_work().await;
    drop(guard);
}

/// Warn after dropping a mutable reference without dropping its guard.
#[expect(
    dropping_references,
    reason = "This control drops only the mutable reference, not its guard."
)]
async fn invalid_guard_after_dropping_mut_ref(span: tracing::Span) {
    let mut guard = Some(span.entered());
    drop(&mut guard);
    other_work().await;
    drop(guard);
}

/// Warn when an aliased take can leave the guard in its original wrapper.
async fn invalid_maybe_aliased_take_retains_guard_on_one_branch(
    span: tracing::Span,
    take_guard: bool,
) {
    let mut guard = Some(span.entered());
    let mut empty = None;
    let alias = if take_guard { &mut guard } else { &mut empty };
    drop(alias.take());
    other_work().await;
    drop(guard);
    drop(empty);
}

/// Stay quiet when the possible source wrapper is dropped before suspension.
async fn valid_maybe_aliased_take_released_before_await(span: tracing::Span, take_guard: bool) {
    let mut guard = Some(span.entered());
    let mut empty = None;
    let alias = if take_guard { &mut guard } else { &mut empty };
    drop(alias.take());
    drop(guard);
    other_work().await;
    drop(empty);
}

/// Stay quiet when the option contains no guard.
async fn valid_none_guard() {
    let _guard: Option<tracing::span::EnteredSpan> = None;
    other_work().await;
}

/// Stay quiet when an owned wrapper is dropped before suspension.
async fn valid_dropped_wrapper(span: tracing::Span) {
    let guard = Some(span.entered());
    drop(guard);
    other_work().await;
}

/// Stay quiet when the direct guard is dropped before suspension.
async fn valid_explicitly_dropped_guard(span: tracing::Span) {
    let guard = span.entered();
    drop(guard);
    other_work().await;
}

/// Stay quiet when `Option::take` removes and drops the guard.
async fn valid_taken_and_dropped_wrapper(span: tracing::Span) {
    let mut guard = Some(span.entered());
    drop(guard.take());
    other_work().await;
}

/// Stay quiet when taking through a direct mutable-reference alias.
async fn valid_taken_through_reference_alias(span: tracing::Span) {
    let mut guard = Some(span.entered());
    let alias = &mut guard;
    drop(alias.take());
    other_work().await;
    other_work().await;
}

/// Stay quiet when taking through a moved mutable-reference alias.
async fn valid_taken_through_moved_reference_alias(span: tracing::Span) {
    let mut guard = Some(span.entered());
    let alias = &mut guard;
    let moved_alias = alias;
    drop(moved_alias.take());
    other_work().await;
    other_work().await;
}

/// Warn at the first await before the wrapper is dropped.
async fn invalid_wrapper_dropped_between_awaits(span: tracing::Span) {
    let guard = Some(span.entered());
    other_work().await;
    drop(guard);
    other_work().await;
}

/// Stay quiet when the wrapper is taken and dropped between awaits.
async fn valid_wrapper_taken_between_awaits(span: tracing::Span) {
    other_work().await;
    let mut guard = Some(span.entered());
    drop(guard.take());
    other_work().await;
}

/// Stay quiet when lexical scope releases the wrapper before suspension.
async fn valid_lexically_released_wrapper(span: tracing::Span) {
    {
        let _guard = Some(span.entered());
    }
    other_work().await;
}

/// Stay quiet when either branch drops the wrapper.
async fn valid_wrapper_released_on_both_branches(span: tracing::Span, first_branch: bool) {
    let guard = Some(span.entered());
    if first_branch {
        drop(guard);
    } else {
        drop(Some(guard));
    }
    other_work().await;
}

/// Warn when one branch can retain the wrapper through suspension.
async fn invalid_wrapper_retained_on_one_branch(span: tracing::Span, release: bool) {
    let guard = Some(span.entered());
    if release {
        drop(guard);
    }
    other_work().await;
}

/// Warn when a directly constructed `Box` retains the guard.
async fn invalid_boxed_guard(span: tracing::Span) {
    let guard = Box::new(span.entered());
    other_work().await;
    drop(guard);
}

/// Store an optional guard beside an unrelated marker field.
struct GuardWrapper {
    /// Marker that does not contain a guard.
    marker: bool,
    /// Count field that does not contain a guard.
    count: u32,
    /// Callback field that does not contain a guard.
    callback: fn(),
    /// Optional guard retained by this wrapper.
    guard: Option<tracing::span::EnteredSpan>,
}

/// Keep the complete wrapper value live through its destructor.
impl Drop for GuardWrapper {
    /// Read wrapper metadata while the guard is dropped.
    fn drop(&mut self) {
        let _marker = std::hint::black_box(self.marker);
        let _count = std::hint::black_box(self.count);
        let _callback = std::hint::black_box(self.callback);
    }
}

/// Store a recursive wrapper before its tracing guard.
struct RecursiveGuard {
    /// Optional recursive ownership link.
    next: Option<Box<Self>>,
    /// Optional tracing guard retained by this wrapper.
    held: Option<tracing::span::EnteredSpan>,
}

/// Warn when a struct field owns the guard across suspension.
async fn invalid_struct_guard(span: tracing::Span) {
    let guard = GuardWrapper {
        marker: false,
        count: 0,
        callback: wrapper_callback,
        guard: Some(span.entered()),
    };
    other_work().await;
    drop(guard);
}

/// Warn when a recursive wrapper retains the guard through suspension.
async fn invalid_recursive_struct_guard(span: tracing::Span) {
    let guard = RecursiveGuard {
        next: None,
        held: Some(span.entered()),
    };
    other_work().await;
    drop(guard);
}

/// Provide the wrapper callback field's function pointer value.
const fn wrapper_callback() {}

/// Leave array wrappers unsupported and quiet.
async fn unsupported_array_guard_wrapper(span: tracing::Span) {
    let guard = [span.entered()];
    other_work().await;
    drop(guard);
}

/// Wrap a guard in exactly sixteen generic tuple structs.
macro_rules! wrap_guard_16 {
    ($guard:expr) => {
        Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(
            Nest(Nest(Nest(Nest(Nest($guard))))),
        )))))))))))
    };
}

/// Wrap a guard in exactly seventeen generic tuple structs.
macro_rules! wrap_guard_17 {
    ($guard:expr) => {
        Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(Nest(
            Nest(Nest(Nest(Nest(Nest(Nest($guard)))))),
        )))))))))))
    };
}

/// Store one value in a generic tuple field.
struct Nest<T>(T);

/// Nest resolved `Box::new` calls without adding MIR aggregate fields.
macro_rules! wrap_box_depth {
    ($guard:expr; $($layers:tt)*) => {
        wrap_box_depth!(@wrap $guard, $($layers)*)
    };
    (@wrap $guard:expr,) => {
        $guard
    };
    (@wrap $guard:expr, @ $($layers:tt)*) => {
        Box::new(wrap_box_depth!(@wrap $guard, $($layers)*))
    };
}

/// Warn when sixteen nested struct fields retain the guard through suspension.
async fn invalid_guard_at_field_depth_limit(span: tracing::Span) {
    let guard = wrap_guard_16!(span.entered());
    other_work().await;
    drop(guard);
}

/// Leave wrappers deeper than the field limit unsupported and quiet.
async fn unsupported_guard_over_field_depth_limit(span: tracing::Span) {
    let guard = wrap_guard_17!(span.entered());
    other_work().await;
    drop(guard);
}

/// Warn when sixteen recursive wrapper types retain the guard through suspension.
async fn invalid_guard_at_type_depth_limit(span: tracing::Span) {
    let guard = Nest(wrap_box_depth!(span.entered(); @ @ @ @ @ @ @ @ @ @ @ @ @ @ @));
    other_work().await;
    drop(guard);
}

/// Leave wrapper types deeper than the type limit unsupported and quiet.
async fn unsupported_guard_over_type_depth_limit(span: tracing::Span) {
    let guard = Nest(wrap_box_depth!(span.entered(); @ @ @ @ @ @ @ @ @ @ @ @ @ @ @ @));
    other_work().await;
    drop(guard);
}

/// Warn when a tuple field owns the guard across suspension.
async fn invalid_tuple_guard(span: tracing::Span) {
    let guard = (span.entered(),);
    other_work().await;
    drop(guard);
}

/// Leave a reborrow deeper than the field limit unsupported and quiet.
async fn unsupported_reborrow_over_field_depth_limit(span: tracing::Span) {
    let guard = span.entered();
    drop(guard);
    let mut wrapper = wrap_guard_17!(None::<tracing::span::EnteredSpan>);
    let alias = &mut wrapper;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    let alias = &mut alias.0;
    drop(alias.take());
    other_work().await;
}

/// Warn when `Box<Option<_>>` retains the guard across suspension.
async fn regression_boxed_option_guard(span: tracing::Span) {
    let guard = Box::new(Some(span.entered()));
    other_work().await;
    drop(guard);
}

/// Warn when a boxed user-defined wrapper retains the guard across suspension.
async fn regression_boxed_user_struct_guard(span: tracing::Span) {
    let guard = Box::new(GuardWrapper {
        marker: false,
        count: 0,
        callback: wrapper_callback,
        guard: Some(span.entered()),
    });
    other_work().await;
    drop(guard);
}

/// Stay quiet when a boxed option is dropped before suspension.
async fn valid_boxed_option_guard_dropped_before_await(span: tracing::Span) {
    let guard = Box::new(Some(span.entered()));
    drop(guard);
    other_work().await;
}

/// Stay quiet when a boxed user-defined wrapper is dropped before suspension.
async fn valid_boxed_user_struct_guard_dropped_before_await(span: tracing::Span) {
    let guard = Box::new(GuardWrapper {
        marker: false,
        count: 0,
        callback: wrapper_callback,
        guard: Some(span.entered()),
    });
    drop(guard);
    other_work().await;
}

/// Satisfy the compiler UI fixture entry-point requirement.
fn main() {}
