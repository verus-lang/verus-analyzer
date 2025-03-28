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

#### Extra Arguments
To pass extra arguments to Verus, add the following table to the `Cargo.toml` file for your Verus project:
```
[package.metadata.verus.ide]
extra_args = "......"
```
where the quoted string is a list of space-separated Verus arguments, e.g., `extra_args = "--rlimit 20 --log-all"`.

#### Advanced Verus Developments
Some advanced Verus projects (e.g., those making changes to `vstd`) may need to use `#[cfg(verus_keep_ghost)]`
in their Verus files.  This will cause various `verus-analyzer` features (like Go To Definition) to stop working,
since `verus-analyzer` won't recognize that `cfg` setting by default.  To address that, edit your VS Code `settings.json`
file to add:
```
    "verus-analyzer.cargo.cfgs": {
        "verus_keep_ghost": null,
        "debug_assertions": null,
        "miri": null
    },
```
In the future, when we sync up with the latest version of `rust-analyzer`, you will need this setting instead:
```
    "verus-analyzer.cargo.cfgs": [
        "debug_assertions",
        "miri",
        "verus_keep_ghost"
    ],
```
since `rust-analyzer` changed the type it expects for this setting.

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

TODO: More details + light bulb icon


[Proof actions](https://www.andrew.cmu.edu/user/bparno/papers/proof-plumber.pdf) 
are an **experimental** feature to assist developers when debugging proof failures.
They show up as light bulb icons in the IDE when you hover over a failed proof.

### Proof Action Demo
[Source code](https://github.com/chanheec/proof-action-example)

![](demo.gif)

### Currently Enabled Proof Actions

The "Hover over" column indicates where you should place your mouse cursor in
so that a "light bulb" will appear and allow you to perform the corresponding
proof action.  In the examples linked to below, the `$0$` characters indicate
where the user has positioned their mouse in the "before" version of the code,
and below you can see the version after the proof action executes.

| Hover over | Proof action | Examples |
|------------|--------------|----------|
| `assert` keyword | Move the current expression "up" one statement in the current function, adjusting it appropriately based on the statement it "moves past" (technically it applies one weakest-precondition step).  Currently only handles a subset of available Verus statements. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/weakest_pre_step.rs#L303) |
| `assert` keyword | Convert `assert(A ==> B)` into `if A { assert(B); }` | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/convert_imply_to_if.rs#L71) |
| `assert` keyword | Take an assertion containing a `forall` quantifier and an implication and introduce a [`forall ... implies ... by` statement](https://verus-lang.github.io/verus/guide/quantproofs.html#proving-forall-with-assert-by) where the quantified variables are in scope and the proof already assumes the left-hand side of the implication. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/intro_forall_implies.rs#L98) |
| `assert` keyword | Take an assertion containing a `forall` quantifier and introduce a `by` clause where the quantified variables are in scope. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/intro_forall.rs#L71) |
| `assert` keyword | Add a `by` block to an existing assertion. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/insert_assert_by_block.rs#L64) |
| `assert` keyword | Add a `by` block containing `assume(false)`. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/intro_assume_false.rs#L66) |
| `ensures` keyword | Introduce a failing ensures clause at the end of the current function | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/insert_failing_postcondition.rs#L92) |
| `ensures` keyword | Take an ensures clause `A ==> B`, and move `A` to the requires clause, leaving `B` in the ensures clause. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/split_imply_ensures.rs#L75) |
| function call | Introduce a failing precondition in the caller's context. |[code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/insert_failing_precondition.rs#L64) |
| function call inside an assertion | Add a reveal statement for this function above the current assertion. | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/reveal_opaque_above.rs#L78) |
| function call inside an assertion | Convert the assertion into an `assert ... by` expression and reveal the selected function's definition inside the `by` block | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/reveal_opaque_in_by_block.rs#L76) |
| `<=` | Split an assertion of `A <= B` into two assertions: `A < B` and `A <= B` | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/split_smaller_or_equal_to.rs#L118) |
| sequence expression inside an `assert ... by` | Adds a clause saying that the sequence index is in bounds | [code](https://github.com/verus-lang/verus-analyzer/blob/55279b828ea54a79916b528567f3919f6eac6fc0/crates/ide-assists/src/handlers/proof_action/seq_index_inbound.rs#L99) |


### Developing Your Own Proof Action

(crates/ide-assists/src/lib.rs)

# TODO

Point to CONTRIBUTING for more details
