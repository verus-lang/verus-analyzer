// use ide_db::syntax_helpers::node_ext::is_pattern_cond;
use syntax::{
    ast::{self, AstNode, make::{expr_assert_by, block_expr_from_predicates, ext::empty_block_expr}},
    T, ted,
};

use crate::{
    assist_context::{AssistContext, Assists},
    // utils::invert_boolean_expression,
    AssistId, AssistKind,
};


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
    let mut assert = assert.clone_for_update();
    assert.make_by_keyword();
    let vv = vec![assert.expr()?.clone()];
    let mut block = block_expr_from_predicates(&vv).clone_for_update();
    ted::insert(ted::Position::after(assert.by_token().unwrap()), block.syntax());
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
