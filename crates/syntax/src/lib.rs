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
        
        fn foo(x:int) -> int {
            if x>0 {1} else {-1}
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
fn verus_walkthrough6() {
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

        spec fn sum2(i: int, j: int) -> int
            recommends
                0 <= i < 10,
                0 <= j < 10,
        {
            i + j
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
fn verus_walkthrough7() {
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


#[test]
fn verus_walkthrough8() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
    fn test_my_funs2(
        a: u32, // exec variable
        b: u32, // exec variable
    )
        requires
            a < 100,
            b < 100,
    {
        let s = a + b; // s is an exec variable
        proof {
            let u = a + b; // u is a ghost variable
            my_proof_fun(u / 2, b as int); // my_proof_fun(x, y) takes ghost parameters x and y
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
fn verus_walkthrough9() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
    proof fn test_tracked(
        tracked w: int,
        tracked x: int,
        tracked y: int,
        z: int,
      ) -> tracked TrackedAndGhost<(int, int), int> {
       
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
fn verus_walkthrough10() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
    fn test_views() {
        let mut v: Vec<u8> = Vec::new();
        v.push(10);
        v.push(20);
        proof {
            let s: Seq<u8> = v@; // v@ is equivalent to v.view()
            assert(s[0] == 10);
            assert(s[1] == 20);
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
fn verus_walkthrough11() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
fn binary_search(v: &Vec<u64>, k: u64) -> (r: usize)
    requires
        forall|i:int, j:int| 0 <= i <= j < v.len() ==> v[i] <= v[j],
        exists|i:int| 0 <= i < v.len() && k == v[i],
    ensures
        r < v.len(),
        k == v[r as int],
{
    let mut i1: usize = 0;
    let mut i2: usize = v.len() - 1;
    while i1 != i2
        invariant
            i2 < v.len(),
            exists|i:int| i1 <= i <= i2 && k == v[i],
            forall|i:int, j:int| 0 <= i <= j < v.len() ==> v[i] <= v[j],
    {
        //let d: Ghost<int> = ghost(i2 - i1);

        let ix = i1 + (i2 - i1) / 2;
        if *v.index(ix) < k {
            i1 = ix + 1;
        } else {
            i2 = ix;
        }

        assert(i2 - i1 < d@);
    }
    i1
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
fn verus_walkthrough12() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
fn pop_test(t: Vec<u64>)
requires
    t.len() > 0,
    forall|i: int| #![auto] 0 <= i < t.len() ==> uninterp_fn(t[i]),
{
let mut t = t;
let x = t.pop();

assert(uninterp_fn(x));
assert(forall|i: int| #![auto] 0 <= i < t.len() ==> uninterp_fn(t[i]));
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
fn verus_walkthrough13() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{

    proof fn arith_sum_int_nonneg(i: nat)
        ensures
            arith_sum_int(i as int) >= 0,
        decreases
            i,
    {
        if i > 0 {
            arith_sum_int_nonneg((i - 1) as nat);
        }
    }
    



    spec fn arith_sum_int(i: int) -> int
    decreases i
{
    if i <= 0 { 0 } else { i + arith_sum_int(i - 1) }
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
fn verus_walkthrough14() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{
fn exec_with_decreases(n: u64) -> u64
    decreases 100 - n,
{
    if n < 100 {
        exec_with_decreases(n + 1)
    } else {
        n
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
fn verus_walkthrough15() {
    use ast::{HasModuleItem, HasName};
    let source_code = 
    "verus!{

spec(checked) fn my_spec_fun2(x: int, y: int) -> int
    recommends
        x < 100,
        y < 100,
{
    // Because of spec(checked), Verus checks that my_spec_fun's recommends clauses are satisfied here:
    my_spec_fun(x, y)
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
// "verus! {

//     /// functions may be declared exec (default), proof, or spec, which contain
//     /// exec code, proof code, and spec code, respectively.
//     ///   - exec code: compiled, may have requires/ensures
//     ///   - proof code: erased before compilation, may have requires/ensures
//     ///   - spec code: erased before compilation, no requires/ensures, but may have recommends
//     /// exec and proof functions may name their return values inside parentheses, before the return type
//     fn my_exec_fun(x: u32, y: u32) -> (sum: u32)
//         requires
//             x < 100,
//             y < 100,
//         ensures
//             sum < 200,
//     {
//         x + y
//     }
    
//     proof fn my_proof_fun(x: int, y: int) -> (sum: int)
//         requires
//             x < 100,
//             y < 100,
//         ensures
//             sum < 200,
//     {
//         x + y
//     }
    
//     spec fn my_spec_fun(x: int, y: int) -> int
//         recommends
//             x < 100,
//             y < 100,
//     {
//         x + y
//     }
    
//     /// exec code cannot directly call proof functions or spec functions.
//     /// However, exec code can contain proof blocks (proof { ... }),
//     /// which contain proof code.
//     /// This proof code can call proof functions and spec functions.
//     fn test_my_funs(x: u32, y: u32)
//         requires
//             x < 100,
//             y < 100,
//     {
//         // my_proof_fun(x, y); // not allowed in exec code
//         // let u = my_spec_fun(x, y); // not allowed exec code
//         proof {
//             let u = my_spec_fun(x as int, y as int); // allowed in proof code
//             my_proof_fun(u / 2, y as int); // allowed in proof code
//         }
//     }
    
//     /// spec functions with pub or pub(...) must specify whether the body of the function
//     /// should also be made publicly visible (open function) or not visible (closed function).
//     pub open spec fn my_pub_spec_fun1(x: int, y: int) -> int {
//         // function and body visible to all
//         x / 2 + y / 2
//     }
//     /* TODO
//     pub open(crate) spec fn my_pub_spec_fun2(x: u32, y: u32) -> u32 {
//         // function visible to all, body visible to crate
//         x / 2 + y / 2
//     }
//     */
//     pub(crate) open spec fn my_pub_spec_fun3(x: int, y: int) -> int {
//         // function and body visible to crate
//         x / 2 + y / 2
//     }
//     pub closed spec fn my_pub_spec_fun4(x: int, y: int) -> int {
//         // function visible to all, body visible to module
//         x / 2 + y / 2
//     }
//     pub(crate) closed spec fn my_pub_spec_fun5(x: int, y: int) -> int {
//         // function visible to crate, body visible to module
//         x / 2 + y / 2
//     }
    
//     /// Recursive functions must have decreases clauses so that Verus can verify that the functions
//     /// terminate.
//     fn test_rec(x: u64, y: u64)
//         requires
//             0 < x < 100,
//             y < 100 - x,
//         decreases x
//     {
//         if x > 1 {
//             test_rec(x - 1, y + 1);
//         }
//     }
    
//     /// Multiple decreases clauses are ordered lexicographically, so that later clauses may
//     /// increase when earlier clauses decrease.
//     spec fn test_rec2(x: int, y: int) -> int
//         decreases x, y
//     {
//         if y > 0 {
//             1 + test_rec2(x, y - 1)
//         } else if x > 0 {
//             2 + test_rec2(x - 1, 100)
//         } else {
//             3
//         }
//     }
 //     /// variables may be exec, tracked, or ghost
//     ///   - exec: compiled
//     ///   - tracked: erased before compilation, checked for lifetimes (advanced feature, discussed later)
//     ///   - ghost: erased before compilation, no lifetime checking, can create default value of any type
//     /// Different variable modes may be used in different code modes:
//     ///   - variables in exec code are always exec
//     ///   - variables in proof code are ghost by default (tracked variables must be marked "tracked")
//     ///   - variables in spec code are always ghost
//     /// For example:
//     fn test_my_funs2(
//         a: u32, // exec variable
//         b: u32, // exec variable
//     )
//         requires
//             a < 100,
//             b < 100,
//     {
//         let s = a + b; // s is an exec variable
//         proof {
//             let u = a + b; // u is a ghost variable
//             my_proof_fun(u / 2, b as int); // my_proof_fun(x, y) takes ghost parameters x and y
//         }
//     }
    
//     /// assume and assert are treated as proof code even outside of proof blocks.
//     /// "assert by" may be used to provide proof code that proves the assertion.
//     #[verifier(opaque)]
//     spec fn f1(i: int) -> int {
//         i + 1
//     }
    
//     fn assert_by_test() {
//         assert(f1(3) > 3) by {
//             reveal(f1); // reveal f1's definition just inside this block
//         }
//         assert(f1(3) > 3);
//     }
    
//     /// "assert by" can also invoke specialized provers for bit-vector reasoning or nonlinear arithmetic.
//     fn assert_by_provers(x: u32) {
//         assert(x ^ x == 0u32) by(bit_vector);
//         assert(2 <= x && x < 10 ==> x * x > x) by(nonlinear_arith);
//     }
    
//     /// "assert by" can use nonlinear_arith with proof code,
//     /// where "requires" clauses selectively make facts available to the proof code.
//     proof fn test5_bound_checking(x: u32, y: u32, z: u32)
//         requires
//             x <= 0xffff,
//             y <= 0xffff,
//             z <= 0xffff,
//     {
//         assert(x * z == mul(x, z)) by(nonlinear_arith)
//             requires
//                 x <= 0xffff,
//                 z <= 0xffff,
//         {
//             assert(0 <= x * z);
//             assert(x * z <= 0xffff * 0xffff);
//         }
//     }
    
//     /// The syntax for forall and exists quantifiers is based on closures:
//     fn test_quantifier() {
//         assert(forall|x: int, y: int| 0 <= x < 100 && 0 <= y < 100 ==> my_spec_fun(x, y) >= x);
//         assert(my_spec_fun(10, 20) == 30);
//         assert(exists|x: int, y: int| my_spec_fun(x, y) == 30);
//     }
    
//     /// "assert forall by" may be used to prove foralls:
//     fn test_assert_forall_by() {
//         assert forall|x: int, y: int| f1(x) + f1(y) == x + y + 2 by {
//             reveal(f1);
//         }
//         assert(f1(1) + f1(2) == 5);
//         assert(f1(3) + f1(4) == 9);
    
//         // to prove forall|...| P ==> Q, write assert forall|...| P implies Q by {...}
//         assert forall|x: int| x < 10 implies f1(x) < 11 by {
//             assert(x < 10);
//             reveal(f1);
//             assert(f1(x) < 11);
//         }
//         assert(f1(3) < 11);
//     }
    
//     /// To extract ghost witness values from exists, use choose:
//     fn test_choose() {
//         assume(exists|x: int| f1(x) == 10);
//         proof {
//             let x_witness = choose|x: int| f1(x) == 10;
//             assert(f1(x_witness) == 10);
//         }
    
//         assume(exists|x: int, y: int| f1(x) + f1(y) == 30);
//         proof {
//             let (x_witness, y_witness): (int, int) = choose|x: int, y: int| f1(x) + f1(y) == 30;
//             assert(f1(x_witness) + f1(y_witness) == 30);
//         }
//     }
    
//     /// To manually specify a trigger to use for the SMT solver to match on when instantiating a forall
//     /// or proving an exists, use #[trigger]:
//     fn test_single_trigger1() {
//         // Use [my_spec_fun(x, y)] as the trigger
//         assume(forall|x: int, y: int| f1(x) < 100 && f1(y) < 100 ==> #[trigger] my_spec_fun(x, y) >= x);
//     }
//     fn test_single_trigger2() {
//         // Use [f1(x), f1(y)] as the trigger
//         assume(forall|x: int, y: int|
//             #[trigger] f1(x) < 100 && #[trigger] f1(y) < 100 ==> my_spec_fun(x, y) >= x
//         );
//     }
    
//     /// To manually specify multiple triggers, use #![trigger]:
//     fn test_multiple_triggers() {
//         // Use both [my_spec_fun(x, y)] and [f1(x), f1(y)] as triggers
//         assume(forall|x: int, y: int|
//             #![trigger my_spec_fun(x, y)]
//             #![trigger f1(x), f1(y)]
//             f1(x) < 100 && f1(y) < 100 ==> my_spec_fun(x, y) >= x
//         );
//     }
    
//     /// Verus can often automatically choose a trigger if no manual trigger is given.
//     /// Use the command-line option --triggers to print the chosen triggers.
//     fn test_auto_trigger1() {
//         // Verus automatically chose [my_spec_fun(x, y)] as the trigger.
//         // (It considers this safer, i.e. likely to match less often, than the trigger [f1(x), f1(y)].)
//         assume(forall|x: int, y: int| f1(x) < 100 && f1(y) < 100 ==> my_spec_fun(x, y) >= x);
//     }
    
//     /// If Verus prints a note saying that it automatically chose a trigger with low confidence,
//     /// you can supply manual triggers or use #![auto] to accept the automatically chosen trigger.
//     fn test_auto_trigger2() {
//         // Verus chose [f1(x), f1(y)] as the trigger; go ahead and accept that
//         assume(forall|x: int, y: int| #![auto] f1(x) < 100 && f1(y) < 100 ==> my_spec_fun(3, y) >= 3);
//     }
    
//     /// &&& and ||| are like && and ||, but have low precedence (lower than all other binary operators).
//     /// &&& must appear before each conjunct, rather than between the conjuncts (similarly for |||).
//     spec fn simple_conjuncts(x: int, y: int) -> bool {
//         &&& 1 < x
//         &&& y > 9 ==> x + y < 50
//         &&& x < 100
//         &&& y < 100
//     }
//     spec fn complex_conjuncts(x: int, y: int) -> bool {
//         let b = x < y;
//         &&& b
//         &&& if false {
//                 &&& b ==> b
//                 &&& !b ==> !b
//             } else {
//                 ||| b ==> b
//                 ||| !b
//             }
//         &&& false ==> true
//     }
    
//     /// ==> associates to the right, while <== associates to the left.
//     /// <==> is nonassociative.
//     /// === is SMT equality (equivalent to the builtin equal function).
//     /// !== is SMT disequality.
//     pub(crate) proof fn binary_ops<A>(a: A, x: int) {
//         assert(false ==> true);
//         assert(true && false ==> false && false);
//         assert(!(true && (false ==> false) && false));
    
//         assert(false ==> false ==> false);
//         assert(false ==> (false ==> false));
//         assert(!((false ==> false) ==> false));
    
//         assert(false <== false <== false);
//         assert(!(false <== (false <== false)));
//         assert((false <== false) <== false);
//         assert(2 + 2 !== 3);
//         assert(a === a);
    
//         assert(false <==> true && false);
//     }
    
//     /// In specs, <=, <, >=, and > may be chained together so that, for example, a <= b < c means
//     /// a <= b && b < c.  (Note on efficiency: if b is a complex expression,
//     /// Verus will automatically introduce a temporary variable under the hood so that
//     /// the expression doesn't duplicate b: {let x_b = b; a <= x_b && x_b < c}.)
//     proof fn chained_comparisons(i: int, j: int, k: int)
//         requires
//             0 <= i + 1 <= j + 10 < k + 7,
//         ensures
//             j < k,
//     {
//     }
    
//     /// In specs, e@ is an abbreviation for e.view()
//     /// Many types implement a view() method to get an abstract ghost view of a concrete type.
//     fn test_views() {
//         let mut v: Vec<u8> = Vec::new();
//         v.push(10);
//         v.push(20);
//         proof {
//             let s: Seq<u8> = v@; // v@ is equivalent to v.view()
//             assert(s[0] == 10);
//             assert(s[1] == 20);
//         }
//     }
    
//     /// struct and enum declarations may be declared exec (default), tracked, or ghost,
//     /// and fields may be declared exec (default), tracked or ghost.
//     tracked struct TrackedAndGhost<T, G>(
//         tracked T,
//         ghost G,
//     );
    
//     /// Proof code may manipulate tracked variables directly.
//     /// Both declarations and uses of tracked variables must be explicitly marked as "tracked".
//     proof fn consume(tracked x: int) {
//     }
    
//     proof fn test_tracked(
//         tracked w: int,
//         tracked x: int,
//         tracked y: int,
//         z: int
//     ) -> tracked TrackedAndGhost<(int, int), int> {
//         consume(tracked w);
//         let tracked tag: TrackedAndGhost<(int, int), int> = TrackedAndGhost((tracked x, tracked y), z);
//         let tracked TrackedAndGhost((a, b), c) = tracked tag;
//         TrackedAndGhost((tracked a, tracked b), c)
//     }
    
//     /// Variables in exec code are always exec; ghost and tracked variables are not available directly.
//     /// Instead, the library types Ghost and Tracked are used to wrap ghost values and tracked values.
//     /// Ghost and tracked expressions ghost(expr) and tracked(expr) create values of type Ghost<T>
//     /// and Tracked<T>, where expr is treated as proof code whose value is wrapped inside Ghost or Tracked.
//     /// The view x@ of a Ghost or Tracked x is the ghost or tracked value inside the Ghost or Tracked.
//     fn test_ghost(x: u32, y: u32)
//         requires
//             x < 100,
//             y < 100,
//     {
//         let u: Ghost<int> = ghost(my_spec_fun(x as int, y as int));
//         let mut v: Ghost<int> = ghost(u@ + 1);
//         assert(v@ == x + y + 1);
//         proof {
//             v@ = v@ + 1; // proof code may assign to the view of exec variables of type Ghost/Tracked
//         }
//         let w: Ghost<int> = ghost({
//             // proof block that returns a ghost value
//             let temp = v@ + 1;
//             temp + 1
//         });
//         assert(w@ == x + y + 4);
//     }
    
//     fn test_consume(t: Tracked<int>)
//         requires t@ <= 7
//     {
//         proof {
//             let tracked x = (tracked t).get();
//             assert(x <= 7);
//             consume(tracked x);
//         }
//     }
    
//     /// Exec code can extract individual Ghost and Tracked values from Ghost and Tracked tuples
//     /// with "let Ghost((...))" and "let Tracked((...))".
//     /// The tuple pattern elements may further match on Trk and Gho from pervasives::modes.
//     fn test_ghost_tuple_match(t: Tracked<(bool, bool, Gho<int>, Gho<int>)>) -> Tracked<bool> {
//         let g: Ghost<(int, int)> = ghost((10, 20));
    
//         let Ghost((g1, g2)) = g; // g1 and g2 both have type Ghost<int>
//         assert(g1@ + g2@ == 30);
    
//         let Ghost((g1, g2)): (Ghost<int>, Ghost<int>) = g;
//         assert(g1@ + g2@ == 30);
    
//         let Tracked((b1, b2, Gho(g3), Gho(g4))) = t; // b1, b2: Tracked<bool> and g3, g4: Ghost<int>
//         b2
//     }
    
//     /// Spec functions are not checked for correctness (although they are checked for termination).
//     /// However, marking a spec function as "spec(checked)" enables lightweight "recommends checking"
//     /// inside the spec function.
//     spec(checked) fn my_spec_fun2(x: int, y: int) -> int
//         recommends
//             x < 100,
//             y < 100,
//     {
//         // Because of spec(checked), Verus checks that my_spec_fun's recommends clauses are satisfied here:
//         my_spec_fun(x, y)
//     }
    
//     /// Spec functions may omit their body, in which case they are considered
//     /// uninterpreted (returning an arbitrary value of the return type depending on the input values).
//     /// This is safe, since spec functions (unlike proof and exec functions) may always
//     /// return arbitrary values of any type,
//     /// where the value may be special "bottom" value for otherwise uninhabited types.
//     spec fn my_uninterpreted_fun1(i: int, j: int) -> int;
    
//     spec fn my_uninterpreted_fun2(i: int, j: int) -> int
//         recommends
//             0 <= i < 10,
//             0 <= j < 10;
    
//     /// Trait functions may have specifications
//     trait T {
//         proof fn my_uninterpreted_fun2(&self, i: int, j: int) -> (r: int)
//             requires
//                 0 <= i < 10,
//                 0 <= j < 10,
//             ensures
//                 i <= r,
//                 j <= r;
//     }
    
//     } // verus!"