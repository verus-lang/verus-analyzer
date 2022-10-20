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
    // dbg!("assert_by");
    let assert_keyword = ctx.find_token_syntax_at_offset(T![assert])?;
    let expr = ast::AssertExpr::cast(assert_keyword.parent()?)?;
    // dbg!(&expr);
    
    let assert_range = assert_keyword.text_range();
    let cursor_in_range = assert_range.contains_range(ctx.selection_trimmed());
    // TODO: make sure that 'by' does not exist.
    // apply this `assert_by` only for "assert(P);"
    if !cursor_in_range {
        return None;
    }



    let assert_by = code_transformer_assert_to_assert_by(expr.clone())?;
    dbg!("let's add acc");
    acc.add(AssistId("assert_by", AssistKind::RefactorRewrite), "Assert by", assert_range, |edit| {
        // let assert_inner = expr.clone().expr().unwrap(); // TODO: unwrap
        // dbg!(&assert_inner);
        // let assert_by = expr_assert_by(assert_inner);
        // dbg!(&assert_by);
        edit.replace_ast(syntax::ast::Expr::AssertExpr(expr), syntax::ast::Expr::AssertExpr(assert_by));

        // let else_node = else_block.syntax();
        // let else_range = else_node.text_range();
        // let then_range = then_node.text_range();

        // edit.replace(else_range, then_node.text());
        // edit.replace(then_range, else_node.text());
    })
}


pub(crate) fn code_transformer_assert_to_assert_by(assert: ast::AssertExpr) -> Option<ast::AssertExpr> {
    if assert.by_token().is_some() {
        return None;
    }
    let mut assert = assert.clone_for_update();
    assert.make_by_keyword();
    dbg!("made by");
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
