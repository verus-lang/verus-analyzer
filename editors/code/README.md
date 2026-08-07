# verus-analyzer

This extension provides IDE support for the
[Verus verification language](https://github.com/verus-lang/verus). It is derived from
[rust-analyzer](https://github.com/rust-lang/rust-analyzer) and remains compatible with ordinary
Rust syntax.

## Verus Features

- Verus syntax parsing and highlighting
- Verification on save with editor diagnostics
- Automatic installation of the latest Verus release
- Experimental proof actions for debugging failed proofs

## Inherited Rust Features

- [code completion] with [imports insertion]
- go to [definition], [implementation], [type definition]
- [find all references], [workspace symbol search], [symbol renaming]
- [types and documentation on hover]
- [inlay hints] for types and parameter names
- [semantic syntax highlighting]
- a lot of [assists (code actions)]
- apply suggestions from errors
- ... and many more, check out the [manual] to see them all

[code completion]: https://rust-analyzer.github.io/book/features.html#magic-completions
[imports insertion]: https://rust-analyzer.github.io/book/features.html#completion-with-autoimport
[definition]: https://rust-analyzer.github.io/book/features.html#go-to-definition
[implementation]: https://rust-analyzer.github.io/book/features.html#go-to-implementation
[type definition]: https://rust-analyzer.github.io/book/features.html#go-to-type-definition
[find all references]: https://rust-analyzer.github.io/book/features.html#find-all-references
[workspace symbol search]: https://rust-analyzer.github.io/book/features.html#workspace-symbol
[symbol renaming]: https://rust-analyzer.github.io/book/features.html#rename
[types and documentation on hover]: https://rust-analyzer.github.io/book/features.html#hover
[inlay hints]: https://rust-analyzer.github.io/book/features.html#inlay-hints
[semantic syntax highlighting]: https://rust-analyzer.github.io/book/features.html#semantic-syntax-highlighting
[assists (code actions)]: https://rust-analyzer.github.io/book/assists.html
[manual]: https://rust-analyzer.github.io/book/features.html

## Quick start

1. Install [rustup] and a Rust toolchain compatible with your Verus project.
2. Install the [verus-analyzer extension].
3. Open a Cargo project containing Verus code.

The extension downloads Verus automatically on supported platforms. Configure
`verus-analyzer.verus.binary` to use an existing Verus build, or disable verification with
`verus-analyzer.verus.enable`.

[rustup]: https://rustup.rs
[verus-analyzer extension]: https://marketplace.visualstudio.com/items?itemName=verus-lang.verus-analyzer

## Configuration

This extension provides configurations through VSCode's configuration settings. All configurations are under `verus-analyzer.*`.

See the [verus-analyzer repository] for Verus settings and the
[rust-analyzer manual](https://rust-analyzer.github.io/book/editor_features.html#vs-code) for
inherited VS Code settings.

## Communication

For usage and troubleshooting, use the [Verus Zulip](https://verus-lang.zulipchat.com/).

[verus-analyzer repository]: https://github.com/verus-lang/verus-analyzer
