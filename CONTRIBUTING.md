# Contributing to verus-analyzer

Thank you for your interest in contributing to verus-analyzer! There are many ways to contribute
and we appreciate all of them.

To get a quick overview of the crates and structure of the project take a look at the
[./docs/dev](./docs/dev) folder.

If you have any questions please ask them in the [Verus Zulip](https://verus-lang.zulipchat.com/) 
or if unsure where to start out when working on a concrete issue drop a comment
on the related issue for mentoring instructions (general discussions are
recommended to happen on Zulip though).

## Fixing a bug or improving a feature

Generally it's fine to just work on these kinds of things and put a pull-request out for it. If there
is an issue accompanying it make sure to link it in the pull request description so it can be closed
afterwards or linked for context.

If you want to find something to fix or work on keep a look out for the `C-bug` and `C-enhancement`
labels.

## Adding a new configuration option
There are two parts to the configuration information.  In
`crates/rust-analyzer/src/config.rs` defines the server-side configuration of
rust-analyzer.  Portions of that file are automatically copied into
`editors/code/package.json`, which also contains editor-specific (for VS Code)
configuration options.  Look for the `"title": "$generated-start"` entry in the
JSON file to see where the automatically copied portion appears.

If you update the config! settings in `crates/rust-analyzer/src/config.rs`, you
need to run:
`cargo test -p rust-analyzer generate_package_json_config generate_config_documentation`
to update the `package.json` file and the documentation (which we don't currently publish).


## Implementing a new feature

It's advised to first open an issue for any kind of new feature so the team can tell upfront whether
the feature is desirable or not before any implementation work happens. We want to minimize the
possibility of someone putting a lot of work into a feature that is then going to waste as we deem
it out of scope (be it due to generally not fitting in with rust-analyzer, or just not having the
maintenance capacity). If there already is a feature issue open but it is not clear whether it is
considered accepted feel free to just drop a comment and ask!

## Interacting with Verus

Verus runs each time you save a file in your project.  This is primarily implemented in
`crates/flycheck/lib.rs` in `run_verus`.

## Bumping the expected version of Rust when Verus upgrades to a new version

1. Update the Rust version used in the Verus CI workflow.
    - Specifically the line `rustup install 1.82.0-x86_64-unknown-linux-gnu` in `.github/workflows/verus.yml`
2. Update the Rust version the VS Code client looks for during installation
    - Specifically the line `const TOOLCHAIN_MAJOR = 82;` in `editors/code/src/bootstrap.ts`

## Adding or Changing Proof Actions

Proof action code primarily lives in `crates/ide-assists/src/`.  In `lib.rs`,
you can find `all()`, which contains the list of enabled IDE assists, including proof actions.
Each proof action has an implementation in `handlers/proof_action/[name].rs`.
To make it more pleasant to author proof actions, we provide the Proof Plumber API
in the `proof_plumber_api` directory.  Part of this API depends on lifting verus-analyzer's
CST to a VST (Verus Syntax Tree).  This part is largely automated via the code in `xtask/src/codegen/grammar/sourcegen_vst.rs`.

## How to update verus-analyzer when Verus syntax changes

### Summary:
1. Add a testcase to `crates/syntax/src/lib.rs`.
2. Update the `crates/syntax/rust.ungram` file and modify `xtask/src/codegen/grammar/ast_src.rs` if necessary.
3. Run `cargo xtask codegen grammar` to  auto-generate `crates/syntax/ast/generated/*` and `crates/parser/src/syntax_kind/generated.rs` files.
4. Update `parser` crate to parse new syntax item.
5. Run the new and existing syntax tests via `cargo test --package syntax --lib`
6. Test that proof actions still work by running:
```
cargo test --package ide-assists --lib -- handlers::proof_action
```
This currently requires setting `TMPDIR` and `VERUS_BINARY_PATH`

### Details:

#### Checking Verus syntax changes
- Before making changes to verus-analyzer, refer to Verus's `verus/dependencies/syn` crate to check how Verus handles the new syntax item. Although there are many differences between `syn` and rust-analyzer, it is helpful to keep them as similar as possible. 
For example, inside `verus/dependencies/syn/src/items.rs`, refer to `impl parse for Signature` to see how Verus parses a function signature. 

- For additional syntax information, refer to Verus's `verus/source/builtin_macros/src/syntax.rs`.

- `verus/source/rust_verify/examples/syntax.rs` contains syntax examples that can be handy for testcases. 

- [`verusfmt`](https://github.com/verus-lang/verusfmt) can also be a useful source of grammar documentation; see in particular the `src/verus.pest` file.  It can also provide useful test cases -- see `tests/verus-consistency.rs`


#### Modifying `syntax` and `parser` crates
Inside the `crates` directory, we need to modify several crates, but most changes will be made on the `parser` and `syntax` crates.
FWIW, these articles provide some helpful background on the philosophy behind how those crates work.
- [Introducing Ungrammar](https://rust-analyzer.github.io//blog/2020/10/24/introducing-ungrammar.html)
- [Simple but Powerful Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html)

1. Add a testcase to `crates/syntax/src/lib.rs`.

1. It can be a nuisance to initially develop syntax changes using `crates/syntax/src/lib.rs`,
   since the parser reports issues in terms of tokens from the start of the "file", which can 
   be hard to use in your favorite editor.  Hence, I often copy the relevant test(s) into
   `crates/va-test/examples/misc-parsing/src/main.rs`.  Then in `crates/va-test/examples/misc-parsing/`,
   running `cargo run -- misc-parsing` will attempt to parse the new code and then lift it to VST.
   This can be an easier and faster way to test changes.

2. Update `syntax/rust.ungram` with the new syntax. Also, update `xtask/src/codegen/grammar/ast_src.rs` for newly introduced tokens if there are any. 
  - In particular, you will need to update the `keywords` list for new keywords to be available

3. Run `cargo xtask codegen grammar` to  auto-generate `crates/syntax/ast/generated/*` and `crates/parser/src/syntax_kind/generated.rs` files.
  - This relies on these files `xtask/src/codegen/grammar/{ast_src.rs,sourcegen_vst.rs}` 

4. Add testcases. Add snippets of new Verus code at `syntax/src/lib.rs`, to make sure the new syntax is parsed correctly. `.github/workflows/verus.yml` will run these tests in the CI.

5. To modify the parser, start from `parser/src/grammar/verus.rs`. Verus specific lang items(e.g. `requires` `ensures`) should be parsed here. For modified items (e.g. `WhileExpr`), the parser is modified in-place. See `item.rs` and `expression.rs` for examples of these. The implicit rule is that for each “ungrammar” object, there is a function that parses that object. 

    For example:
    - For `AssertExpr`, we have `grammar::verus::assert` function to parse it. 
    - For `struct`, there is  `grammar::items::adt::struckt` function to parse struct.
    - For major syntax items, refer to `grammar/item.rs` file.

6. Test that proof actions still work by running:
```
cargo test --package ide-assists --lib -- handlers::proof_action
```
This currently requires setting the `TMPDIR` and `VERUS_BINARY_PATH` environment variables


#### Modifying the rest
Modify `hir-def` and `hit-ty` crates if necessary. The changes will be alerted
by the compiler("missing enum case"), and they can be largely straight forward.
These changes are needed for the IDE purposes(e.g. type inference, code
scanning, etc).  The best approach is to find an existing piece of syntax
similar to the one you added and mimic it.

#### Debugging issues lifting into VST
The VST (Verus Syntax Tree) representation is used by Verus Analyzer's proof actions.  
If you run `crates/va_test` on a workspace and see errors along the lines of:
```
Failed to lift function Name(Text("ptr_null_mut")): got error: Err("type_bound_list")
```
This means our auto-generated lifting code failed to convert the CST provided by
the parser into a VST representation.  To start debugging such an issue, try
searching through `crates/syntax/src/ast/generated/vst_nodes.rs` and
`crates/syntax/src/ast/vst.rs` for the term `stringify!(PROBLEM)`, where
PROBLEM is the type name in the `Err("PROBLEM")` above.  That may help locate
the problematic code.

## Building a VSIX file

This requires the `esbuild` tool to be installed.  On Mac OS, run `brew install esbuild`.

You may also need to install the `vscode-languageclient` package via:
```
npm install vscode-languageclient
```

We include a build of the server in the VSIX file, so in the base of this repo, run:
```
cargo xtask dist --proof-action --client-patch-version 42
```
which will cause a copy of the server to be placed in `editors/code/server/`
The number you pass in will be concatenated to "0.4" to form the extension's
version number.  The actual value does not matter.  Part of this process modifies
`verus-analyzer/editors/package.json`.  If you subsequently need to rebuild
the server after making changes, you typically need to restore the `package.json` file
to its original state, or else `cargo task dist` will panic.

Now, in `verus-analyzer/editors/code`, run:
```
npx vsce package -o ../../dist/verus-analyzer-aarch64-apple-darwin.vsix --target darwin-arm64
```
You should update `aarch64-apple-darwin` as appropriate.  Choices include:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `x86_64-pc-windows-msvc`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`

You should also update the argument to `--target` appropriately.  Choices include:
 - `win32-x64`
 - `win32-arm64`
 - `linux-x64`
 - `linux-arm64`
 - `linux-armhf`
 - `alpine-x64`
 - `alpine-arm64`
 - `darwin-x64`
 - `darwin-arm64`
 - `web`
If you don't pass the `--target` flag, the package will be used as a fallback
for all platforms that have no platform-specific package.

You can install the resulting `.vsix` file from the commandline.  In the base of the repo, run:
```
code --install-extension ./dist/verus-analyzer-[your-arch-choice].vsix
```
Or in VS Code, you can open the Extensions panel, click the '...' button in the upper-right
portion of the panel, and select "Install from VSIX..."

### Notes

If you see this complaint:
```
Cannot find base config file "@tsconfig/strictest/tsconfig.json"
```
Try running:
```
npm install --save-dev @tsconfig/strictest
yarn add --dev @tsconfig/strictest
```

