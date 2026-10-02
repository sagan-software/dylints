# Rust module-layout Clippy lints

Research date: 2026-08-12.

Yes. With the normal declarations `lib.rs: mod foo;` and `foo.rs: mod child;`, a layout containing `src/foo.rs` and `src/foo/child.rs` triggers `clippy::self_named_module_files` on `src/foo.rs`. Clippy records that `foo.rs` contains an external child, then checks the parent file; the lint requires the file to be `mod.rs` and suggests moving it to `src/foo/mod.rs`. The child file itself is not necessarily reported unless it also contains an external child. This source path is excluded when `#[path]` changes the module location. [Clippy implementation](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/module_style.rs#L184-L235)

The layout is valid Rust. The Reference documents both forms: a module can use `foo.rs` with children under `foo/`, or use `foo/mod.rs`; it forbids having both `foo.rs` and `foo/mod.rs` for the same module. [Rust Reference: module source filenames](https://doc.rust-lang.org/reference/items/modules.html#module-source-filenames)

## Lint distinction

- `clippy::self_named_module_files` is a restriction lint, allowed by default. It selects the `mod.rs` convention and was added in Rust 1.57.0. Its current implementation checks non-`mod.rs` files when they contain external child modules. [Stable lint entry](https://rust-lang.github.io/rust-clippy/stable/index.html#self_named_module_files) · [source](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/module_style.rs#L73-L100)
- `clippy::mod_module_files` is the inverse restriction for ordinary source modules: it selects self-named files such as `foo.rs` and bans `mod.rs` (with an integration-test exception in the implementation). It was also added in Rust 1.57.0. Therefore `src/foo.rs` plus `src/foo/child.rs` is the layout this lint permits. [Stable lint entry](https://rust-lang.github.io/rust-clippy/stable/index.html#mod_module_files) · [source](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/module_style.rs#L44-L71)
- `clippy::module_inception` is a style lint, not a filesystem-layout lint. It checks for a module with the same name as its parent, such as `mod foo;` in `lib.rs` followed by `mod foo { ... }` inside `foo.rs`; that creates the surprising path `foo::foo::...`. A `foo.rs` file with an external `child` module does not trigger it because `foo/child.rs` exists. It has been available since before Rust 1.29.0. [Stable lint entry](https://rust-lang.github.io/rust-clippy/stable/index.html#module_inception) · [source](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/item_name_repetitions.rs#L48-L76)

The current official stable Clippy index lists all three lints. Both layout lints are `restriction`/`allow`; `module_inception` is `style`/`warn`. [Stable Clippy lint index](https://rust-lang.github.io/rust-clippy/stable/index.html)
