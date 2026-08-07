> [!IMPORTANT]
> We have enacted a feature freeze for IDE assists to cope with the PR backlog as well as allowing us to prepare for the rowan transition!
> If you submit a PR that **adds** new ide-assists, chances are very high that we will just close it on this basis alone until we have the capacity to deal with them again.


# Contributing to verus-analyzer

Thank you for your interest in contributing to verus-analyzer! There are many ways to contribute
and we appreciate all of them.

To get a quick overview of the crates and structure of the project take a look at the
[Contributing](https://rust-analyzer.github.io/book/contributing) section of the manual.

Questions about the Verus-specific parts of the fork belong in the
[Verus Zulip](https://verus-lang.zulipchat.com/). Upstream rust-analyzer design questions belong in
the [rust-analyzer Zulip stream](
https://rust-lang.zulipchat.com/#narrow/stream/185405-t-compiler.2Frust-analyzer).

## Fixing a bug or improving a feature

Generally it's fine to just work on these kinds of things and put a pull-request out for it. If there
is an issue accompanying it make sure to link it in the pull request description so it can be closed
afterwards or linked for context.

If you want to find something to fix or work on keep a look out for the `C-bug` and `C-enhancement`
labels.

## Implementing a new feature

It's advised to first open an issue for any kind of new feature so the team can tell upfront whether
the feature is desirable or not before any implementation work happens. We want to minimize the
possibility of someone putting a lot of work into a feature that is then going to waste as we deem
it out of scope (be it due to generally not fitting in with rust-analyzer, or just not having the
maintenance capacity). If there already is a feature issue open but it is not clear whether it is
considered accepted feel free to just drop a comment and ask!

## Use of AI

All use of AI in contributions must follow the [AI Policy](./AI_POLICY.md).

Contributions not following the AI Policy will be closed.

## Verus-specific architecture

Verus support is intentionally layered on top of current rust-analyzer:

- Verus grammar and parser changes live in `crates/parser` and `crates/syntax`.
- HIR lowering and analysis support live in `crates/hir-def`, `crates/hir-ty`, and `crates/hir`.
- Verus syntax tree generation and proof actions live in `xtask/src/codegen/grammar`,
  `crates/syntax/src/ast/vst.rs`, and `crates/ide-assists/src/handlers/proof_action`.
- Verifier execution and diagnostics live in `crates/rust-analyzer/src/flycheck.rs` and
  `crates/rust-analyzer/src/verus_interaction.rs`.
- Verus installation and editor integration live in `editors/code`.

The server runs Verus when a Rust file is saved. `verus.enable` controls this behavior,
`verus.extraArgs` adds verifier arguments, `verus.reportAllErrorsEnable` disables per-module
filtering, and `cargo.verusEnable` selects `cargo-verus` instead of direct invocation. The VS Code
extension sets `VERUS_BINARY_PATH` for the server process.

## Verus test matrix

Run focused tests while developing:

```bash
cargo codegen --check
CARGO_BUILD_JOBS=1 cargo test -p syntax --lib
CARGO_BUILD_JOBS=1 cargo test -p hir-def -p hir-ty --lib
CARGO_BUILD_JOBS=1 cargo test -p ide-assists --lib handlers::proof_action
CARGO_BUILD_JOBS=1 cargo test -p rust-analyzer --lib flycheck::tests
```

Before merging an upstream sync or broad Verus change, run:

```bash
CARGO_BUILD_JOBS=1 cargo nextest run --no-fail-fast
```

If the VS Code dependencies are installed, also run `npm run typecheck` and `npm run lint` from
`editors/code`. Tests that execute a locally installed verifier may require `VERUS_BINARY_PATH`.

## Changing Verus syntax

1. Add parser coverage for the new syntax.
2. Update `crates/syntax/rust.ungram` and token definitions in
   `xtask/src/codegen/grammar/ast_src.rs`.
3. Update parser grammar, primarily `crates/parser/src/grammar/verus.rs`.
4. Run `cargo codegen` and commit the generated CST and VST files.
5. Update HIR lowering, inference, traversal, and pretty-printing as required.
6. Run the focused test matrix above.

Verus's `dependencies/syn`, `builtin_macros/src/syntax.rs`, examples, and verusfmt grammar are useful
references when the accepted syntax is unclear.

## Updating configuration

Server configuration is defined in `crates/rust-analyzer/src/config.rs`. Regenerate checked-in
editor settings and documentation with:

```bash
cargo test -p rust-analyzer --lib generate_package_json_config
cargo test -p rust-analyzer --lib generate_config_documentation
```
