use ast::make;
use hir::{db::HirDatabase, PathResolution, Semantics};
use ide_db::{
    base_db::{FileId, FileRange, fixture::WithFixture},
    defs::Definition,
    search::FileReference,
    syntax_helpers::{insert_whitespace_into_node::insert_ws_into, node_ext::expr_as_name_ref},
    RootDatabase,
    imports::insert_use::{ImportGranularity, InsertUseConfig},
};
use itertools::izip;
use syntax::{
    ast::{self, edit_in_place::Indent, HasArgList, PathExpr, make::block_expr_from_predicates, HasModuleItem, Fn},
    ted, AstNode, SyntaxKind,
};

use crate::{
    assist_context::{AssistContext, Assists},
    AssistId, AssistKind,
};

use hir::db::DefDatabase;
use ide_db::base_db::SourceDatabaseExt;
use ide_db::SnippetCap;
use crate::AssistConfig;

pub(crate) const TEST_CONFIG: AssistConfig = AssistConfig {
    snippet_cap: SnippetCap::new(true),
    allowed: None,
    insert_use: InsertUseConfig {
        granularity: ImportGranularity::Crate,
        prefix_kind: hir::PrefixKind::Plain,
        enforce_granularity: true,
        group: true,
        skip_glob_imports: true,
    },
};


pub(crate) fn intro_requires(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    dbg!("hey1");
    let name_ref: ast::NameRef = ctx.find_node_at_offset()?;
    dbg!("hey11");
    let call_info = CallInfo::from_name_ref(name_ref.clone())?;
    dbg!("hey2");
    let (function, _label) = match &call_info.node {
        ast::CallableExpr::Call(call) => {
            let path = match call.expr()? {
                ast::Expr::PathExpr(path) => path.path(),
                _ => None,
            }?;
            let function = match ctx.sema.resolve_path(&path)? {
                PathResolution::Def(hir::ModuleDef::Function(f)) => f,
                _ => return None,
            };
            (function, format!("Inline `{}`", path))
        }
        // for now dont care
        ast::CallableExpr::MethodCall(_call) => {
            // (ctx.sema.resolve_method_call(call)?, format!("Inline `{}`", name_ref))
            return None;
        }
    };
    dbg!("hey3");

    let fn_source = ctx.sema.source(function)?;
    let fn_body = fn_source.value.body()?;
    let param_list = fn_source.value.param_list()?;



    dbg!("hey4");
    // let FileRange { file_id, range } = fn_source.syntax().original_file_range(ctx.sema.db);
    // allow recursive...
    // if file_id == ctx.file_id() && range.contains(ctx.offset()) {
    //     cov_mark::hit!(inline_call_recursive);
    //     return None;
    // }
    

    let params = get_fn_params(ctx.sema.db, function, &param_list)?;

    if call_info.arguments.len() != params.len() {
        // Can't inline the function because they've passed the wrong number of
        // arguments to this function
        cov_mark::hit!(inline_call_incorrect_number_of_arguments);
        return None;
    }

    let syntax = call_info.node.syntax().clone();
    dbg!("hey5");





    let requires = fn_source.value.requires_clause()?;
    let first_req = requires.expr()?;
    dbg!(&first_req);
    let mut req_vec = vec![first_req.clone()];


    let mut requires_clauses = requires.comma_and_conds();
    while let Some(req) = requires_clauses.next() {
        let req_without_comma = req.condition()?;
        dbg!(&req_without_comma);
        req_vec.push(req_without_comma.clone());
    }

    let req_as_body = block_expr_from_predicates(&req_vec);


    // clone
    // clone_subtree
    // clone_for_update
    // let mut temp_fn = fn_source.value.clone_subtree(); // is this deep copy????
    let mut temp_fn = fn_source.value.clone_for_update();
    ted::replace(temp_fn.body()?.syntax(), req_as_body.syntax().clone_for_update());

    dbg!(&temp_fn);
    // if self.body().is_none() {
    //     let body = make::ext::empty_block_expr().clone_for_update();
    //     match self.semicolon_token() {
    //         Some(semi) => {
    //             ted::replace(semi, body.syntax());
    //             ted::insert(Position::before(body.syntax), make::tokens::single_space());
    //         }
    //         None => ted::append_child(self.syntax(), body.syntax()),
    //     }
    // }

    let mut temp_fn_str = temp_fn.to_string();
    temp_fn_str.insert_str(0,"$0");
    let (mut db, file_with_caret_id, range_or_offset) = RootDatabase::with_range_or_offset(&temp_fn_str);
    db.set_enable_proc_attr_macros(true);
    let text_without_caret = db.file_text(file_with_caret_id).to_string();
    let frange = FileRange { file_id: file_with_caret_id, range: range_or_offset.into() };
    let sema = Semantics::new(&db);
    let config = TEST_CONFIG;
    let tmp_ctx = AssistContext::new(sema, &config, frange);
    let tmp_foo = tmp_ctx.find_node_at_offset::<ast::Fn>()?;
    dbg!(&tmp_foo);


    let tmp_body = tmp_foo.body()?;

    
    let tmp_param_list = tmp_foo.param_list()?;
    
    let tmp_function = tmp_ctx.sema.to_def(&tmp_foo)?;
    dbg!(&tmp_function);
    let tmp_params = get_fn_params(tmp_ctx.db(), tmp_function , &tmp_param_list)?;

    // let params = get_fn_params(ctx.sema.db, function, &param_list)?;


    let mut where_to_insert = call_info.node.syntax().text_range().start();
    for ancestor in  call_info.node.syntax().ancestors() {
        match ancestor.kind() {
            SyntaxKind::EXPR_STMT | SyntaxKind::LET_STMT => {
                where_to_insert = ancestor.text_range().start();
                break;
            }
            _ => (),
        }
        dbg!(ancestor.kind());
    }



    acc.add(
        AssistId("intro_requires", AssistKind::RefactorInline),
        "Intro Requires",
        syntax.text_range(),
        |builder| {
            dbg!(&req_as_body);
            // let replacement = inline(&ctx.sema, file_id, function, &req_as_body, &params, &call_info);
            let replacement = inline(&tmp_ctx.sema, file_with_caret_id, tmp_function, &tmp_body, &tmp_params, &call_info);

            // builder.replace_ast(
            //     match call_info.node {
            //         ast::CallableExpr::Call(it) => ast::Expr::CallExpr(it),
            //         ast::CallableExpr::MethodCall(it) => ast::Expr::MethodCallExpr(it),
            //     },
            //     replacement,
            // );
            builder.insert(
                where_to_insert,
                replacement.to_string(),
            );
        },
    )
    
}



struct CallInfo {
    node: ast::CallableExpr,
    arguments: Vec<ast::Expr>,
    generic_arg_list: Option<ast::GenericArgList>,
}

impl CallInfo {
    fn from_name_ref(name_ref: ast::NameRef) -> Option<CallInfo> {
        let parent = name_ref.syntax().parent()?;
        if let Some(call) = ast::MethodCallExpr::cast(parent.clone()) {
            let receiver = call.receiver()?;
            let mut arguments = vec![receiver];
            arguments.extend(call.arg_list()?.args());
            Some(CallInfo {
                generic_arg_list: call.generic_arg_list(),
                node: ast::CallableExpr::MethodCall(call),
                arguments,
            })
        } else if let Some(segment) = ast::PathSegment::cast(parent) {
            let path = segment.syntax().parent().and_then(ast::Path::cast)?;
            let path = path.syntax().parent().and_then(ast::PathExpr::cast)?;
            let call = path.syntax().parent().and_then(ast::CallExpr::cast)?;

            Some(CallInfo {
                arguments: call.arg_list()?.args().collect(),
                node: ast::CallableExpr::Call(call),
                generic_arg_list: segment.generic_arg_list(),
            })
        } else {
            None
        }
    }
}

fn get_fn_params(
    db: &dyn HirDatabase,
    function: hir::Function,
    param_list: &ast::ParamList,
) -> Option<Vec<(ast::Pat, Option<ast::Type>, hir::Param)>> {
    let mut assoc_fn_params = function.assoc_fn_params(db).into_iter();

    let mut params = Vec::new();
    if let Some(self_param) = param_list.self_param() {
        // FIXME this should depend on the receiver as well as the self_param
        params.push((
            make::ident_pat(
                self_param.amp_token().is_some(),
                self_param.mut_token().is_some(),
                make::name("this"),
            )
            .into(),
            None,
            assoc_fn_params.next()?,
        ));
    }
    for param in param_list.params() {
        params.push((param.pat()?, param.ty(), assoc_fn_params.next()?));
    }

    Some(params)
}

fn inline(
    sema: &Semantics<'_, RootDatabase>,
    function_def_file_id: FileId,
    function: hir::Function,
    fn_body: &ast::BlockExpr,
    params: &[(ast::Pat, Option<ast::Type>, hir::Param)],
    CallInfo { node, arguments, generic_arg_list }: &CallInfo,
) -> ast::Expr {
    let body = fn_body.clone_for_update();
    dbg!("inline 1");
    let usages_for_locals = |local| {
        Definition::Local(local)
            .usages(sema)
            .all()
            .references
            .remove(&function_def_file_id)
            .unwrap_or_default()
            .into_iter()
    };

    dbg!("inline2");
    let param_use_nodes: Vec<Vec<_>> = params
        .iter()
        .map(|(pat, _, param)| {
            if !matches!(pat, ast::Pat::IdentPat(pat) if pat.is_simple_ident()) {
                return Vec::new();
            }
            // FIXME: we need to fetch all locals declared in the parameter here
            // not only the local if it is a simple binding
            match param.as_local(sema.db) {
                Some(l) => usages_for_locals(l)
                    .map(|FileReference { name, range, .. }| match name {
                        ast::NameLike::NameRef(_) => body
                            .syntax()
                            .covering_element(range)
                            .ancestors()
                            .nth(3)
                            .and_then(ast::PathExpr::cast),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        })
        .collect();

    dbg!("inline 3");
    // Inline parameter expressions or generate `let` statements depending on whether inlining works or not.
    for ((pat, param_ty, _), usages, expr) in izip!(params, param_use_nodes, arguments).rev() {
        let inline_direct = |usage, replacement: &ast::Expr| {
            if let Some(field) = path_expr_as_record_field(usage) {
                cov_mark::hit!(inline_call_inline_direct_field);
                field.replace_expr(replacement.clone_for_update());
            } else {
                ted::replace(usage.syntax(), &replacement.syntax().clone_for_update());
            }
        };
        // izip confuses RA due to our lack of hygiene info currently losing us type info causing incorrect errors
        let usages: &[ast::PathExpr] = &*usages;
        let expr: &ast::Expr = expr;
        match usages {
            // inline single use closure arguments
            [usage]
                if matches!(expr, ast::Expr::ClosureExpr(_))
                    && usage.syntax().parent().and_then(ast::Expr::cast).is_some() =>
            {
                cov_mark::hit!(inline_call_inline_closure);
                let expr = make::expr_paren(expr.clone());
                inline_direct(usage, &expr);
            }
            // inline single use literals
            [usage] if matches!(expr, ast::Expr::Literal(_)) => {
                cov_mark::hit!(inline_call_inline_literal);
                inline_direct(usage, expr);
            }
            // inline direct local arguments
            [_, ..] if expr_as_name_ref(expr).is_some() => {
                dbg!("inline 3-1");
                cov_mark::hit!(inline_call_inline_locals);
                usages.iter().for_each(|usage| inline_direct(usage, expr));
            }
            // can't inline, emit a let statement
            _ => {
                dbg!("inline 3-2");
                // let ty =
                //     sema.type_of_expr(expr).filter(TypeInfo::has_adjustment).and(param_ty.clone());
                if let Some(stmt_list) = body.stmt_list() {
                    stmt_list.push_front(
                        make::let_stmt(pat.clone(), None, Some(expr.clone()))
                            .clone_for_update()
                            .into(),
                    )
                }
            }
        }
    }
    dbg!("inline 4");

    let original_indentation = match node {
        ast::CallableExpr::Call(it) => it.indent_level(),
        ast::CallableExpr::MethodCall(it) => it.indent_level(),
    };
    body.reindent_to(original_indentation);
    dbg!("inline 5");
    match body.tail_expr() {
        Some(expr) if body.statements().next().is_none() => expr,
        _ => match node
            .syntax()
            .parent()
            .and_then(ast::BinExpr::cast)
            .and_then(|bin_expr| bin_expr.lhs())
        {
            Some(lhs) if lhs.syntax() == node.syntax() => {
                make::expr_paren(ast::Expr::BlockExpr(body)).clone_for_update()
            }
            _ => ast::Expr::BlockExpr(body),
        },
    }
}

fn path_expr_as_record_field(usage: &PathExpr) -> Option<ast::RecordExprField> {
    let path = usage.path()?;
    let name_ref = path.as_single_name_ref()?;
    ast::RecordExprField::for_name_ref(&name_ref)
}




#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn intro_requires_easy() {
        check_assist(
            intro_requires,
            r#"
proof fn my_proof_fun(x: u32, y: u32)
    requires
        x > 0,
        y > 0,
    ensures
        x * y > 0,
{       
    let multiplied = x * y;
}

proof fn call_fun(a: u32, b: u32)
    requires
        a > 0,
        b > 0,
    ensures
        a * b > 0,
{
    my_proof_fun$0(a, b);
}
"#,
            r#"
proof fn my_proof_fun(x: u32, y: u32)
    requires
        x > 0,
        y > 0,
    ensures
        x * y > 0,
{       
    let multiplied = x * y;
}

proof fn call_fun(a: u32, b: u32)
    requires
        a > 0,
        b > 0,
    ensures
        a * b > 0,
{
    assert(a > 0);
    assert(b > 0);
    my_proof_fun(a, b);
}
"#,
        );
    }









    
    #[test]
    fn intro_requires_recursive() {
        check_assist(
            intro_requires,
            r#"
spec fn fibo(n: nat) -> nat
    decreases n
{
    if n == 0 { 0 } else if n == 1 { 1 }
    else { fibo((n - 2) as nat) + fibo((n - 1) as nat) }
}

proof fn lemma_fibo_is_monotonic(i: nat, j: nat)
    requires
        i <= j,
    ensures
        fibo(i) <= fibo(j),
    decreases j - i
{
    if i < 2 && j < 2 {
    } else if i == j {
    } else if i == j - 1 {
        reveal_with_fuel(fibo, 2);
        lemma_fibo_is_monotonic$0(i, (j - 1) as nat);
    } else {
        lemma_fibo_is_monotonic(i, (j - 1) as nat);
        lemma_fibo_is_monotonic(i, (j - 2) as nat);
    }
}   
"#,
            r#"
spec fn fibo(n: nat) -> nat
    decreases n
{
    if n == 0 { 0 } else if n == 1 { 1 }
    else { fibo((n - 2) as nat) + fibo((n - 1) as nat) }
}

proof fn lemma_fibo_is_monotonic(i: nat, j: nat)
    requires
        i <= j,
    ensures
        fibo(i) <= fibo(j),
    decreases j - i
{
    if i < 2 && j < 2 {
    } else if i == j {
    } else if i == j - 1 {
        reveal_with_fuel(fibo, 2);
        lemma_fibo_is_monotonic(i, (j - 1) as nat);
    } else {
        lemma_fibo_is_monotonic(i, (j - 1) as nat);
        lemma_fibo_is_monotonic(i, (j - 2) as nat);
    }
}   
            


"#,
        );
    }


}
