use clap::Parser as ClapParser;
use hir::{HasSource, HirFileIdExt, Semantics};
//use ide_assists::assist_context::AssistContext;
//use rust_analyzer;
use hir::{db::HirDatabase, Crate, Module, ModuleDef};
//use hir_def::{self, visibility::Visibility};
//use hir_ty::{self};
use base_db::{self, SourceDatabaseExt};
use load_cargo::*;
use project_model::CargoConfig;
use syntax::ast::vst;

use std::collections::HashSet;
use std::path::PathBuf;

//use ide_assists;
use syntax;

#[derive(ClapParser)]
#[command(version, about)]
struct Args {
    /// Workspace folder to load
    workspace: PathBuf,
}

fn all_modules(db: &dyn HirDatabase) -> Vec<Module> {
    let mut worklist: Vec<_> =
        Crate::all(db).into_iter().map(|krate| krate.root_module()).collect();
    let mut modules = Vec::new();

    while let Some(module) = worklist.pop() {
        modules.push(module);
        worklist.extend(module.children(db));
    }

    modules
}

fn main() {
    let args = Args::parse();

    // step1: Load workspace
    let cargo_config = CargoConfig::default();
    let load_cargo_config = LoadCargoConfig {
        load_out_dirs_from_check: true,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
    };

    let (db, vfs, _proc_macro) =
        { load_workspace_at(&args.workspace, &cargo_config, &load_cargo_config, &|_| {}).unwrap() };

    let work = all_modules(&db).into_iter().filter(|module| {
        let file_id = module.definition_source_file_id(&db).original_file(&db);
        let source_root = db.file_source_root(file_id);
        let source_root = db.source_root(source_root);
        !source_root.is_library
    });
    let mut visited_files = HashSet::new();

    // step2: TODO: setup assist context
    let _sema = Semantics::new(&db);
    // let ctx = AssistContext::new(sema, /* TODO*/ )

    // step3: visit every function in the project for some work
    for module in work {
        let file_id = module.definition_source_file_id(&db).original_file(&db);
        if !visited_files.contains(&file_id) {
            let crate_name =
                module.krate().display_name(&db).as_deref().unwrap_or("unknown").to_string();
            println!("processing crate: {crate_name}, module: {}", vfs.file_path(file_id));
            for def in module.declarations(&db) {
                if let ModuleDef::Function(foo) = def {
                    let fn_cst = foo.source(&db).expect("source not found");
                    //dbg!(&cst);
                    let fn_vst: vst::Fn = fn_cst.value.try_into().expect("vst lifting failure");
                    dbg!(&fn_vst.name);

                    // TODO: use the source-level proof rewrite
                    // probably using vst rewriting functions inside proof actions
                    // (need to make them public first -- currently most of them are private I think)
                }
            }
            visited_files.insert(file_id);
        }
    }
}
