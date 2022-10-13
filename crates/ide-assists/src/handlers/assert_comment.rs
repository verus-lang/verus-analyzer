use ide_db::base_db::SourceDatabaseExt;
// use ide_db::syntax_helpers::node_ext::is_pattern_cond;
use syntax::{
    ast::{self, AstNode, make::expr_assert_by},
    T,
};
use std::process::Command;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;


use crate::{
    assist_context::{AssistContext, Assists},
    // utils::invert_boolean_expression,
    AssistId, AssistKind,
};

// Assist: invert_if
//

pub(crate) fn assert_comment(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {
    dbg!("assert_comment");
    let assert_keyword = ctx.find_token_syntax_at_offset(T![assert])?;
    let mut temp_text_string = String::new();

    for par in assert_keyword.parent_ancestors() {
        dbg!(&par.text());
        temp_text_string = String::from(par.text());
    }
    dbg!(&temp_text_string);


    // TODO: instead of writing to a file, consider
    // 1) dev/shm 
    // OR
    // 2) man memfd_create 
    let path = Path::new("/tmp/testing_verus_action.rs");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    // Write the `LOREM_IPSUM` string to `file`, returns `io::Result<()>`
    match file.write_all(temp_text_string.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => dbg!("successfully wrote to {}", display),
    };

    let verus_exec_path = "/Users/chanhee/Works/secure-foundations/verus/source/verus-log.sh";

    let output = Command::new(verus_exec_path)
    .arg(path)
    .output().ok()?;

    dbg!(&output);

    let expr = ast::AssertExpr::cast(assert_keyword.parent()?)?;
    
    
    let assert_range = assert_keyword.text_range();
    let cursor_in_range = assert_range.contains_range(ctx.selection_trimmed());

    if !cursor_in_range {
        return None;
    }

    if output.status.success() {
        dbg!("success");
        dbg!("continue code action");
        // TODO should comment out whole "assertExpr" not only one line
        acc.add(
            AssistId("assert_comment", AssistKind::RefactorRewrite),
            "Confirm if assert necessary",
            assert_range,
            |builder| {
                builder.insert(assert_keyword.text_range().start(), &format!("// "));
            },
        )
    } else {
        // TODO should comment out whole "assertExpr" not only one line
        let assert_stmt = expr.syntax().parent()?;
        acc.add(
            AssistId("assert_comment", AssistKind::RefactorRewrite),
            "Confirm if assert necessary",
            assert_range,
            |builder| {
                builder.insert(assert_stmt.text_range().end(), &format!(" // OBSERVE"));
            },
        )       
    }




}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{check_assist, check_assist_not_applicable};


//     #[test]
//     fn assert_comment_success() {
//         check_assist(
//             assert_comment,
// r#"
// #[allow(unused_imports)]
// use builtin_macros::*;
// #[allow(unused_imports)]
// use builtin::*;

// mod pervasive;
// #[allow(unused_imports)]
// use crate::pervasive::{modes::*, seq::*, vec::*};

// #[verifier(external)]
// fn main() {
// }

// verus! {
//     proof fn proof_index(a: u16, offset: u16)
//     requires    
//         offset < 16
//     ensures
//         offset < 16
//     {
//         ass$0ert(offset < 16);
//     }
// } // verus!
// "#,

// r#"
// #[allow(unused_imports)]
// use builtin_macros::*;
// #[allow(unused_imports)]
// use builtin::*;

// mod pervasive;
// #[allow(unused_imports)]
// use crate::pervasive::{modes::*, seq::*, vec::*};

// #[verifier(external)]
// fn main() {
// }

// verus! {
//     proof fn proof_index(a: u16, offset: u16)
//     requires    
//         offset < 16
//     ensures
//         offset < 16
//     {
//         // assert(offset < 16);
//     }
// } // verus!
// "#,
//         );
//     }







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
        offset < 1000
    {
        ass$0ert(offset < 10);
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
        offset < 1000
    {
        assert(offset < 10); // OBSERVE
    }
} // verus!
"#,
        );
    }


}
