#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks for wildcard lets that discard lazy Bevy message iterators.
//!
//! `MessageReader::read()` and `read_with_id()` return iterators. The reader
//! advances its cursor only as the caller consumes items, so dropping an
//! unread iterator leaves messages available for a later read.
//!
//! This lint uses resolved Bevy method identities and reports direct calls in
//! wildcard `let` initializers. It ignores named bindings and adapter chains.
//! Consume the iterator when processing its messages. Call `clear()` only
//! when discarding unread payloads is intentional.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use dylint_linting as _;
use rustc_hir::{Expr, ExprKind, PatKind, Stmt, StmtKind};
use rustc_lint::LintContext as _;
use rustc_span::Span;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_MESSAGE_PRESENCE_WITHOUT_CONSUMPTION,
    Warn,
    "checks for discarded unread Bevy message iterators",
    BevyMessagePresenceWithoutConsumption
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyMessagePresenceWithoutConsumption {
    /// Report a wildcard binding that drops a Bevy message iterator before consuming it.
    fn check_stmt(&mut self, cx: &rustc_lint::LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        let StmtKind::Let(local) = stmt.kind else {
            return;
        };
        // Named locals can consume the iterator later, so only wildcard lets qualify.
        if !matches!(local.pat.kind, PatKind::Wild) {
            return;
        }
        // Without an initializer, the wildcard has no iterator to discard.
        let Some(initializer) = local.init else {
            return;
        };
        let Some(read_span) = discarded_message_reader_iterator(cx, initializer) else {
            return;
        };

        cx.emit_span_lint(
            BEVY_MESSAGE_PRESENCE_WITHOUT_CONSUMPTION,
            read_span,
            rustc_errors::DiagDecorator(|diagnostic| {
                let _message = diagnostic.primary_message(
                    "creating this lazy message iterator does not consume unread messages",
                );
                let _help = diagnostic.help(
                    "consume the iterator, or call `clear()` only when discarding unread messages is intentional",
                );
            }),
        );
    }
}

/// Resolve a direct Bevy reader call after removing compiler-inserted wrappers.
fn discarded_message_reader_iterator<'hir>(
    cx: &rustc_lint::LateContext<'_>,
    mut expr: &'hir Expr<'hir>,
) -> Option<Span> {
    // HIR can wrap a source expression in `DropTemps`; this wrapper does not change its type.
    while let ExprKind::DropTemps(inner) = expr.kind {
        expr = inner;
    }

    ["read", "read_with_id"].into_iter().find_map(|method| {
        bevy_support::bevy_method_call(cx, expr, "bevy_ecs", "MessageReader", method)
            .map(|call| call.method_span)
    })
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    use bevy::ecs::{
        message::{Message, MessageReader, Messages},
        system::SystemState,
        world::World,
    };
    use bevy::reflect::Reflect;

    #[derive(Message, Reflect)]
    struct Ping;

    fn world_with_unread_ping() -> (World, SystemState<MessageReader<'static, 'static, Ping>>) {
        let mut world = World::new();
        // Readers need a message resource before a message can be written.
        let _messages_component = world.init_resource::<Messages<Ping>>();
        let _message_id = world.write_message(Ping);
        // Keep the system state so each test can borrow the reader again.
        let state = SystemState::new(&mut world);
        (world, state)
    }

    #[test]
    fn dropping_read_iterator_preserves_unread_message() {
        let (mut world, mut state) = world_with_unread_ping();
        // Leave the iterator unpolled so dropping it cannot advance the cursor.
        {
            let mut reader = state.get_mut(&mut world).expect("message resource exists");
            let _iterator = reader.read();
        }
        // Fetch the reader again to observe its cursor after the drop.
        let reader = state.get_mut(&mut world).expect("message resource exists");
        assert!(!reader.is_empty());
    }

    #[test]
    fn consuming_read_iterator_advances_reader_cursor() {
        let (mut world, mut state) = world_with_unread_ping();
        let mut reader = state.get_mut(&mut world).expect("message resource exists");
        // Count all items to poll the iterator through the pending message.
        let consumed = reader.read().count();
        assert_eq!(consumed, 1);
        assert!(reader.is_empty());
    }

    #[test]
    fn clearing_messages_advances_reader_cursor() {
        let (mut world, mut state) = world_with_unread_ping();
        let mut reader = state.get_mut(&mut world).expect("message resource exists");
        // Confirm unread data exists before deliberately discarding it.
        assert!(!reader.is_empty());
        reader.clear();
        assert!(reader.is_empty());
    }

    #[test]
    fn dropping_read_with_id_iterator_preserves_unread_message() {
        let (mut world, mut state) = world_with_unread_ping();
        // Dropping the identified iterator without polling must keep the message unread.
        {
            let mut reader = state.get_mut(&mut world).expect("message resource exists");
            let _iterator = reader.read_with_id();
        }
        let mut reader = state.get_mut(&mut world).expect("message resource exists");
        assert!(!reader.is_empty());
        // Poll the identified item to advance the same cursor.
        assert_eq!(reader.read_with_id().count(), 1);
        assert!(reader.is_empty());
    }
}
