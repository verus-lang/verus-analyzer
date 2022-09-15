use syntax::{ast, match_ast, AstNode, SyntaxKind, SyntaxToken, TextRange, TextSize};

use crate::{AssistContext, AssistId, AssistKind, Assists};


pub(crate) fn intro_ensures(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    let func = ctx.find_node_at_offset::<ast::Fn>()?;


    let body = func.body()?;
    let stmt_list = body.stmt_list()?;
    let tail_expr = stmt_list.tail_expr()?;
    let r_curly = stmt_list.r_curly_token()?;
    let ensures = func.ensures_clause()?; 
    let ensures_keyword = ensures.ensures_token()?;
    let mut ensures_clauses = ensures.cond_and_commas();

    let ensures_range = ensures_keyword.text_range();
    let cursor_in_range = ensures_range.contains_range(ctx.selection_trimmed());
    if !cursor_in_range {
        return None;
    }

    let mut intro_enss = String::new();
    while let Some(ens) = ensures_clauses.next() {
        let ens_without_comma = ens.condition()?;
        intro_enss = format!("{intro_enss}\n    assert({ens_without_comma});");
    }
    dbg!(&intro_enss);

    //TODO: if there's named return value, should introduce `let binding` before assertion, and also return the value 

    acc.add(
        AssistId("intro_ensures", AssistKind::RefactorRewrite),
        "Intro ensures",
        tail_expr.syntax().text_range(),
        |builder| {
            builder.insert(r_curly.text_range().start(), &format!("{}\n", intro_enss));
            // builder.replace(tail_expr.syntax().text_range(), &format!("{}\n{}", tail_expr, intro_enss));
        },
    )
}


#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn infer_return_type_cursor_at_return_type_pos() {
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

}
