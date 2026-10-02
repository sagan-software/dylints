# maintainability

Quantitative Rust source metrics intended as review gates for generated code.

## Lints

- [`cyclomatic_complexity`](cyclomatic_complexity): bounds independent control-flow decisions per callable.
- [`source_cognitive_complexity`](source_cognitive_complexity): bounds nested source control-flow burden per callable.
- [`npath_complexity`](npath_complexity): bounds estimated acyclic execution routes per callable.
- [`type_method_complexity`](type_method_complexity): bounds summed method complexity per implementation block.
- [`module_fan_out`](module_fan_out): bounds distinct top-level local module dependencies.
- [`module_dependency_cycle`](module_dependency_cycle): detects cycles between top-level local modules.
- [`abc_size`](abc_size): bounds assignment, call, and condition magnitude per callable.
- [`many_exit_points`](many_exit_points): bounds explicit and `?` exit paths per callable.
- [`public_surface_size`](public_surface_size): bounds reachable public names per module.
- [`impl_method_count`](impl_method_count): bounds inherent methods per local type.
- [`field_usage_cohesion`](field_usage_cohesion): detects substantial disconnected method-field clusters in local types.
