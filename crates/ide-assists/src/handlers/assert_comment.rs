// use ide_db::syntax_helpers::node_ext::is_pattern_cond;
use syntax::{
    ast::{self, AstNode},
    T, SyntaxToken, SyntaxKind,
};
use std::{process::Command, hash::{Hash, Hasher}};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::time::{Instant};
use crate::{
    assist_context::{AssistContext, Assists},
    // utils::invert_boolean_expression,
    AssistId, AssistKind,
};
use std::collections::hash_map::DefaultHasher;


pub(crate) fn assert_comment(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    let assert_keyword = ctx.find_token_syntax_at_offset(T![assert])?;
    let assert_range = assert_keyword.text_range();
    let cursor_in_range = assert_range.contains_range(ctx.selection_trimmed());
    if !cursor_in_range {
        return None;
    }
    let func = ctx.find_node_at_offset::<ast::Fn>()?;
    let assert_expr = ast::AssertExpr::cast(assert_keyword.parent()?)?;
    let assert_stmt = ast::Stmt::ExprStmt(ast::ExprStmt::cast(assert_expr.syntax().parent()?)?);
    let assert_removed_fn = code_transformer_remove_expr_stmt(func, assert_stmt.clone())?;

    if run_verus_for_ast(assert_removed_fn.fn_token()?)? {
        dbg!("still success");
        // TODO: comment out using // rather than /* */
        acc.add(
            AssistId("assert_comment", AssistKind::RefactorRewrite),
            "Confirm if assert necessary",
            assert_range,
            |builder| {
                builder.insert(assert_stmt.syntax().text_range().start(), &format!("/* "));
                builder.insert(assert_stmt.syntax().text_range().end(), &format!(" */"));
            },
        )
    } else {
        dbg!("verification failed without this asseriton");
        acc.add(
            AssistId("assert_comment", AssistKind::RefactorRewrite),
            "Confirm if assert necessary",
            assert_range,
            |builder| {
                builder.insert(assert_stmt.syntax().text_range().end(), &format!(" // OBSERVE"));
            },
        )       
    }


}


// a code action that removes a chosen assertion
pub(crate) fn code_transformer_remove_expr_stmt(func:ast::Fn, assert_stmt: ast::Stmt) -> Option<ast::Fn> {
    let mut func = func;
    let assert_stmt = assert_stmt.clone_for_update();
    for ancestor in  assert_stmt.syntax().ancestors() {
        match ancestor.kind() {
            SyntaxKind::FN => {
                func = ast::Fn::cast(ancestor)?;
                break;
            }
            _ => (),
        }
    }
    assert_stmt.remove();
    Some(func)
}


// TODO: change output type ---- could give Verus error code
// TODO: get function name, and include "--verify-function" flag
pub(crate) fn run_verus_for_ast(token: SyntaxToken) -> Option<bool> {
    let mut temp_text_string = String::new();
    // get the text of the most grand parent
    for par in token.parent_ancestors() {
        temp_text_string = String::from(par.text());
    }

    // TODO: instead of writing to a file, consider
    // 1) dev/shm 
    // OR
    // 2) man memfd_create
    let mut hasher = DefaultHasher::new();
    let now = Instant::now();
    now.hash(&mut hasher);

    let tmp_name = format!("/Users/chanhee/Works/rust-analyzer/tmp/testing_verus_action_{:?}_.rs", hasher.finish());
    let path = Path::new(&tmp_name);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) =>{dbg!("couldn't create {}: {}", display, why); return None},
        Ok(file) => file,
    };

    // Write the modified verus program to `file`, returns `io::Result<()>`
    match file.write_all(temp_text_string.as_bytes()) {
        Err(why) => {dbg!("couldn't write to {}: {}", display, why); return None},
        Ok(_) => dbg!("successfully wrote to {}", display),
    };

    // TODO - get path from `settings.json` or other source
    let verus_exec_path = "/Users/chanhee/Works/secure-foundations/verus/source/verus-log.sh";
    let output = Command::new(verus_exec_path)
    .arg(path)
    .output().ok()?;
    // TODO: remove this temporary file!
    dbg!(&output);

    if output.status.success() {
        return Some(true);
    } else {
        // disambiguate verification failure     VS    compile error etc
        match std::str::from_utf8(&output.stdout) {
            Ok(out) => {
                if out.contains("verification results:: verified: 0 errors: 0") {
                    // failure from other errors. (e.g. compile error)
                    return None;
                } else {
                    // verification failure
                    return Some(false);
                }
            }
            Err(_) => return None,
        }
    }
}






#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::check_assist;


    #[test]
    fn assert_comment_success() {
        check_assist(
            assert_comment,
r#"
#[allow(unused_imports)]
use builtin_macros::*;
#[allow(unused_imports)]
use builtin::*;

mod pervasive;
#[allow(unused_imports)]
use crate::pervasive::{modes::*, seq::*, vec::*};

#[verifier(external)]
fn main() {
}

verus! {
    proof fn proof_index(a: u16, offset: u16)
    requires    
        offset < 16
    ensures
        offset < 16
    {
        ass$0ert(offset < 16);
    }
} // verus!
"#,

r#"
#[allow(unused_imports)]
use builtin_macros::*;
#[allow(unused_imports)]
use builtin::*;

mod pervasive;
#[allow(unused_imports)]
use crate::pervasive::{modes::*, seq::*, vec::*};

#[verifier(external)]
fn main() {
}

verus! {
    proof fn proof_index(a: u16, offset: u16)
    requires    
        offset < 16
    ensures
        offset < 16
    {
        /* assert(offset < 16); */
    }
} // verus!
"#,
        );
    }







    #[test]
    fn assert_comment_fail() {
        check_assist(
            assert_comment,
r#"
#[allow(unused_imports)]
use builtin_macros::*;
#[allow(unused_imports)]
use builtin::*;

mod pervasive;
#[allow(unused_imports)]
use crate::pervasive::{modes::*, seq::*, vec::*};

#[verifier(external)]
fn main() {
}

verus! {
    proof fn proof_index(a: u16, offset: u16)
    requires    
        offset < 1000
    ensures
        offset & offset < 1000
    {
        ass$0ert(offset & offset == offset) by(bit_vector);
    }
} // verus!
"#,

r#"
#[allow(unused_imports)]
use builtin_macros::*;
#[allow(unused_imports)]
use builtin::*;

mod pervasive;
#[allow(unused_imports)]
use crate::pervasive::{modes::*, seq::*, vec::*};

#[verifier(external)]
fn main() {
}

verus! {
    proof fn proof_index(a: u16, offset: u16)
    requires    
        offset < 1000
    ensures
        offset & offset < 1000
    {
        assert(offset & offset == offset) by(bit_vector); // OBSERVE
    }
} // verus!
"#,
        );
    }


}
