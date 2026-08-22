# 2026 rust-analyzer sync summary

## Status

The sync was completed on branch `sync/rust-analyzer-2026-08-06`.

| Role | Revision | Date | Notes |
| --- | --- | --- | --- |
| Previous verus-analyzer tip | `652a84ecdc` | 2026-08-06 | Preserved as `archive/pre-2026-upstream-sync` |
| Previous rust-analyzer base | `7b11fdeb68` | 2024-06-24 | The common ancestor of the old fork and upstream |
| New rust-analyzer base | `ece721d6cd` | 2026-08-06 | `upstream/master` when the sync was finalized |
| Completed sync | `4f56fd1ade` | 2026-08-07 | New upstream plus the ported Verus layers |

Although development continued in the fork through 2026, its rust-analyzer foundation was still
from June 2024. The sync therefore incorporated a little over two years of upstream changes. A
direct diff from the old fork to the result touches more than 2,000 files, mostly because of
upstream development. The Verus-specific delta on top of the new upstream base is much smaller:
134 files, with approximately 28,000 insertions. Most of those insertions are the generated Verus
syntax tree.

The work was split into reviewable checkpoints:

1. `970eba0fb1` ports the Verus grammar and parser.
2. `59284f2925` lowers Verus syntax into current HIR.
3. `bfd4bf4ec0` ports the VST generator and proof actions.
4. `9351073392` integrates verifier execution and editor support.
5. `5579082e20` completes CI, release, documentation, folding, and branding work.
6. `4f56fd1ade` closes compatibility gaps found by the old Verus test corpus.

## Sync strategy

The result was built from the selected upstream revision rather than by merging upstream into the
old fork and resolving files mechanically. Verus behavior was then ported in dependency order.
This made current rust-analyzer structure and APIs the default, prevented old copies of upstream
files from replacing newer implementations, and made it possible to test each layer before adding
the next one.

The practical rule was to preserve Verus behavior, not the old implementation. Where upstream had
introduced a better abstraction, the Verus feature was adapted to that abstraction. This is most
visible in expression storage, flycheck, configuration generation, and the VS Code bootstrap code.

## Verus functionality carried forward

### Syntax and parser

The parser continues to recognize Verus language extensions, including:

- `spec`, `proof`, `exec`, `ghost`, `tracked`, `broadcast`, `open`, and related item modifiers.
- Function contracts such as `requires`, `recommends`, `ensures`, `returns`, `decreases`,
  `opens_invariants`, and `no_unwind`.
- Verus expressions and operators such as `assert`, `assume`, `assert forall`, `choose`, `forall`,
  `exists`, `final`, view expressions, implication, equivalence, `is`, and `matches`.
- Verus-specific parameters, return values, closures, proof-function types, global declarations,
  and `assume_specification`.
- Verus modes on functions, constants, and statics.

Most Verus parsing is concentrated in `crates/parser/src/grammar/verus.rs`, with small integration
points in the normal Rust item, expression, type, parameter, attribute, and generic-parameter
grammars. Tokens remain contextual where possible so ordinary Rust identifiers are not
unnecessarily reclassified.

`crates/syntax/rust.ungram` is the source of truth for the concrete syntax tree. Changes there feed
the normal AST generator and the Verus syntax tree generator. The committed files
`crates/parser/src/syntax_kind/generated.rs`,
`crates/syntax/src/ast/generated/nodes.rs`, and
`crates/syntax/src/ast/generated/vst_nodes.rs` must not be edited by hand.

The final compatibility pass found and fixed several behaviors that had been lost during the
initial port:

- `final(...)` was restored as an expression-start token.
- `assert ... by {}` and `assert forall ... by {}` are treated as block-like expressions and do
  not require a trailing semicolon.
- `pub open(crate)` and `pub open(super)` are accepted.
- Function-level `by (nonlinear_arith)` is recognized as the prover clause rather than a malformed
  return type.
- `spec` and `exec` constants, plus `exec` statics, preserve their mode in CST and VST.

These cases are covered by `verus_regressions_parse_and_lift_to_vst` in
`crates/syntax/src/tests.rs`. The older syntax/VST corpus was also compiled against the new code;
all 61 cases passed.

### HIR and type inference

Verus expressions are represented in the current `hir-def` expression model rather than being
kept only as syntax:

- `Expr::Assert`
- `Expr::AssertForall`
- `Expr::Assume`

Lowering, source maps, pretty-printing, expression walking, type inference, mutability analysis,
closure capture analysis, and MIR lowering were updated for these variants. Verus proof constructs
generally produce unit, while their conditions and optional proof blocks are still inferred and
walked. This is important for ordinary IDE features inside Verus code, not just for verifier
execution.

The old fork modified the former body representation directly. Current upstream separates
expression data into `ExprStore` and related body structures, so the port follows those ownership
and traversal APIs. Future syntax additions that introduce expressions need to update all relevant
visitors; exhaustive compiler errors are useful, but not every visitor is expressed as an
exhaustive match.

### Verus syntax tree and proof actions

Proof actions use a mutable, printable Verus syntax tree (VST), distinct from the immutable rowan
CST. Its implementation is split into:

- `xtask/src/codegen/grammar/sourcegen_vst.rs`: generates VST structs, CST conversions, display
  implementations, and constructors from the ungrammar.
- `crates/syntax/src/ast/generated/vst_nodes.rs`: committed generated output.
- `crates/syntax/src/ast/vst.rs`: handwritten behavior for nodes that cannot be generated cleanly.
- `crates/ide-assists/src/proof_plumber_api`: VST traversal, semantic helpers, formatting, Verus
  invocation, and verifier-error types.
- `crates/ide-assists/src/handlers/proof_action`: individual proof-oriented assists.

Structural proof actions remain normal assists. Experimental actions that invoke Verus repeatedly
are compiled for verifier-backed integration tests through the `verus-integration-tests` feature,
but are not registered in production. This preserves their regression coverage without exposing
unfinished actions in the extension.

Proof-action Verus runs still use a unique temporary directory as both the scratch-file location
and process working directory. This prevents concurrent tests or actions from sharing Verus output
files. The verifier-backed suite passes both serially and in parallel.

### Verifier execution and diagnostics

Verifier-on-save is now integrated into the current flycheck implementation in
`crates/rust-analyzer/src/flycheck.rs`. Upstream moved flycheck from a standalone crate into the
language-server crate after the old fork point, so the old implementation could not be copied in
place.

`FlycheckConfig::VerusCommand` supports two execution modes:

- Direct `verus` execution.
- `cargo-verus verify` when `cargo.verusEnable` is set.

The current implementation:

- Uses `VERUS_BINARY_PATH` when supplied and otherwise falls back to `verus` on `PATH`.
- Finds the enclosing Cargo project and chooses its `src/main.rs` or `src/lib.rs` when needed.
- Restricts verification to the module containing the saved file unless
  `verus.verificationScope` is set to `crate`.
- Reads extra verifier arguments from `[package.metadata.verus.ide]` with a TOML parser rather than
  scanning manifest lines.
- Preserves normal Cargo target, feature, environment, and package selection behavior for
  `cargo-verus`.
- Parses rustc-compatible JSON diagnostics and the Verus verification summary in the existing
  flycheck stream.

`crates/rust-analyzer/src/verus_interaction.rs` converts relevant diagnostics into proof-action
precondition, postcondition, and assertion failures. The conversion now looks up primary and
secondary spans safely instead of indexing an assumed two-element span list. Errors are stored per
flycheck instance and file, merged into the immutable analysis snapshot, and supplied to assists
through the `ide` API.

### VS Code installation and startup

The extension keeps Verus installation in `editors/code/src/bootstrap.ts`. The current flow:

1. Honors the configured `verus-analyzer.verus.binary` path.
2. Reuses a valid installation in VS Code global storage.
3. Otherwise queries the latest Verus GitHub release and selects the platform asset.
4. Downloads to a staging location, extracts with platform tools, validates the executable, and
   atomically installs the extracted release.
5. Passes `VERUS_BINARY_PATH` only to the language-server process.

This replaces the older bootstrap implementation, which mixed installation with legacy
rust-analyzer startup code, mutated the extension host's process environment, and encoded more
platform and toolchain assumptions directly.

Current Verus releases print multiline version information. The extension parses both the Verus
version and its declared Rust toolchain, displays the Verus version in the status tooltip, and
warns if the required toolchain is missing. Startup is not blocked by that warning. CI uses the
same principle: it reads the toolchain from `verus --version` and installs that exact toolchain
instead of maintaining a hard-coded version.

### Branding, protocol, and release artifacts

User-visible identifiers are consistently branded as `verus-analyzer`:

- VS Code commands and configuration keys.
- Extension display text and output channels.
- LSP extension method names.
- Server and proc-macro-server artifact names.
- VSIX, release archive, repository, and marketplace metadata.
- Generated configuration descriptions.

Some `rust-analyzer` strings intentionally remain because they are compatibility or implementation
identifiers, not branding:

- The internal Rust crate and binary package names.
- `rust-analyzer.toml`.
- `#[rust_analyzer::rust_fixture]`.
- `[package.metadata.rust-analyzer]`.
- Links to the upstream rust-analyzer manual.

Renaming those would create incompatibility or recurring merge cost without improving the
user-facing fork.

CI was rebuilt on the current upstream workflow rather than retaining the old standalone
`.github/workflows/verus.yml`. The main CI workflow now downloads the current Verus release,
installs the release-declared toolchain, and runs verifier-backed proof-action tests. Release
packaging uses the current xtask and extension build, while preserving `verus-analyzer` binary,
archive, and VSIX names.

## Current code orientation

The shortest route into a Verus feature depends on the layer:

| Area | Primary locations |
| --- | --- |
| Token and node definitions | `xtask/src/codegen/grammar/ast_src.rs`, `crates/syntax/rust.ungram` |
| Parser | `crates/parser/src/grammar/verus.rs` and nearby Rust grammar modules |
| Generated CST | `crates/parser/src/syntax_kind/generated.rs`, `crates/syntax/src/ast/generated` |
| HIR lowering | `crates/hir-def/src/expr_store/lower.rs`, `crates/hir-def/src/hir.rs` |
| Type and use analysis | `crates/hir-ty/src/infer`, `crates/hir-ty/src/mir`, closure analysis |
| VST generation | `xtask/src/codegen/grammar/sourcegen_vst.rs` |
| VST handwritten behavior | `crates/syntax/src/ast/vst.rs` |
| Proof-action framework | `crates/ide-assists/src/proof_plumber_api` |
| Individual proof actions | `crates/ide-assists/src/handlers/proof_action` |
| Verifier command and output | `crates/rust-analyzer/src/flycheck.rs` |
| Diagnostic-to-proof conversion | `crates/rust-analyzer/src/verus_interaction.rs` |
| LSP state and routing | `crates/rust-analyzer/src/global_state.rs`, `main_loop.rs`, and `handlers` |
| Server configuration | `crates/rust-analyzer/src/config.rs` |
| Verus download and startup | `editors/code/src/bootstrap.ts`, `editors/code/src/ctx.ts` |
| User-visible editor schema | Generated sections of `editors/code/package.json` |
| Distribution | `xtask/src/dist.rs`, `.github/workflows/release.yaml` |
| Verus test commands | `CONTRIBUTING.md` |

The normal dependency direction still applies: parser to syntax, syntax to HIR, HIR to IDE, IDE to
the language server, and language server to the editor client. Verifier process management belongs
at the language-server boundary; proof transformations belong in `ide-assists`; generated syntax
types belong in `syntax`.

When changing syntax, edit the grammar and generator inputs first, run `cargo xtask codegen`, and
inspect all generated changes. Do not patch generated CST or VST files directly. The maintenance
procedure is summarized in [Maintaining the Verus fork](verus-upstream-sync.md), and the full test
matrix is in [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Significant upstream changes since 2024

The upstream interval is too large to enumerate commit by commit, but several architectural changes
directly affected the Verus port.

### Incremental database and type system

Upstream migrated through major Salsa revisions and now uses Salsa 0.28.2 with tracked query APIs.
Several old database traits and query-group patterns disappeared. Interning was also reorganized,
including the later `intern-db` work.

Type inference, MIR, and public HIR types moved substantially toward rustc's next trait solver.
Verus HIR variants therefore had to be integrated into current inference and traversal code rather
than copied from their former locations.

### Expression storage and HIR organization

Function-body expression data was split and reorganized around `ExprStore`, with new visitors,
source maps, scope handling, and support for expressions outside conventional bodies. This was the
largest semantic adaptation for Verus syntax. It also means future syncs should treat expression
visitor coverage as a first-class compatibility check.

### Parser and editions

The parser became edition-aware at the token and file level, syntax kinds and token information are
generated differently, and a separate `edition` crate now owns edition data. The grammar and AST
generators also gained new conventions. The Verus parser was adapted to contextual-token handling
instead of restoring the old generated token files.

The new baseline brings current Rust syntax and Edition 2024 support, along with two years of parser
recovery, macro expansion, inference, diagnostics, assists, and completion fixes.

### Flycheck and project loading

Flycheck moved into `crates/rust-analyzer`, and its command construction now handles workspace
discovery, package scopes, target directories, toolchains, and command substitution in one place.
Project loading, sysroot discovery, `rust-project.json`, and workspace configuration also changed
substantially. Verus verification is now another flycheck command origin instead of a parallel
process subsystem.

### Configuration and editor client

Configuration is more strongly generated from `config.rs`, with changed workspace-level semantics
and invocation strategies. The checked-in `package.json` and configuration documentation are
generated outputs and must stay synchronized with the Rust schema.

The VS Code extension changed its bootstrap, logging, toolchain selection, test explorer, build
pipeline, and dependency versions. The synced extension targets VS Code 1.93 or newer, Node 20 for
the bundle, and current TypeScript and ESLint tooling. Verus installation was reimplemented as a
small addition to this current client rather than retaining the old extension wholesale.

### Robustness and feature coverage

The new baseline contains extensive improvements in proc-macro isolation, malformed-code recovery,
incrementality, diagnostics, assists, and support for newer Rust compiler behavior. Preserving this
upstream robustness was a reason to port only the Verus delta instead of resolving the sync by
favoring old fork files.

## Validation performed

The completed branch passed:

- The 61-case legacy Verus syntax/VST compatibility corpus.
- `cargo test -p parser -p syntax --lib` with 382 tests at the compatibility checkpoint.
- Focused syntax, HIR, proof-action, and flycheck suites.
- The verifier-backed proof-action suite with 48 tests, both serially and in parallel.
- `cargo nextest run --no-fail-fast`: 8,153 passed and 10 configured tests skipped.
- The repository's exact Clippy gate.
- `cargo fmt --all -- --check`.
- `cargo codegen --check`.
- TypeScript type checking, ESLint, and Prettier.
- VSIX packaging.
- `cargo xtask dist --client-patch-version 0` for the Linux host target.

The VS Code Electron test harness was built and launched, but this host had neither an X server nor
`Xvfb`, so the UI-hosted unit suite could not complete locally. The TypeScript tests compile, and
the newly added Verus version parser test is part of that harness.

## Remaining maintenance considerations

- `main` still points at the archived pre-sync fork until the sync branch is deliberately merged.
- Experimental verifier-invoking proof actions remain test-only.
- Proof-action scratch verification is intentionally single-file and does not yet model an entire
  multi-file Verus project.
- The VST generator is a sizable fork-specific component and is likely to be the highest-conflict
  generated-code surface in future syncs.
- Verus and rust-analyzer have distinct Rust toolchain requirements. The extension and CI should
  continue reading the Verus release's declared toolchain rather than coupling it to the
  analyzer's workspace `rust-version`.
- Future upstream updates should start from current upstream and replay the Verus layers in the
  same dependency order, using the archive branch and checkpoint commits for behavior comparison.

## docs/dev/verus-upstream-sync.md (+3 -0)

For the baseline, implementation decisions, and validation from the 2026 sync, see
[2026 rust-analyzer sync summary](2026-rust-analyzer-sync.md).
