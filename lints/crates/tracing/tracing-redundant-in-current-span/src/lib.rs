#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to identify tracing future chains that may use one wrapper when
//! the inner span is disabled.
//!
//! Its diagnostic describes enter/exit event changes for enabled inner spans.
//! The README and UI fixtures define the supported boundary, conditional
//! advice, and non-triggering cases.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use tracing_support::tracing_method_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_REDUNDANT_IN_CURRENT_SPAN,
    Warn,
    "nested tracing instrumentation may be redundant when the inner span is disabled",
    TracingRedundantInCurrentSpan
}

impl<'tcx> LateLintPass<'tcx> for TracingRedundantInCurrentSpan {
    /// Check the exact method nesting documented by tracing.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the outer span attachment before comparing its instrumented receiver.
        let Some(outer) =
            tracing_method_call(cx, expr, "tracing::instrument::Instrument::in_current_span")
        else {
            return;
        };
        if tracing_method_call(
            cx,
            outer.receiver,
            "tracing::instrument::Instrument::instrument",
        )
        .is_none()
        {
            return;
        }

        cx.emit_span_lint(
            TRACING_REDUNDANT_IN_CURRENT_SPAN,
            outer.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message(
                        "this nested tracing instrumentation may be redundant when the inner span is disabled",
                    )
                    .help(
                        "when the inner span is disabled, pass `span.or_current()` to `instrument` and remove `in_current_span`",
                    )
                    .note(
                        "for an enabled independent inner span, removing `in_current_span()` can remove enter/exit events for the captured current span on each poll and drop",
                    );
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod span_events {
    //! Verify poll and drop callbacks with an enabled or filtered inner span.

    use std::{
        sync::{Arc, Mutex},
        task::{Context, Waker},
    };

    use tracing::{Instrument, Subscriber, debug_span, info_span};
    use tracing_subscriber::{
        filter::LevelFilter,
        layer::{Context as LayerContext, Layer},
        prelude::*,
        registry::LookupSpan,
    };

    /// The ordered span transitions recorded by one subscriber.
    type Events = Arc<Mutex<Vec<(&'static str, &'static str)>>>;

    /// Record span enter and exit callbacks for spans accepted by its filter.
    struct Recorder {
        /// Holds the ordered transitions observed by this layer.
        events: Events,
    }

    impl<S> Layer<S> for Recorder
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        /// Record the span name when the subscriber enters the span.
        fn on_enter(&self, id: &tracing::span::Id, context: LayerContext<'_, S>) {
            let name = context.span(id).expect("entered span is registered").name();
            self.events
                .lock()
                .expect("event recorder mutex is not poisoned")
                .push(("enter", name));
        }

        /// Record the span name when the subscriber exits the span.
        fn on_exit(&self, id: &tracing::span::Id, context: LayerContext<'_, S>) {
            let name = context.span(id).expect("exited span is registered").name();
            self.events
                .lock()
                .expect("event recorder mutex is not poisoned")
                .push(("exit", name));
        }
    }

    /// Select the original nested wrappers or the `or_current` rewrite.
    #[derive(Clone, Copy)]
    enum Instrumentation {
        /// Enter the inner span inside the captured current span.
        Nested,
        /// Use one wrapper around the inner span or current-span fallback.
        OrCurrent,
    }

    /// Poll a pending future once, then drop it while its current span remains entered.
    fn poll_once_then_drop(future: impl Future<Output = ()>) {
        let mut future = Box::pin(future);
        let mut context = Context::from_waker(Waker::noop());
        let _poll = future.as_mut().poll(&mut context);
        drop(future);
    }

    /// Return span callbacks for an independent inner span and the chosen wrappers.
    fn recorded_events(
        instrumentation: Instrumentation,
        is_inner_disabled: bool,
    ) -> Vec<(&'static str, &'static str)> {
        let events = Arc::new(Mutex::new(Vec::new()));
        // The subscriber-wide level filter disables only the debug-level inner span.
        let max_level = if is_inner_disabled {
            LevelFilter::INFO
        } else {
            LevelFilter::DEBUG
        };
        let subscriber = tracing_subscriber::registry()
            .with(Recorder {
                events: events.clone(),
            })
            .with(max_level);

        tracing::subscriber::with_default(subscriber, || {
            let outer = info_span!("outer");
            let inner = debug_span!(parent: None, "inner");
            assert!(!outer.is_disabled());
            assert_eq!(inner.is_disabled(), is_inner_disabled);
            let _outer_entered = outer.enter();
            events
                .lock()
                .expect("event recorder mutex is not poisoned")
                .clear();

            // The inner span has no parent, so only the outer wrapper enters `outer`.
            match instrumentation {
                Instrumentation::Nested => {
                    #[cfg_attr(
                        dylint_lib = "crates",
                        expect(
                            tracing_redundant_in_current_span,
                            reason = "The test compares callbacks from the original nested wrappers."
                        )
                    )]
                    poll_once_then_drop(std::future::pending().instrument(inner).in_current_span());
                }
                Instrumentation::OrCurrent => {
                    poll_once_then_drop(std::future::pending().instrument(inner.or_current()));
                }
            }

            events
                .lock()
                .expect("event recorder mutex is not poisoned")
                .clone()
        })
    }

    /// Show that an enabled independent inner span loses outer callbacks in the rewrite.
    #[test]
    fn enabled_independent_inner_span_changes_poll_and_drop_callbacks() {
        assert_eq!(
            recorded_events(Instrumentation::Nested, false),
            vec![
                ("enter", "outer"),
                ("enter", "inner"),
                ("exit", "inner"),
                ("exit", "outer"),
                ("enter", "outer"),
                ("enter", "inner"),
                ("exit", "inner"),
                ("exit", "outer"),
            ]
        );
        assert_eq!(
            recorded_events(Instrumentation::OrCurrent, false),
            vec![
                ("enter", "inner"),
                ("exit", "inner"),
                ("enter", "inner"),
                ("exit", "inner"),
            ]
        );
    }

    /// Show that the disabled inner span falls back to the captured current span.
    #[test]
    fn disabled_inner_span_keeps_current_span_callbacks() {
        let expected = vec![
            ("enter", "outer"),
            ("exit", "outer"),
            ("enter", "outer"),
            ("exit", "outer"),
        ];
        assert_eq!(recorded_events(Instrumentation::Nested, true), expected);
        assert_eq!(recorded_events(Instrumentation::OrCurrent, true), expected);
    }
}
