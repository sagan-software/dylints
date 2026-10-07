//! Declarative lint-pass definitions shared by Bevy lint crates.

/// Declare one panicking `World` method lint.
#[macro_export]
macro_rules! declare_world_method_lint {
    (
        $lint:ident, $pass:ident, $level:ident, $method:literal, $alternative:literal,
        $description:literal, $message:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            $level,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check an exact panicking method on `bevy_ecs::world::World`.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(call) = $crate::world_method_call(cx, expr, $method) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.method_span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help(concat!(
                                "use `World::",
                                $alternative,
                                "` and handle the fallible result"
                            ));
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose semantic matcher returns one expression span.
#[macro_export]
macro_rules! declare_expression_span_lint {
    (
        $lint:ident, $pass:ident, $level:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            $level,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check one semantically resolved Bevy expression.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(span) = $checker(cx, expr) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint against mutable query access to one managed Bevy component.
#[macro_export]
macro_rules! declare_mutable_query_component_lint {
    (
        $lint:ident, $pass:ident, $component_crate:literal, $component:literal,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameters for mutable query access to the managed component.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                _: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $crate::mutable_query_component_parameters(
                    cx,
                    kind,
                    local_def_id,
                    $component_crate,
                    $component,
                );
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose matcher returns function-parameter indexes.
#[macro_export]
macro_rules! declare_function_parameter_lint {
    (
        $lint:ident, $pass:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameters through the configured semantic matcher.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                body: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $checker(cx, kind, body, local_def_id);
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare a lint whose type-only matcher returns function-parameter indexes.
#[macro_export]
macro_rules! declare_function_parameter_type_lint {
    (
        $lint:ident, $pass:ident, $checker:path, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check function parameter types through the configured semantic matcher.
            fn check_fn(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                kind: rustc_hir::intravisit::FnKind<'tcx>,
                declaration: &'tcx rustc_hir::FnDecl<'tcx>,
                _: &'tcx rustc_hir::Body<'tcx>,
                _: rustc_span::Span,
                local_def_id: rustc_span::def_id::LocalDefId,
            ) {
                use rustc_lint::LintContext as _;

                let indexes = $checker(cx, kind, local_def_id);
                for span in $crate::parameter_spans(declaration, indexes) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic =
                                diagnostic.primary_message($message).help($help);
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare one project schedule policy lint.
#[macro_export]
macro_rules! declare_disallowed_schedule_lint {
    (
        $lint:ident, $pass:ident, $schedule:literal, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check `App::add_systems` for the configured schedule label.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                use rustc_lint::LintContext as _;

                let Some(span) = $crate::disallowed_schedule_span(cx, expr, $schedule) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Declare one trait convention lint for unit components.
#[macro_export]
macro_rules! declare_missing_unit_component_trait_lint {
    (
        $lint:ident, $pass:ident, $marker_trait:ident, $description:literal,
        $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check local unit components for the configured standard trait.
            fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
                use rustc_lint::LintContext as _;

                let marker_trait = $crate::MarkerTrait::$marker_trait;
                for target in $crate::local_unit_components_missing_trait(cx, marker_trait) {
                    let derive = $crate::missing_trait_derive(cx, target, marker_trait);
                    cx.emit_span_lint(
                        $lint,
                        cx.tcx.def_span(target),
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic = diagnostic.primary_message($message);
                            // Offer the derive only where it is known to compile.
                            if let Some((span, attribute)) = derive {
                                let _suggested = diagnostic.span_suggestion_verbose(
                                    span,
                                    $help,
                                    attribute,
                                    rustc_errors::Applicability::MachineApplicable,
                                );
                            } else {
                                let _helped = diagnostic.help($help);
                            }
                        }),
                    );
                }
            }
        }

        /// Run the positive and negative UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}
