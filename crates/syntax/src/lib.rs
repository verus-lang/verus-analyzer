//! Syntax Tree library used throughout the rust-analyzer.
//!
//! Properties:
//!   - easy and fast incremental re-parsing
//!   - graceful handling of errors
//!   - full-fidelity representation (*any* text can be precisely represented as
//!     a syntax tree)
//!
//! For more information, see the [RFC]. Current implementation is inspired by
//! the [Swift] one.
//!
//! The most interesting modules here are `syntax_node` (which defines concrete
//! syntax tree) and `ast` (which defines abstract syntax tree on top of the
//! CST). The actual parser live in a separate `parser` crate, though the
//! lexer lives in this crate.
//!
//! See `api_walkthrough` test in this file for a quick API tour!
//!
//! [RFC]: <https://github.com/rust-lang/rfcs/pull/2256>
//! [Swift]: <https://github.com/apple/swift/blob/13d593df6f359d0cb2fc81cfaac273297c539455/lib/Syntax/README.md>

#![warn(rust_2018_idioms, unused_lifetimes, semicolon_in_expressions_from_macros)]

#[allow(unused)]
macro_rules! eprintln {
    ($($tt:tt)*) => { stdx::eprintln!($($tt)*) };
}

mod syntax_node;
mod syntax_error;
mod parsing;
mod validation;
mod ptr;
mod token_text;
#[cfg(test)]
mod tests;

pub mod algo;
pub mod ast;
#[doc(hidden)]
pub mod fuzz;
pub mod utils;
pub mod ted;
pub mod hacks;

use std::{marker::PhantomData, sync::Arc};

use stdx::format_to;
use text_edit::Indel;

pub use crate::{
    ast::{AstNode, AstToken},
    ptr::{AstPtr, SyntaxNodePtr},
    syntax_error::SyntaxError,
    syntax_node::{
        PreorderWithTokens, RustLanguage, SyntaxElement, SyntaxElementChildren, SyntaxNode,
        SyntaxNodeChildren, SyntaxToken, SyntaxTreeBuilder,
    },
    token_text::TokenText,
};
pub use parser::{SyntaxKind, T};
pub use rowan::{
    api::Preorder, Direction, GreenNode, NodeOrToken, SyntaxText, TextRange, TextSize,
    TokenAtOffset, WalkEvent,
};
pub use smol_str::SmolStr;

/// `Parse` is the result of the parsing: a syntax tree and a collection of
/// errors.
///
/// Note that we always produce a syntax tree, even for completely invalid
/// files.
#[derive(Debug, PartialEq, Eq)]
pub struct Parse<T> {
    green: GreenNode,
    errors: Arc<Vec<SyntaxError>>,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Parse<T> {
    fn clone(&self) -> Parse<T> {
        Parse { green: self.green.clone(), errors: self.errors.clone(), _ty: PhantomData }
    }
}

impl<T> Parse<T> {
    fn new(green: GreenNode, errors: Vec<SyntaxError>) -> Parse<T> {
        Parse { green, errors: Arc::new(errors), _ty: PhantomData }
    }

    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
    pub fn errors(&self) -> &[SyntaxError] {
        &*self.errors
    }
}

impl<T: AstNode> Parse<T> {
    pub fn to_syntax(self) -> Parse<SyntaxNode> {
        Parse { green: self.green, errors: self.errors, _ty: PhantomData }
    }

    pub fn tree(&self) -> T {
        T::cast(self.syntax_node()).unwrap()
    }

    pub fn ok(self) -> Result<T, Arc<Vec<SyntaxError>>> {
        if self.errors.is_empty() {
            Ok(self.tree())
        } else {
            Err(self.errors)
        }
    }
}

impl Parse<SyntaxNode> {
    pub fn cast<N: AstNode>(self) -> Option<Parse<N>> {
        if N::cast(self.syntax_node()).is_some() {
            Some(Parse { green: self.green, errors: self.errors, _ty: PhantomData })
        } else {
            None
        }
    }
}

impl Parse<SourceFile> {
    pub fn debug_dump(&self) -> String {
        let mut buf = format!("{:#?}", self.tree().syntax());
        for err in self.errors.iter() {
            format_to!(buf, "error {:?}: {}\n", err.range(), err);
        }
        buf
    }

    pub fn reparse(&self, indel: &Indel) -> Parse<SourceFile> {
        self.incremental_reparse(indel).unwrap_or_else(|| self.full_reparse(indel))
    }

    fn incremental_reparse(&self, indel: &Indel) -> Option<Parse<SourceFile>> {
        // FIXME: validation errors are not handled here
        parsing::incremental_reparse(self.tree().syntax(), indel, self.errors.to_vec()).map(
            |(green_node, errors, _reparsed_range)| Parse {
                green: green_node,
                errors: Arc::new(errors),
                _ty: PhantomData,
            },
        )
    }

    fn full_reparse(&self, indel: &Indel) -> Parse<SourceFile> {
        let mut text = self.tree().syntax().text().to_string();
        indel.apply(&mut text);
        SourceFile::parse(&text)
    }
}

/// `SourceFile` represents a parse tree for a single Rust file.
pub use crate::ast::SourceFile;

impl SourceFile {
    pub fn parse(text: &str) -> Parse<SourceFile> {
        let (green, mut errors) = parsing::parse_text(text);
        let root = SyntaxNode::new_root(green.clone());

        errors.extend(validation::validate(&root));

        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
        Parse { green, errors: Arc::new(errors), _ty: PhantomData }
    }
}

/// Matches a `SyntaxNode` against an `ast` type.
///
/// # Example:
///
/// ```ignore
/// match_ast! {
///     match node {
///         ast::CallExpr(it) => { ... },
///         ast::MethodCallExpr(it) => { ... },
///         ast::MacroCall(it) => { ... },
///         _ => None,
///     }
/// }
/// ```
#[macro_export]
macro_rules! match_ast {
    (match $node:ident { $($tt:tt)* }) => { match_ast!(match ($node) { $($tt)* }) };

    (match ($node:expr) {
        $( $( $path:ident )::+ ($it:pat) => $res:expr, )*
        _ => $catch_all:expr $(,)?
    }) => {{
        $( if let Some($it) = $($path::)+cast($node.clone()) { $res } else )*
        { $catch_all }
    }};
}

/// This test does not assert anything and instead just shows off the crate's
/// API.
#[test]
fn api_walkthrough() {
    use ast::{HasModuleItem, HasName};

    let source_code = "
        fn foo() {
            1 + 1
        }
    ";
    // `SourceFile` is the main entry point.
    //
    // The `parse` method returns a `Parse` -- a pair of syntax tree and a list
    // of errors. That is, syntax tree is constructed even in presence of errors.
    let parse = SourceFile::parse(source_code);
    assert!(parse.errors().is_empty());

    // The `tree` method returns an owned syntax node of type `SourceFile`.
    // Owned nodes are cheap: inside, they are `Rc` handles to the underling data.
    let file: SourceFile = parse.tree();

    // `SourceFile` is the root of the syntax tree. We can iterate file's items.
    // Let's fetch the `foo` function.
    let mut func = None;
    for item in file.items() {
        match item {
            ast::Item::Fn(f) => func = Some(f),
            _ => unreachable!(),
        }
    }
    let func: ast::Fn = func.unwrap();

    // Each AST node has a bunch of getters for children. All getters return
    // `Option`s though, to account for incomplete code. Some getters are common
    // for several kinds of node. In this case, a trait like `ast::NameOwner`
    // usually exists. By convention, all ast types should be used with `ast::`
    // qualifier.
    let name: Option<ast::Name> = func.name();
    let name = name.unwrap();
    assert_eq!(name.text(), "foo");

    // Let's get the `1 + 1` expression!
    let body: ast::BlockExpr = func.body().unwrap();
    let stmt_list: ast::StmtList = body.stmt_list().unwrap();
    let expr: ast::Expr = stmt_list.tail_expr().unwrap();

    // Enums are used to group related ast nodes together, and can be used for
    // matching. However, because there are no public fields, it's possible to
    // match only the top level enum: that is the price we pay for increased API
    // flexibility
    let bin_expr: &ast::BinExpr = match &expr {
        ast::Expr::BinExpr(e) => e,
        _ => unreachable!(),
    };

    // Besides the "typed" AST API, there's an untyped CST one as well.
    // To switch from AST to CST, call `.syntax()` method:
    let expr_syntax: &SyntaxNode = expr.syntax();

    // Note how `expr` and `bin_expr` are in fact the same node underneath:
    assert!(expr_syntax == bin_expr.syntax());

    // To go from CST to AST, `AstNode::cast` function is used:
    let _expr: ast::Expr = match ast::Expr::cast(expr_syntax.clone()) {
        Some(e) => e,
        None => unreachable!(),
    };

    // The two properties each syntax node has is a `SyntaxKind`:
    assert_eq!(expr_syntax.kind(), SyntaxKind::BIN_EXPR);

    // And text range:
    assert_eq!(expr_syntax.text_range(), TextRange::new(32.into(), 37.into()));

    // You can get node's text as a `SyntaxText` object, which will traverse the
    // tree collecting token's text:
    let text: SyntaxText = expr_syntax.text();
    assert_eq!(text.to_string(), "1 + 1");

    // There's a bunch of traversal methods on `SyntaxNode`:
    assert_eq!(expr_syntax.parent().as_ref(), Some(stmt_list.syntax()));
    assert_eq!(stmt_list.syntax().first_child_or_token().map(|it| it.kind()), Some(T!['{']));
    assert_eq!(
        expr_syntax.next_sibling_or_token().map(|it| it.kind()),
        Some(SyntaxKind::WHITESPACE)
    );

    // As well as some iterator helpers:
    let f = expr_syntax.ancestors().find_map(ast::Fn::cast);
    assert_eq!(f, Some(func));
    assert!(expr_syntax.siblings_with_tokens(Direction::Next).any(|it| it.kind() == T!['}']));
    assert_eq!(
        expr_syntax.descendants_with_tokens().count(),
        8, // 5 tokens `1`, ` `, `+`, ` `, `!`
           // 2 child literal expressions: `1`, `1`
           // 1 the node itself: `1 + 1`
    );

    // There's also a `preorder` method with a more fine-grained iteration control:
    let mut buf = String::new();
    let mut indent = 0;
    for event in expr_syntax.preorder_with_tokens() {
        match event {
            WalkEvent::Enter(node) => {
                let text = match &node {
                    NodeOrToken::Node(it) => it.text().to_string(),
                    NodeOrToken::Token(it) => it.text().to_string(),
                };
                format_to!(buf, "{:indent$}{:?} {:?}\n", " ", text, node.kind(), indent = indent);
                indent += 2;
            }
            WalkEvent::Leave(_) => indent -= 2,
        }
    }
    assert_eq!(indent, 0);
    assert_eq!(
        buf.trim(),
        r#"
"1 + 1" BIN_EXPR
  "1" LITERAL
    "1" INT_NUMBER
  " " WHITESPACE
  "+" PLUS
  " " WHITESPACE
  "1" LITERAL
    "1" INT_NUMBER
"#
        .trim()
    );

    // To recursively process the tree, there are three approaches:
    // 1. explicitly call getter methods on AST nodes.
    // 2. use descendants and `AstNode::cast`.
    // 3. use descendants and `match_ast!`.
    //
    // Here's how the first one looks like:
    let exprs_cast: Vec<String> = file
        .syntax()
        .descendants()
        .filter_map(ast::Expr::cast)
        .map(|expr| expr.syntax().text().to_string())
        .collect();

    // An alternative is to use a macro.
    let mut exprs_visit = Vec::new();
    for node in file.syntax().descendants() {
        match_ast! {
            match node {
                ast::Expr(it) => {
                    let res = it.syntax().text().to_string();
                    exprs_visit.push(res);
                },
                _ => (),
            }
        }
    }
    assert_eq!(exprs_cast, exprs_visit);
}


#[test]
fn verus_walkthrough1() {
    use ast::{HasModuleItem, HasName};

    let source_code = 
    "verus!{
        proof fn my_proof_fun(x: int, y: int)
            requires
                x < 100,
                y < 100,
            ensures
                x + y < 200,
            {
                assert(x + y < 200);
            }
    }";
    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();

    // dbg!(&file);
    for item in file.items() {
        dbg!(&item);
        // match item {
        //     ast::Item::Fn(f) => func = Some(f),
        //     _ => unreachable!(),
        // }
    }
}


#[test]
fn verus_walkthrough2() {
    use ast::{HasModuleItem, HasName};

    let source_code = 
    "verus!{
        proof fn my_proof_fun(x: int, y: int) -> (sum: int)
            requires
                x < 100,
                y < 100,
            ensures
                sum < 200,
        {
            x + y
        }    

        spec fn my_spec_fun(x: int, y: int) -> int
            recommends
                x < 100,
                y < 100,
        {
            x + y
        }
        pub(crate) open spec fn my_pub_spec_fun3(x: int, y: int) -> int {
            // function and body visible to crate
            x / 2 + y / 2
        }
        pub closed spec fn my_pub_spec_fun4(x: int, y: int) -> int {
            // function visible to all, body visible to module
            x / 2 + y / 2
        }
        pub(crate) closed spec fn my_pub_spec_fun5(x: int, y: int) -> int {
            // function visible to crate, body visible to module
            x / 2 + y / 2
        }
    }";

    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();
    dbg!(&file);
    for item in file.items() {
        dbg!(&item);
    }

}


#[test]
fn verus_walkthrough3() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
        proof fn test5_bound_checking(x: u32, y: u32, z: u32)
            requires
                x <= 0xffff,
                y <= 0xffff,
                z <= 0xffff,
        {
            assert(x * z == mul(x, z)) by(nonlinear_arith)
                requires
                    x <= 0xffff,
                    z <= 0xffff,
            {
                assert(0 <= x * z);
                assert(x * z <= 0xffff * 0xffff);
            }
            assert(0 <= y < 100 ==> my_spec_fun(x, y) >= x);
            assert(forall|x: int, y: int| 0 <= x < 100 && 0 <= y < 100 ==> my_spec_fun(x, y) >= x);
        }
        fn test_quantifier() {
            assert(forall|x: int, y: int| 0 <= x < 100 && 0 <= y < 100 ==> my_spec_fun(x, y) >= x);
            assert(my_spec_fun(10, 20) == 30);
            assert(exists|x: int, y: int| my_spec_fun(x, y) == 30);
        }
    }";
    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();
    for item in file.items() {
        dbg!(&item);
    }
}


#[test]
fn verus_walkthrough4() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
        fn test_assert_forall_by() {
            assert forall|x: int, y: int| f1(x) + f1(y) == x + y + 2 by {
                reveal(f1);
            }
            assert(f1(1) + f1(2) == 5);
            assert(f1(3) + f1(4) == 9);

            // to prove forall|...| P ==> Q, write assert forall|...| P implies Q by {...}
            assert forall|x: int| x < 10 implies f1(x) < 11 by {
                assert(x < 10);
                reveal(f1);
                assert(f1(x) < 11);
            }
            assert(f1(3) < 11);
        }
        fn test_choose() {
            assume(exists|x: int| f1(x) == 10);
            proof {
                let x_witness = choose|x: int| f1(x) == 10;
                assert(f1(x_witness) == 10);
            }
        
            assume(exists|x: int, y: int| f1(x) + f1(y) == 30);
            proof {
                let (x_witness, y_witness): (int, int) = choose|x: int, y: int| f1(x) + f1(y) == 30;
                assert(f1(x_witness) + f1(y_witness) == 30);
            }
        }        
    }";
    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();
    dbg!(&file);
    for item in file.items() {
        dbg!(&item);
    }
}


#[test]
fn verus_walkthrough5() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
        fn test_single_trigger1() {
            assume(forall|x: int, y: int| f1(x) < 100 && f1(y) < 100 ==> #[trigger] my_spec_fun(x, y) >= x);
        }
    }";
    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();
    dbg!(&file);
    for item in file.items() {
        dbg!(&item);
    }
}


// TODO
// maybe I will get back to "full" parsing of Verus syntax
#[test]
fn verus_walkthrough6() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
        fn test_single_trigger2() {
            // Use [f1(x), f1(y)] as the trigger
            assume(forall|x: int, y: int| #[trigger] f1(x) < 100 && #[trigger] f1(y) < 100 ==> my_spec_fun(x, y) >= x);
        }
        /// To manually specify multiple triggers, use #![trigger]:
        fn test_multiple_triggers() {
            // Use both [my_spec_fun(x, y)] and [f1(x), f1(y)] as triggers
            assume(forall|x: int, y: int|
                #![trigger my_spec_fun(x, y)]
                #![trigger f1(x), f1(y)]
                f1(x) < 100 && f1(y) < 100 ==> my_spec_fun(x, y) >= x
            );
        }
    }";
    let parse = SourceFile::parse(source_code);
    dbg!(&parse.errors);
    assert!(parse.errors().is_empty());
    let file: SourceFile = parse.tree();
    dbg!(&file);
    for item in file.items() {
        dbg!(&item);
    }
}
