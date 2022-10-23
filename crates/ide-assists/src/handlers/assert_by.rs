// use ide_db::syntax_helpers::node_ext::is_pattern_cond;
use syntax::{
    ast::{self, AstNode, make::{block_expr_from_predicates, ext::empty_block_expr}},
    T, ted,
};
use syntax::ast::make::assert_stmt_from_predicate;
use crate::{
    assist_context::{AssistContext, Assists},
    // utils::invert_boolean_expression,
    AssistId, AssistKind,
};
use syntax::ast::edit::IndentLevel;

pub(crate) fn assert_by(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    let assert_keyword = ctx.find_token_syntax_at_offset(T![assert])?;
    let expr = ast::AssertExpr::cast(assert_keyword.parent()?)?;
    let assert_range = assert_keyword.text_range();
    let cursor_in_range = assert_range.contains_range(ctx.selection_trimmed());
    if !cursor_in_range {
        return None;
    }

    let assert_by = code_transformer_assert_to_assert_by(expr.clone())?;
    acc.add(AssistId("assert_by", AssistKind::RefactorRewrite), "Assert by", assert_range, |edit| {
        edit.replace_ast(syntax::ast::Expr::AssertExpr(expr), syntax::ast::Expr::AssertExpr(assert_by));
    })
}


pub(crate) fn code_transformer_assert_to_assert_by(assert: ast::AssertExpr) -> Option<ast::AssertExpr> {
    if assert.by_token().is_some() {
        return None;
    }
    let indent_level = IndentLevel::from_node(assert.syntax());
    let assert = assert.clone_for_update();
    assert.make_by_keyword();
    let proof_block = empty_block_expr().clone_for_update();
    let stmt_list = proof_block.stmt_list()?;
    let expr = assert.expr()?;    
    let assert_stmt = assert_stmt_from_predicate(expr).clone_for_update();
    ted::insert(ted::Position::first_child_of(assert_stmt.syntax()), ast::make::tokens::whitespace(&format!("\n{}", indent_level+1)));
    let assert_stmt = syntax::ast::Stmt::ExprStmt(assert_stmt);
    stmt_list.push_back(assert_stmt);
    ted::insert(ted::Position::before(stmt_list.r_curly_token()?), ast::make::tokens::whitespace(&format!("\n{}", indent_level)));
    assert.register_proof_block(proof_block);
    Some(assert) 
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::check_assist;

    #[test]
    fn assert_by_composite_condition() {
        check_assist(
            assert_by,
            "
proof fn f() { 
    ass$0ert(x == 3); 
}
            ",

            "
proof fn f() { 
    assert(x == 3) by {
        assert(x == 3);
    } 
}
            ",
        )
    }

}
