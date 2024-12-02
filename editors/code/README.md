# verus-analyzer

This extension provides support for the [Verus programming language](https://github.com/verus-lang/verus).
It is derived from [rust-analyzer](https://rust-analyzer.github.io/)

This extension is **experimental** and subject to change; some features are likely broken.
At present, it works best on small, self-contained Verus projects.  Anything more complex
will likely fail. 

## Verus-specific Features

- Support for Verus syntax
- Each time you save a file in your project, Verus runs and reports proof failures and warnings
- [Proof actions](https://www.andrew.cmu.edu/user/bparno/papers/proof-plumber.pdf) are an **experimental** 
  feature to assist developers when debugging proof failures.  They show up as
  light bulb icons in the IDE when you hover over a failed proof.


## Features Extended from Rust Analyzer

- [code completion] with [imports insertion]
- go to [definition], [implementation], [type definition]
- [find all references], [workspace symbol search], [symbol renaming]
- [types and documentation on hover]
- [inlay hints] for types and parameter names
- [semantic syntax highlighting]
- a lot of [assists (code actions)]
- apply suggestions from errors
- ... and many more, check out the [manual] to see them all

[code completion]: https://rust-analyzer.github.io/manual.html#magic-completions
[imports insertion]: https://rust-analyzer.github.io/manual.html#completion-with-autoimport
[definition]: https://rust-analyzer.github.io/manual.html#go-to-definition
[implementation]: https://rust-analyzer.github.io/manual.html#go-to-implementation
[type definition]: https://rust-analyzer.github.io/manual.html#go-to-type-definition
[find all references]: https://rust-analyzer.github.io/manual.html#find-all-references
[workspace symbol search]: https://rust-analyzer.github.io/manual.html#workspace-symbol
[symbol renaming]: https://rust-analyzer.github.io/manual.html#rename
[types and documentation on hover]: https://rust-analyzer.github.io/manual.html#hover
[inlay hints]: https://rust-analyzer.github.io/manual.html#inlay-hints
[semantic syntax highlighting]: https://rust-analyzer.github.io/manual.html#semantic-syntax-highlighting
[assists (code actions)]: https://rust-analyzer.github.io/manual.html#assists-code-actions
[manual]: https://rust-analyzer.github.io/manual.html

## Quick start

1. Install [rustup].
2. Install the [verus-analyzer extension].

[rustup]: https://rustup.rs
[verus-analyzer extension]: https://marketplace.visualstudio.com/items?itemName=verus-lang.verus-analyzer

## Configuration

This extension provides configurations through VSCode's configuration settings. All configurations are under `verus-analyzer.*`.

See [the Rust analyzer manual](https://rust-analyzer.github.io/manual.html#vs-code-2) for more information on VSCode-specific configurations.

## Communication

For usage and troubleshooting requests, please use the [Verus Zulip](https://verus-lang.zulipchat.com/).

## Documentation

See [rust-analyzer.github.io](https://rust-analyzer.github.io/) for more information about the original Rust analyzer.
