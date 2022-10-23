use syntax::{ast::{self, make::{assert_stmt_from_predicate, let_stmt, ext::ident_path, expr_path}}, AstNode};
// use syntax::{ast, match_ast, AstNode, SyntaxKind, SyntaxToken, TextRange, TextSize};
use syntax::ast::edit::IndentLevel;
use crate::{AssistContext, AssistId, AssistKind, Assists};
use syntax::ted;

pub(crate) fn intro_ensures(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    dbg!("intro_ensures");
    let func = ctx.find_node_at_offset::<ast::Fn>()?;
    let ensures = func.ensures_clause()?; 
    let ensures_keyword = ensures.ensures_token()?;
    let ensures_range = ensures_keyword.text_range();
    let cursor_in_range = ensures_range.contains_range(ctx.selection_trimmed());
    if !cursor_in_range {
        return None;
    }
    dbg!("intro_ensures calculate diff");
    let new_func = code_transformer_intro_ensures(func.clone())?;
    dbg!("intro_ensures register");
    acc.add(AssistId("intro_ensures", AssistKind::RefactorRewrite), "Copy ensures to the end", ensures_range, |edit| {
        edit.replace_ast(func , new_func );
    })
}

pub(crate) fn code_transformer_intro_ensures(func: ast::Fn) -> Option<ast::Fn> {
    let func = func.clone_for_update();
    let ensures = func.ensures_clause()?; 
    let mut ensures_clauses = ensures.comma_and_conds();

    let stmt_list = func.body()?.stmt_list()?;
    let indent_level = IndentLevel::from_node(stmt_list.syntax()) + 1;

    dbg!("code diff for intro_ensures");
    
    match stmt_list.tail_expr() { // TODO: match on ret arg instead
        // REVIEW: it is assumed that ret_type is "named" if this function is returning something
        Some(ret_expr) => {
            dbg!("code diff for intro_ensures ret arg");
            let ret_name = func.ret_type()?.pat()?.clone();
            let ret_id = 
                match ret_name {
                    ast::Pat::IdentPat(ref id) => id.clone(),
                    _ => return None,
                };
            let let_stmt = let_stmt(ret_name, None, Some(ret_expr.clone())).clone_for_update();
            // ted::insert(ted::Position::after(let_stmt.semicolon_token()?), ast::make::tokens::single_newline());
            let let_stmt = syntax::ast::Stmt::LetStmt(let_stmt);
            stmt_list.push_back(let_stmt);

            let first_ens = ensures.expr()?;    
            let first_assert_stmt = assert_stmt_from_predicate(first_ens).clone_for_update();
            ted::insert(ted::Position::first_child_of(first_assert_stmt.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));
            let first_assert = syntax::ast::Stmt::ExprStmt(first_assert_stmt);
            stmt_list.push_back(first_assert);
        
            while let Some(ens) = ensures_clauses.next() {
                let ens_without_comma = ens.condition()?;
                let assert_stmt_without_indent = assert_stmt_from_predicate(ens_without_comma).clone_for_update();
                ted::insert(ted::Position::first_child_of(assert_stmt_without_indent.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));
                let assert_stmt = syntax::ast::Stmt::ExprStmt(assert_stmt_without_indent);
                stmt_list.push_back(assert_stmt);
            }

            let id_expr = expr_path(ident_path(&format!("{ret_id}").as_str()));
            stmt_list.set_tail_expr(id_expr.clone_for_update());
            ted::insert(ted::Position::before(stmt_list.tail_expr()?.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));

            // let stmt_list = stmt_list.indent(indent_level);
            // for ancestor in  stmt_list.syntax().ancestors() {
            //     match ancestor.kind() {
            //         SyntaxKind::FN => {
            //             let func = ast::Fn::cast(ancestor)?;
            //             return Some(func);
            //         }
            //         _ => (),
            //     }
            // }

            return Some(func); 
        }
        None => {
            dbg!("code diff for intro_ensures no ret arg");
            let first_ens = ensures.expr()?;    
            let first_assert_stmt = assert_stmt_from_predicate(first_ens).clone_for_update();
            ted::insert(ted::Position::first_child_of(first_assert_stmt.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));
            let first_assert = syntax::ast::Stmt::ExprStmt(first_assert_stmt);
            stmt_list.push_back(first_assert);
        
            while let Some(ens) = ensures_clauses.next() {
                let ens_without_comma = ens.condition()?;
                let assert_stmt_without_indent = assert_stmt_from_predicate(ens_without_comma).clone_for_update();
                ted::insert(ted::Position::first_child_of(assert_stmt_without_indent.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));
                let assert_stmt = syntax::ast::Stmt::ExprStmt(assert_stmt_without_indent);
                stmt_list.push_back(assert_stmt);
            }
            ted::insert(ted::Position::before(stmt_list.r_curly_token()?), ast::make::tokens::single_newline());
 

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
        check_assist(
            intro_ensures,
            r#"
proof fn my_proof_fun(x: int, y: int) -> (sum: int)
    requires
        x < 100,
        y < 100,
    ens$0ures
        sum < 200,
        sum < 300,
        sum < 400,
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
        sum < 300,
        sum < 400,
{
    let sum = x + y; 
    assert(sum < 200); 
    assert(sum < 300); 
    assert(sum < 400); 
    sum
}
"#,
        );
    }


    #[test]
    fn intro_ensure_fibo() {
        check_assist(
            intro_ensures,
            r#"
proof fn lemma_fibo_is_monotonic(i: nat, j: nat)
    requires
        i <= j,
    e$0nsures
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
            r#"
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
    assert(fibo(i) <= fibo(j));
}
"#,
        );
    }









}
