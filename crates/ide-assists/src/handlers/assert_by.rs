// use ide_db::syntax_helpers::node_ext::is_pattern_cond;
use syntax::{
    ast::{self, AstNode, make::expr_assert_by},
    T,
};

use crate::{
    assist_context::{AssistContext, Assists},
    // utils::invert_boolean_expression,
    AssistId, AssistKind,
};

// Assist: invert_if
//
// This transforms if expressions of the form `if !x {A} else {B}` into `if x {B} else {A}`
// This also works with `!=`. This assist can only be applied with the cursor on `if`.
//
// ```
// fn main() {
//     if$0 !y { A } else { B }
// }
// ```
// ->
// ```
// fn main() {
//     if y { B } else { A }
// }
// assert(x == 3);
//
// assert(x == 3) by {
//    assert(x == 3);
//}
// ```
pub(crate) fn assert_by(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    dbg!("assert_by");
    let assert_keyword = ctx.find_token_syntax_at_offset(T![assert])?;
    let expr = ast::AssertExpr::cast(assert_keyword.parent()?)?;
    dbg!(&expr);
    
    let assert_range = assert_keyword.text_range();
    let cursor_in_range = assert_range.contains_range(ctx.selection_trimmed());
    // TODO: make sure that 'by' does not exist.
    // apply this `assert_by` only for "assert(P);"
    if !cursor_in_range {
        return None;
    }

    dbg!("let's add acc");
    acc.add(AssistId("assert_by", AssistKind::RefactorRewrite), "Assert by", assert_range, |edit| {
        // let flip_expr = invert_boolean_expression(syntax::ast::Expr::AssertExpr(expr.clone()));
        let assert_inner = expr.clone().expr().unwrap(); // TODO: unwrap
        dbg!(&assert_inner);
        let assert_by = expr_assert_by(assert_inner);
        dbg!(&assert_by);

        // let's do assert_by
        // let if_cond = invert_boolean_expression(while_cond);
        // let if_expr = make::expr_if(if_cond, break_block, None);
        // let stmts = once(make::expr_stmt(if_expr).into()).chain(while_body.statements());
        // make::block_expr(stmts, while_body.tail_expr())






        edit.replace_ast(syntax::ast::Expr::AssertExpr(expr), assert_by);

        // let else_node = else_block.syntax();
        // let else_range = else_node.text_range();
        // let then_range = then_node.text_range();

        // edit.replace(else_range, then_node.text());
        // edit.replace(then_range, else_node.text());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{check_assist, check_assist_not_applicable};

    #[test]
    fn assert_by_composite_condition() {
        check_assist(
            assert_by,
            "fn f() { ass$0ert(x == 3); }",
            "fn f() { assert(x == 3) by {assert(x == 3);} }",
        )
    }

}
