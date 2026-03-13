use ide_db::{syntax_helpers::node_ext::vis_eq, FxHashSet};
use syntax::{
    ast::{self, AstNode, AstToken},
    match_ast, Direction, NodeOrToken, SourceFile,
    SyntaxKind::{self, *},
    TextRange, TextSize,
};

use std::hash::Hash;

const REGION_START: &str = "// region:";
const REGION_END: &str = "// endregion";

#[derive(Debug, PartialEq, Eq)]
pub enum FoldKind {
    Comment,
    Imports,
    Mods,
    Block,
    ArgList,
    Region,
    Consts,
    Statics,
    Array,
    WhereClause,
    ReturnType,
    MatchArm,
    ProofBlock,
}

#[derive(Debug)]
pub struct Fold {
    pub range: TextRange,
    pub kind: FoldKind,
}

// Feature: Folding
//
// Defines folding regions for curly braced blocks, runs of consecutive use, mod, const or static
// items, and `region` / `endregion` comment markers.
pub(crate) fn folding_ranges(file: &SourceFile) -> Vec<Fold> {
    let mut res = vec![];
    let mut visited_comments = FxHashSet::default();
    let mut visited_imports = FxHashSet::default();
    let mut visited_mods = FxHashSet::default();
    let mut visited_consts = FxHashSet::default();
    let mut visited_statics = FxHashSet::default();

    // regions can be nested, here is a LIFO buffer
    let mut region_starts: Vec<TextSize> = vec![];

    for element in file.syntax().descendants_with_tokens() {
        // Fold items that span multiple lines
        if let Some(mut kind) = fold_kind(element.kind()) {
            let is_multiline = match &element {
                NodeOrToken::Node(node) => {
                    if kind == FoldKind::Block && is_proof_block(node) {
                        kind = FoldKind::ProofBlock;
                    }
                    node.text().contains_char('\n')
                }
                NodeOrToken::Token(token) => token.text().contains('\n'),
            };
            if is_multiline {
                let range = if kind == FoldKind::ProofBlock {
                    let l_curly = element
                        .as_node()
                        .and_then(|n| n.children().find(|it| it.kind() == STMT_LIST))
                        .and_then(|stmt_list| {
                            stmt_list.children_with_tokens().find(|it| it.kind() == L_CURLY)
                        });

                    if let Some(NodeOrToken::Token(l_curly)) = l_curly {
                        TextRange::new(l_curly.text_range().start(), element.text_range().end())
                    } else {
                        element.text_range()
                    }
                } else {
                    element.text_range()
                };
                res.push(Fold { range, kind });
                continue;
            }
        }

        match element {
            NodeOrToken::Token(token) => {
                // Fold groups of comments
                if let Some(comment) = ast::Comment::cast(token) {
                    if visited_comments.contains(&comment) {
                        continue;
                    }
                    let text = comment.text().trim_start();
                    if text.starts_with(REGION_START) {
                        region_starts.push(comment.syntax().text_range().start());
                    } else if text.starts_with(REGION_END) {
                        if let Some(region) = region_starts.pop() {
                            res.push(Fold {
                                range: TextRange::new(region, comment.syntax().text_range().end()),
                                kind: FoldKind::Region,
                            })
                        }
                    } else if let Some(range) =
                        contiguous_range_for_comment(comment, &mut visited_comments)
                    {
                        res.push(Fold { range, kind: FoldKind::Comment })
                    }
                }
            }
            NodeOrToken::Node(node) => {
                match_ast! {
                    match node {
                        ast::Module(module) => {
                            if module.item_list().is_none() {
                                if let Some(range) = contiguous_range_for_item_group(
                                    module,
                                    &mut visited_mods,
                                ) {
                                    res.push(Fold { range, kind: FoldKind::Mods })
                                }
                            }
                        },
                        ast::Use(use_) => {
                            if let Some(range) = contiguous_range_for_item_group(use_, &mut visited_imports) {
                                res.push(Fold { range, kind: FoldKind::Imports })
                            }
                        },
                        ast::Const(konst) => {
                            if let Some(range) = contiguous_range_for_item_group(konst, &mut visited_consts) {
                                res.push(Fold { range, kind: FoldKind::Consts })
                            }
                        },
                        ast::Static(statik) => {
                            if let Some(range) = contiguous_range_for_item_group(statik, &mut visited_statics) {
                                res.push(Fold { range, kind: FoldKind::Statics })
                            }
                        },
                        ast::WhereClause(where_clause) => {
                            if let Some(range) = fold_range_for_where_clause(where_clause) {
                                res.push(Fold { range, kind: FoldKind::WhereClause })
                            }
                        },
                        ast::MatchArm(match_arm) => {
                            if let Some(range) = fold_range_for_multiline_match_arm(match_arm) {
                                res.push(Fold {range, kind: FoldKind::MatchArm})
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
    }

    res
}

fn fold_kind(kind: SyntaxKind) -> Option<FoldKind> {
    match kind {
        COMMENT => Some(FoldKind::Comment),
        ARG_LIST | PARAM_LIST => Some(FoldKind::ArgList),
        ARRAY_EXPR => Some(FoldKind::Array),
        RET_TYPE => Some(FoldKind::ReturnType),
        ASSOC_ITEM_LIST
        | RECORD_FIELD_LIST
        | RECORD_PAT_FIELD_LIST
        | RECORD_EXPR_FIELD_LIST
        | ITEM_LIST
        | EXTERN_ITEM_LIST
        | USE_TREE_LIST
        | BLOCK_EXPR
        | MATCH_ARM_LIST
        | VARIANT_LIST
        | TOKEN_TREE => Some(FoldKind::Block),
        _ => None,
    }
}

fn contiguous_range_for_item_group<N>(first: N, visited: &mut FxHashSet<N>) -> Option<TextRange>
where
    N: ast::HasVisibility + Clone + Hash + Eq,
{
    if !visited.insert(first.clone()) {
        return None;
    }

    let (mut last, mut last_vis) = (first.clone(), first.visibility());
    for element in first.syntax().siblings_with_tokens(Direction::Next) {
        let node = match element {
            NodeOrToken::Token(token) => {
                if let Some(ws) = ast::Whitespace::cast(token) {
                    if !ws.spans_multiple_lines() {
                        // Ignore whitespace without blank lines
                        continue;
                    }
                }
                // There is a blank line or another token, which means that the
                // group ends here
                break;
            }
            NodeOrToken::Node(node) => node,
        };

        if let Some(next) = N::cast(node) {
            let next_vis = next.visibility();
            if eq_visibility(next_vis.clone(), last_vis) {
                visited.insert(next.clone());
                last_vis = next_vis;
                last = next;
                continue;
            }
        }
        // Stop if we find an item of a different kind or with a different visibility.
        break;
    }

    if first != last {
        Some(TextRange::new(first.syntax().text_range().start(), last.syntax().text_range().end()))
    } else {
        // The group consists of only one element, therefore it cannot be folded
        None
    }
}

fn eq_visibility(vis0: Option<ast::Visibility>, vis1: Option<ast::Visibility>) -> bool {
    match (vis0, vis1) {
        (None, None) => true,
        (Some(vis0), Some(vis1)) => vis_eq(&vis0, &vis1),
        _ => false,
    }
}

fn contiguous_range_for_comment(
    first: ast::Comment,
    visited: &mut FxHashSet<ast::Comment>,
) -> Option<TextRange> {
    visited.insert(first.clone());

    // Only fold comments of the same flavor
    let group_kind = first.kind();
    if !group_kind.shape.is_line() {
        return None;
    }

    let mut last = first.clone();
    for element in first.syntax().siblings_with_tokens(Direction::Next) {
        match element {
            NodeOrToken::Token(token) => {
                if let Some(ws) = ast::Whitespace::cast(token.clone()) {
                    if !ws.spans_multiple_lines() {
                        // Ignore whitespace without blank lines
                        continue;
                    }
                }
                if let Some(c) = ast::Comment::cast(token) {
                    if c.kind() == group_kind {
                        let text = c.text().trim_start();
                        // regions are not real comments
                        if !(text.starts_with(REGION_START) || text.starts_with(REGION_END)) {
                            visited.insert(c.clone());
                            last = c;
                            continue;
                        }
                    }
                }
                // The comment group ends because either:
                // * An element of a different kind was reached
                // * A comment of a different flavor was reached
                break;
            }
            NodeOrToken::Node(_) => break,
        };
    }

    if first != last {
        Some(TextRange::new(first.syntax().text_range().start(), last.syntax().text_range().end()))
    } else {
        // The group consists of only one element, therefore it cannot be folded
        None
    }
}

fn is_proof_block(node: &syntax::SyntaxNode) -> bool {
    match node.kind() {
        BLOCK_EXPR => {
            // proof { ... } — FN_MODE is a direct child of BLOCK_EXPR
            if node.children().any(|child| {
                child.kind() == FN_MODE
                    && child.children_with_tokens().any(|it| it.kind() == PROOF_KW)
            }) {
                return true;
            }
            if let Some(parent) = node.parent() {
                // proof! { ... } — parent is PREFIX_EXPR containing FN_MODE(PROOF_KW) and BANG
                if parent.kind() == PREFIX_EXPR {
                    let has_proof_mode = parent.children().any(|child| {
                        child.kind() == FN_MODE
                            && child.children_with_tokens().any(|it| it.kind() == PROOF_KW)
                    });
                    let has_bang = parent.children_with_tokens().any(|it| it.kind() == BANG);
                    return has_proof_mode && has_bang;
                }
                // assert(...) by { ... } — parent is ASSERT_EXPR with a BY_KW sibling
                if parent.kind() == ASSERT_EXPR {
                    return parent
                        .children_with_tokens()
                        .any(|it| it.kind() == BY_KW);
                }
                // proof fn foo() { ... } — parent is FN with FN_MODE(PROOF_KW)
                if parent.kind() == FN {
                    return parent.children().any(|child| {
                        child.kind() == FN_MODE
                            && child.children_with_tokens().any(|it| it.kind() == PROOF_KW)
                    });
                }
            }
            false
        }
        TOKEN_TREE => {
            if let Some(parent) = node.parent() {
                // proof_decl! { ... } — parent MACRO_CALL whose path text is "proof_decl"
                if parent.kind() == MACRO_CALL {
                    return parent.children().any(|child| {
                        child.kind() == PATH && child.text().to_string().trim() == "proof_decl"
                    });
                }
                // ghost name => { ... } inside atomic_with_ghost!:
                // TOKEN_TREE starting with L_CURLY whose preceding siblings in a TOKEN_TREE
                // contain a GHOST_KW (pattern: ghost <ident> => { ... })
                if parent.kind() == TOKEN_TREE {
                    let starts_with_curly = node
                        .children_with_tokens()
                        .next()
                        .map_or(false, |it| it.kind() == L_CURLY);
                    if starts_with_curly {
                        let mut found_ghost = false;
                        let mut sib = node.prev_sibling_or_token();
                        while let Some(s) = sib {
                            if s.kind() == GHOST_KW {
                                found_ghost = true;
                                break;
                            }
                            sib = s.prev_sibling_or_token();
                        }
                        return found_ghost;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn fold_range_for_where_clause(where_clause: ast::WhereClause) -> Option<TextRange> {
    let first_where_pred = where_clause.predicates().next();
    let last_where_pred = where_clause.predicates().last();

    if first_where_pred != last_where_pred {
        let start = where_clause.where_token()?.text_range().end();
        let end = where_clause.syntax().text_range().end();
        return Some(TextRange::new(start, end));
    }
    None
}

fn fold_range_for_multiline_match_arm(match_arm: ast::MatchArm) -> Option<TextRange> {
    if fold_kind(match_arm.expr()?.syntax().kind()).is_some() {
        None
    } else if match_arm.expr()?.syntax().text().contains_char('\n') {
        Some(match_arm.expr()?.syntax().text_range())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use test_utils::extract_tags;

    use super::*;

    fn check(ra_fixture: &str) {
        let (ranges, text) = extract_tags(ra_fixture, "fold");

        let parse = SourceFile::parse(&text, span::Edition::CURRENT);
        let mut folds = folding_ranges(&parse.tree());
        folds.sort_by_key(|fold| (fold.range.start(), fold.range.end()));

        assert_eq!(
            folds.len(),
            ranges.len(),
            "The amount of folds is different than the expected amount"
        );

        for (fold, (range, attr)) in folds.iter().zip(ranges.into_iter()) {
            assert_eq!(fold.range.start(), range.start(), "mismatched start of folding ranges");
            assert_eq!(fold.range.end(), range.end(), "mismatched end of folding ranges");

            let kind = match fold.kind {
                FoldKind::Comment => "comment",
                FoldKind::Imports => "imports",
                FoldKind::Mods => "mods",
                FoldKind::Block => "block",
                FoldKind::ArgList => "arglist",
                FoldKind::Region => "region",
                FoldKind::Consts => "consts",
                FoldKind::Statics => "statics",
                FoldKind::Array => "array",
                FoldKind::WhereClause => "whereclause",
                FoldKind::ReturnType => "returntype",
                FoldKind::MatchArm => "matcharm",
                FoldKind::ProofBlock => "proof_block",
            };
            assert_eq!(kind, &attr.unwrap());
        }
    }

    #[test]
    fn fold_proof_block() {
        check(
            r#"
fn main() <fold block>{
    proof <fold proof_block>{
        assert(true);
    }</fold>
}</fold>
"#,
        );
    }

    #[test]
    fn fold_proof_bang_block() {
        check(
            r#"
fn main() <fold block>{
    proof! <fold proof_block>{
        assert(true);
    }</fold>
}</fold>
"#,
        );
    }

    #[test]
    fn fold_proof_decl_block() {
        check(
            r#"
fn main() <fold block>{
    proof_decl! <fold proof_block>{
        let tracked mut g = 0u32;
    }</fold>
}</fold>
"#,
        );
    }

    #[test]
    fn fold_atomic_with_ghost_block() {
        check(
            r#"
fn main() <fold block>{
    let value = AtomicBool::new(0);
    atomic_with_ghost! <fold block>(
        value => compare_exchange(false, true);
        returning res;
        ghost g => <fold proof_block>{
            assert(true);
        }</fold>
    )</fold>;
}</fold>
"#,
        );
    }

    #[test]
    fn fold_assert_by_block() {
        check(
            r#"
fn main() <fold block>{
    assert(1 > 0) by <fold proof_block>{
        assert(true);
    }</fold>;
}</fold>
"#,
        );
    }

    #[test]
    fn fold_proof_fn_body() {
        check(
            r#"
proof fn helper() 
    requires false, 
    ensures true,
    <fold proof_block>{
    assert(true);
}</fold>
"#,
        );
    }

    #[test]
    fn test_fold_comments() {
        check(
            r#"
<fold comment>// Hello
// this is a multiline
// comment
//</fold>

// But this is not

fn main() <fold block>{
    <fold comment>// We should
    // also
    // fold
    // this one.</fold>
    <fold comment>//! But this one is different
    //! because it has another flavor</fold>
    <fold comment>/* As does this
    multiline comment */</fold>
}</fold>
"#,
        );
    }

    #[test]
    fn test_fold_imports() {
        check(
            r#"
use std::<fold block>{
    str,
    vec,
    io as iop
}</fold>;
"#,
        );
    }

    #[test]
    fn test_fold_mods() {
        check(
            r#"

pub mod foo;
<fold mods>mod after_pub;
mod after_pub_next;</fold>

<fold mods>mod before_pub;
mod before_pub_next;</fold>
pub mod bar;

mod not_folding_single;
pub mod foobar;
pub not_folding_single_next;

<fold mods>#[cfg(test)]
mod with_attribute;
mod with_attribute_next;</fold>

mod inline0 {}
mod inline1 {}

mod inline2 <fold block>{

}</fold>
"#,
        );
    }

    #[test]
    fn test_fold_import_groups() {
        check(
            r#"
<fold imports>use std::str;
use std::vec;
use std::io as iop;</fold>

<fold imports>use std::mem;
use std::f64;</fold>

<fold imports>use std::collections::HashMap;
// Some random comment
use std::collections::VecDeque;</fold>
"#,
        );
    }

    #[test]
    fn test_fold_import_and_groups() {
        check(
            r#"
<fold imports>use std::str;
use std::vec;
use std::io as iop;</fold>

<fold imports>use std::mem;
use std::f64;</fold>

use std::collections::<fold block>{
    HashMap,
    VecDeque,
}</fold>;
// Some random comment
"#,
        );
    }

    #[test]
    fn test_folds_structs() {
        check(
            r#"
struct Foo <fold block>{
}</fold>
"#,
        );
    }

    #[test]
    fn test_folds_traits() {
        check(
            r#"
trait Foo <fold block>{
}</fold>
"#,
        );
    }

    #[test]
    fn test_folds_macros() {
        check(
            r#"
macro_rules! foo <fold block>{
    ($($tt:tt)*) => { $($tt)* }
}</fold>
"#,
        );
    }

    #[test]
    fn test_fold_match_arms() {
        check(
            r#"
fn main() <fold block>{
    match 0 <fold block>{
        0 => 0,
        _ => 1,
    }</fold>
}</fold>
"#,
        );
    }

    #[test]
    fn test_fold_multiline_non_block_match_arm() {
        check(
            r#"
            fn main() <fold block>{
                match foo <fold block>{
                    block => <fold block>{
                    }</fold>,
                    matcharm => <fold matcharm>some.
                        call().
                        chain()</fold>,
                    matcharm2
                        => 0,
                    match_expr => <fold matcharm>match foo2 <fold block>{
                        bar => (),
                    }</fold></fold>,
                    array_list => <fold array>[
                        1,
                        2,
                        3,
                    ]</fold>,
                    structS => <fold matcharm>StructS <fold block>{
                        a: 31,
                    }</fold></fold>,
                }</fold>
            }</fold>
            "#,
        )
    }

    #[test]
    fn fold_big_calls() {
        check(
            r#"
fn main() <fold block>{
    frobnicate<fold arglist>(
        1,
        2,
        3,
    )</fold>
}</fold>
"#,
        )
    }

    #[test]
    fn fold_record_literals() {
        check(
            r#"
const _: S = S <fold block>{

}</fold>;
"#,
        )
    }

    #[test]
    fn fold_multiline_params() {
        check(
            r#"
fn foo<fold arglist>(
    x: i32,
    y: String,
)</fold> {}
"#,
        )
    }

    #[test]
    fn fold_multiline_array() {
        check(
            r#"
const FOO: [usize; 4] = <fold array>[
    1,
    2,
    3,
    4,
]</fold>;
"#,
        )
    }

    #[test]
    fn fold_region() {
        check(
            r#"
// 1. some normal comment
<fold region>// region: test
// 2. some normal comment
<fold region>// region: inner
fn f() {}
// endregion</fold>
fn f2() {}
// endregion: test</fold>
"#,
        )
    }

    #[test]
    fn fold_consecutive_const() {
        check(
            r#"
<fold consts>const FIRST_CONST: &str = "first";
const SECOND_CONST: &str = "second";</fold>
"#,
        )
    }

    #[test]
    fn fold_consecutive_static() {
        check(
            r#"
<fold statics>static FIRST_STATIC: &str = "first";
static SECOND_STATIC: &str = "second";</fold>
"#,
        )
    }

    #[test]
    fn fold_where_clause() {
        // fold multi-line and don't fold single line.
        check(
            r#"
fn foo()
where<fold whereclause>
    A: Foo,
    B: Foo,
    C: Foo,
    D: Foo,</fold> {}

fn bar()
where
    A: Bar, {}
"#,
        )
    }

    #[test]
    fn fold_return_type() {
        check(
            r#"
fn foo()<fold returntype>-> (
    bool,
    bool,
)</fold> { (true, true) }

fn bar() -> (bool, bool) { (true, true) }
"#,
        )
    }
}
