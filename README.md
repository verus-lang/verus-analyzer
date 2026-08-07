# verus-analyzer

verus-analyzer is a fork of [rust-analyzer](https://github.com/rust-lang/rust-analyzer) that adds
IDE support for the [Verus](https://github.com/verus-lang/verus) verification language. It tracks
current rust-analyzer while adding:

- Parsing, highlighting, lowering, and analysis for Verus syntax.
- Verus verification on save, with diagnostics shown in the editor.
- Experimental proof actions for debugging failed proofs.
- Automatic Verus installation in the VS Code extension.

The project is experimental. Verus-specific analysis is not yet as complete as rust-analyzer's
analysis of ordinary Rust.

## Quick Start

Install the
[verus-analyzer VS Code extension](https://marketplace.visualstudio.com/items?itemName=verus-lang.verus-analyzer)
and open a Cargo project containing Verus code. The extension downloads the latest Verus release
for supported platforms and runs the verifier whenever a Rust file is saved.

The most relevant settings are:

- `verus-analyzer.verus.enable`: enable verification on save.
- `verus-analyzer.verus.binary`: use a specific Verus executable instead of downloading one.
- `verus-analyzer.verus.extraArgs`: pass additional arguments to Verus.
- `verus-analyzer.verus.reportAllErrorsEnable`: report errors from the whole crate instead of the
  module containing the saved file.
- `verus-analyzer.cargo.verusEnable`: run `cargo verus` instead of invoking Verus directly.

Verus arguments can also be specified in a project manifest:

```toml
[package.metadata.verus.ide]
extra_args = ["--rlimit", "20"]
```

## Proof Actions

Proof actions are experimental code actions based on
[Proof Plumber](https://www.andrew.cmu.edu/user/bparno/papers/proof-plumber.pdf). They appear as
light-bulb actions near failed assertions, preconditions, and postconditions and automate common
proof-debugging transformations.

## Development

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the Verus architecture and test matrix. The
[rust-analyzer manual](https://rust-analyzer.github.io/book/) remains the reference for inherited
editor features and internals.

Questions about Verus-specific behavior belong in the
[Verus Zulip](https://verus-lang.zulipchat.com/). Upstream rust-analyzer design discussions belong
in the [rust-analyzer Zulip stream](https://rust-lang.zulipchat.com/#narrow/stream/185405-t-compiler.2Frust-analyzer).

## License

verus-analyzer is distributed under the terms of the MIT and Apache 2.0 licenses. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
