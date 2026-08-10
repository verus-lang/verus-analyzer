# Maintaining the Verus fork

For the baseline, implementation decisions, and validation from the 2026 sync, see
[2026 rust-analyzer sync summary](2026-rust-analyzer-sync.md).

Keep upstream syncs separate from Verus feature work whenever possible. Start from a tagged archive
of the previous fork, merge or rebase onto the selected rust-analyzer revision, and port the
Verus-specific layers in dependency order:

1. Grammar, parser, and generated CST.
2. HIR lowering, inference, traversal, and formatting.
3. Generated VST and proof actions.
4. Verifier execution and diagnostic routing.
5. Editor installation, settings, protocol branding, and distribution.
6. Documentation and the complete test matrix in `CONTRIBUTING.md`.

Do not replay old rust-analyzer files wholesale. Current upstream structure and APIs should remain
the default; transplant only the behavior required for Verus.  In particular, retain upstream 
tests and configuration unless a Verus requirement conflicts with them.
However, do not retain upstream CI; we only want CI focused on verus-analyzer's tests and release.

The main fork-specific surfaces are:

- Contextual Verus keywords, contracts, proof/spec modes, verifier expressions, and operators.
- VST generation and Proof Plumber APIs used by proof actions.
- Verifier command construction, JSON diagnostic parsing, and proof-action error context.
- VS Code Verus acquisition and `VERUS_BINARY_PATH` propagation.
- User-facing `verus-analyzer` protocol, configuration, command, and artifact names.
- Fork CI and release workflow repository gates, VSIX names, and marketplace publishing.

Archive the pre-sync tip before each update and checkpoint each layer after its focused tests pass.
This keeps future conflict diagnosis and regression bisection tractable.

Before declaring a sync complete, refresh `upstream/master`, run both the self-contained nextest
suite and the verifier-backed proof-action suite, validate the TypeScript extension, and smoke-test
the host release bundle. Search release workflows and package metadata for stale upstream artifact
or repository names; internal Rust crate and binary names may remain `rust-analyzer` where changing
them would add no user-facing value.
