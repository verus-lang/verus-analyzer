export interface JsonProject {
    /// Path to the sysroot directory.
    ///
    /// The sysroot is where rustc looks for the
    /// crates that are built-in to rust, such as
    /// std.
    ///
    /// https://doc.rust-lang.org/rustc/command-line-arguments.html#--sysroot-override-the-system-root
    ///
    /// To see the current value of sysroot, you
    /// can query rustc:
    ///
    /// ```
    /// $ rustc --print sysroot
    /// /Users/yourname/.rustup/toolchains/stable-x86_64-apple-darwin
    /// ```
    sysroot?: string;
    /// Path to the directory with *source code* of
    /// sysroot crates.
    ///
    /// By default, this is `lib/rustlib/src/rust/library`
    /// relative to the sysroot.
    ///
    /// It should point to the directory where std,
    /// core, and friends can be found:
    ///
    /// https://github.com/rust-lang/rust/tree/master/library.
    ///
    /// If provided, verus-analyzer automatically adds
    /// dependencies on sysroot crates. Conversely,
    /// if you omit this path, you can specify sysroot
    /// dependencies yourself and, for example, have
    /// several different "sysroots" in one graph of
    /// crates.
    sysroot_src?: string;
    /// The set of crates comprising the current
    /// project. Must include all transitive
    /// dependencies as well as sysroot crate (libstd,
    /// libcore and such).
    crates: Crate[];
}

export interface Crate {
    /// Optional crate name used for display purposes,
    /// without affecting semantics. See the `deps`
    /// key for semantically-significant crate names.
    display_name?: string;
    /// Path to the root module of the crate.
    root_module: string;
    /// Edition of the crate.
    edition: "2015" | "2018" | "2021";
    /// Dependencies
    deps: Dep[];
    /// Should this crate be treated as a member of
    /// current "workspace".
    ///
    /// By default, inferred from the `root_module`
    /// (members are the crates which reside inside
    /// the directory opened in the editor).
    ///
    /// Set this to `false` for things like standard
    /// library and 3rd party crates to enable
    /// performance optimizations (verus-analyzer
    /// assumes that non-member crates don't change).
    is_workspace_member?: boolean;
    /// Optionally specify the (super)set of `.rs`
    /// files comprising this crate.
    ///
    /// By default, verus-analyzer assumes that only
    /// files under `root_module.parent` can belong
    /// to a crate. `include_dirs` are included
    /// recursively, unless a subdirectory is in
    /// `exclude_dirs`.
    ///
    /// Different crates can share the same `source`.
    ///
    /// If two crates share an `.rs` file in common,
    /// they *must* have the same `source`.
    /// verus-analyzer assumes that files from one
    /// source can't refer to files in another source.
    source?: {
        include_dirs: string[];
        exclude_dirs: string[];
    };
    /// The set of cfgs activated for a given crate, like
    /// `["unix", "feature=\"foo\"", "feature=\"bar\""]`.
    cfg: string[];
    /// Target triple for this Crate.
    ///
    /// Used when running `rustc --print cfg`
    /// to get target-specific cfgs.
    target?: string;
    /// Environment variables, used for
    /// the `env!` macro
    env: { [key: string]: string };

    /// Whether the crate is a proc-macro crate.
    is_proc_macro: boolean;
    /// For proc-macro crates, path to compiled
    /// proc-macro (.so file).
    proc_macro_dylib_path?: string;
}

export interface Dep {
    /// Index of a crate in the `crates` array.
    crate: number;
    /// Name as should appear in the (implicit)
    /// `extern crate name` declaration.
    name: string;
}
