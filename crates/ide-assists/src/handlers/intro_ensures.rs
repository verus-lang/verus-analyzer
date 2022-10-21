use syntax::{ast::{self, make::{assert_stmt_from_predicate, let_stmt, ext::ident_path, expr_path}}, AstNode};
// use syntax::{ast, match_ast, AstNode, SyntaxKind, SyntaxToken, TextRange, TextSize};

use crate::{AssistContext, AssistId, AssistKind, Assists};


pub(crate) fn intro_ensures(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    let func = ctx.find_node_at_offset::<ast::Fn>()?;
    let ensures = func.ensures_clause()?; 
    let ensures_keyword = ensures.ensures_token()?;
    let ensures_range = ensures_keyword.text_range();
    let cursor_in_range = ensures_range.contains_range(ctx.selection_trimmed());
    if !cursor_in_range {
        return None;
    }
    let new_func = code_transformer_intro_ensures(func.clone())?;
    acc.add(AssistId("intro_ensures", AssistKind::RefactorRewrite), "Copy ensures to last", ensures_range, |edit| {
        edit.replace_ast(func , new_func );
    })
}

pub(crate) fn code_transformer_intro_ensures(func: ast::Fn) -> Option<ast::Fn> {
    let func = func.clone_for_update();
    let ensures = func.ensures_clause()?; 
    let mut ensures_clauses = ensures.comma_and_conds();

    let stmt_list = func.body()?.stmt_list()?;
    
    match stmt_list.tail_expr() {
        // REVIEW: it is assumed that ret_type is "named" if this function is returning something
        Some(ret_expr) => {
            let ret_name = func.ret_type()?.pat()?.clone();
            let ret_id = 
                match ret_name {
                    ast::Pat::IdentPat(ref id) => id.clone(),
                    _ => return None,
                };

            let let_stmt = syntax::ast::Stmt::LetStmt(let_stmt(ret_name, None, Some(ret_expr.clone())));
            stmt_list.push_back(let_stmt.clone_for_update());

            let first_ens = ensures.expr()?;    
            let first_assert = syntax::ast::Stmt::ExprStmt(assert_stmt_from_predicate(first_ens));                
            stmt_list.push_back(first_assert.clone_for_update());
        
            while let Some(ens) = ensures_clauses.next() {
                let ens_without_comma = ens.condition()?;
                let assert_stmt = syntax::ast::Stmt::ExprStmt(assert_stmt_from_predicate(ens_without_comma));
                stmt_list.push_back(assert_stmt.clone_for_update());
            }

            let id_expr = expr_path(ident_path(&format!("{ret_id}").as_str()));
            stmt_list.set_tail_expr(id_expr.clone_for_update());

            return Some(func); 
        }
        None => {
            let first_ens = ensures.expr()?;    
            let first_assert = syntax::ast::Stmt::ExprStmt(assert_stmt_from_predicate(first_ens));
            dbg!(&first_assert);
            stmt_list.push_back(first_assert.clone_for_update());
            while let Some(ens) = ensures_clauses.next() {
                let ens_without_comma = ens.condition()?;
                let assert_stmt = syntax::ast::Stmt::ExprStmt(assert_stmt_from_predicate(ens_without_comma));
                stmt_list.push_back(assert_stmt.clone_for_update());
            }
            return Some(func);
        }
    }
}



#[cfg(test)]
mod tests {
    use crate::tests::check_assist;

    use super::*;

    #[test]
    fn intro_ensure_easy() {
        cov_mark::check!(cursor_in_ret_position);
        check_assist(
            intro_ensures,
            r#"
proof fn my_proof_fun(x: int, y: int)
    requires
        x < 100,
        y < 100,
    ens$0ures
        x + y < 200,
        x + y < 400,
{       
    assert(x + y < 600);
}
"#,
            r#"
            
proof fn my_proof_fun(x: int, y: int)
    requires
        x < 100,
        y < 100,
    ensures
        x + y < 200,
        x + y < 400,
{
    assert(x + y < 600);

    assert(x + y < 200);
    assert(x + y < 400);
}
"#,
        );
    }

    #[test]
    fn intro_ensure_ret_arg() {
        cov_mark::check!(cursor_in_ret_position);
        check_assist(
            intro_ensures,
            r#"
proof fn my_proof_fun(x: int, y: int) -> (sum: int)
    requires
        x < 100,
        y < 100,
    ens$0ures
        sum < 200,
{
    x + y
}
"#,
            r#"
            
proof fn my_proof_fun(x: int, y: int) -> (sum: int)
    requires
        x < 100,
        y < 100,
    ensures
        sum < 200,
{
    let sum = x + y;
    assert(sum<200);
    sum
}
"#,
        );
    }

}
