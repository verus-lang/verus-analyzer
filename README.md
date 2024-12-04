# <a href="https://verus-lang.github.io/verus/verus/logo.html"><img height="30px" src="https://verus-lang.github.io/verus/verus/assets/verus-color.svg" alt="Verus" /></a> Verus-Analyzer
Verus-analyzer is a version of [rust-analyzer](https://github.com/rust-lang/rust-analyzer) that has
been modified to provide IDE support for writing [Verus](https://github.com/verus-lang/verus) code 
and proofs, including syntax support and various IDE features.

## WARNING!
This software is **experimental** and subject to change; some features are likely broken.
At present, it works best on small, self-contained Verus projects.  Anything more complex
will likely fail.  You may file issues, but we do not currently have dedicated engineering
support for `verus-analyzer`, so **your issue may not be addressed**.  Pull requests with
fixes are always welcome, although it is unlikely they will be reviewed immediately.

## Quick Start

### Requirements

The main requirement is that `verus-analyzer` expects you to "Open Folder..." on
a directory containing a standard Rust project layout and metadata (`Cargo.toml`) file.
`verus-analyzer` scans the project root (`src/lib.rs` or `src/main.rs`) and all files
that are reachable from the root. If the file you are working on is not
reachable from the project root, most of the IDE features like "Go to
Definition" will not work. For example, if you have a file named `foo.rs`
next to `main.rs`, but you do not import `foo.rs` in `main.rs`(i.e., you haven't added
`mod foo` in `main.rs`), then the IDE features will not work for `foo.rs`.

As mentioned above, `verus-analyzer` also expects to find a `Cargo.toml` metadata file,
as is in standard in Rust projects. For a small
project, you could start by running `cargo new`, which will automatically generate a
suitable `Cargo.toml` file for you. For a larger project, you could use a Rust
[workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html) to
manage multiple crates.

### Installation

Please install the `verus-analyzer` extension via the Visual Studio Code extension marketplace.

---
## Features and Details

### 1. Verus Syntax
We extended rust-analyzer's grammar for Verus-specific syntax. This means that it highlights reserved Verus keywords (e.g., `spec`, `proof`, `requires`, `ensures`). If a user types `prof` instead of `proof`, a syntax error will be generated.


### 2. IDE features
You can find more documentation of the IDE features by following these links.
- [Go to Definition](https://rust-analyzer.github.io/manual.html#go-to-definition)
- [Go to Type Declaration](https://rust-analyzer.github.io/manual.html#go-to-type-definition)
- [Find all References](https://rust-analyzer.github.io/manual.html#find-all-references)
- [Hover](https://rust-analyzer.github.io/manual.html#hover)

#### 2.1 TODOs for IDE features
- Code scanning is incomplete for Verus-specific items. To be specific, `requires`, `ensures`, `decreases`, `invariant`, `assert-by-block`, and `assert-forall-block` are not fully scanned for IDE purposes (e.g., you might not be able to use "Go to Definition" on a function mentioned in a `requires` or `ensures` expression, or "Find All References" might omit occurrences inside `requires` and `ensures` expressions).

- Although Verus' custom operators are parsed, they are not registered for IDE purposes. For example, type inference around such operators might not work (e.g., `A ==> B` is parsed as `implies(A, B)`, but the IDE might not be able to infer that `A` and `B` are Booleans).

- Currently, `builtin` and `vstd` are not scanned. For example, the builtin types like `int` and `nat` could be shown as `unknown`. Auto completion for `vstd` might not work.

### 3. Running Verus

Each time you save a file in your project, Verus should run and report proof failures and warnings in the IDE.

---
## Limitations
- This is experimental software and subject to change.
- It is intended to be used only for Verus code. 
- Multiple features of `rust-analyzer` might be broken or missing.
- Syntax might not be updated to the latest version of Verus.

## Misc
- The `verus-analyzer: Clear flycheck diagnostics` command can be used to clear the error messages in VS Code
- The `Developer: Reload Window` command can be used to reload VS Code and the verus-analyzer server instead of closing and reopening VS Code
- Setting `"rust-analyzer.diagnostics.disabled": ["syntax-error"]` in your workspace's settings can disable the syntax error messages in VS Code. You could also add `unresolved-module` to the above list to disable the error message for unresolved modules.
- There is no proper support for `buildin`/`vstd`. However, in your project's `Cargo.toml` file, you can add `vstd` in `dependencices` or `dev-dependencies`, which might make `verus-analyzer` scan `vstd` and `builtin`. For example, you can try adding:
```
[dependencies]
vstd = { path = "../verus/source/vstd"}  # assuming verus and the project are in the same directory
```

---

## Proof Actions

[Proof actions](https://www.andrew.cmu.edu/user/bparno/papers/proof-plumber.pdf) 
are an **experimental** feature to assist developers when debugging proof failures.
They show up as light bulb icons in the IDE when you hover over a failed proof.

### Proof Action Demo
[Source code](https://github.com/chanheec/proof-action-example)

![](demo.gif)
