//! This module generates VST datatype used by verus-analyzer.
//!
//! The VST datatype is generated from the ungrammar file.

use itertools::Itertools;
use quote::{format_ident, quote};
use crate::tests::ast_src::{
    AstSrc, KindsSrc, KINDS_SRC,
};

use crate::tests::sourcegen_ast::*;

// From sourcegen_ast::extract_struct_traits
const special_items: &[(&str, &[&str])] = &[
    ("HasAttrs", &["attrs"]),
    ("HasName", &["name"]),
    ("HasVisibility", &["visibility"]),
    ("HasGenericParams", &["generic_param_list", "where_clause"]),
    ("HasTypeBounds", &["type_bound_list", "colon_token"]),
    ("HasModuleItem", &["items"]),
    ("HasLoopBody", &["label", "loop_body"]),
    ("HasArgList", &["arg_list"]),
];

#[test]
fn sourcegen_vst() {
    let grammar =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/rust.ungram")).parse().unwrap();
    let ast = lower(&grammar, true);

    let ast_nodes = generate_vst(KINDS_SRC, &ast);
    let ast_nodes_file = sourcegen::project_root().join("crates/syntax/src/ast/generated/vst_nodes.rs");
    sourcegen::ensure_file_contents(ast_nodes_file.as_path(), &ast_nodes);
}

pub(crate) fn generate_vst(kinds: KindsSrc<'_>, grammar: &AstSrc) -> String {
    // TODO: add "Comment" item

    // generate struct definitions
    let node_defs: Vec<_> = grammar
        .nodes
        .iter()
        .map(|node| {
            let name = format_ident!("{}", node.name);
            let fields = node.fields.iter().map(|field| {
                let name = field.method_name();
                let ty = field.ty();

                if field.is_many() {
                    quote! {
                        pub #name : Vec<#ty>,
                    }
                } else if let Some(token_kind) = field.token_kind() {
                    // hacky for now
                    // maybe special-case identifier to "#name : Option<String>"
                    // 'ident, 'int_number', and 'lifetime_ident'.
                    if token_kind.to_string() == "T ! [ident]"
                        || token_kind.to_string() == "T ! [int_number]"
                        || token_kind.to_string() == "T ! [lifetime_ident]"
                    {
                        quote! {
                            #name : Option<String>,
                        }
                    } else {
                        quote! {
                            #name : bool,
                        }
                    }
                } else {
                    // As source code can be incomplete, we use Option even if the field is not optional in ungrammar.
                    // TODO:
                    // As source code can be incomplete, we use might use `Option` even if the field is not optional in ungrammar.
                    // instead, however, since proof action might choose to be available when syntax is complete
                    // therefore, we do not use `Option` for VST.
                    // we only use `Option` when the syntax item is optional in ungrammar.
                    if field.is_one() {
                        quote! {
                            pub #name : Box<#ty>,
                        }
                    } else {
                        quote! {
                            pub #name : Option<Box<#ty>>,
                        }
                    }
                }
            });

            quote! {
                #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                pub struct #name {
                    #(#fields)*
                }
            }
        })
        .collect_vec();

    // CST -> VST
    // impl From (eventually `TryFrom` to remove all the options around every fields) for each node
    let from_node_to_vnode_struct: Vec<_> = grammar
        .nodes
        .iter()
        .map(|node| {
            let name = format_ident!("{}", node.name);
            let fields = node.fields.iter().map(|field| {
                let name = field.method_name();
                let ty = field.ty();

                if field.is_many() {
                    quote! {
                        #name : item.#name().into_iter().map(#ty::try_from).collect::<Result<Vec<#ty>, String>>()?,
                    }
                } else if let Some(token_kind) = field.token_kind() {
                    // hacky for now
                    // maybe special-case identifier to "#name : Option<String>"
                    // 'ident, 'int_number', and 'lifetime_ident'.
                    if token_kind.to_string() == "T ! [ident]"
                        || token_kind.to_string() == "T ! [int_number]"
                        || token_kind.to_string() == "T ! [lifetime_ident]"
                    {
                        // #name : Option<String>,
                        quote! {
                            #name : item.#name().map(|it| it.text().to_string()),
                        }
                    } else {
                        // #name : bool,
                        quote! {
                            #name : item.#name().is_some(),
                        }
                    }
                } else {
                    if field.is_one() {
                        // pub #name : Box<#ty>,
                        quote! {
                            #name: Box::new(item.#name().ok_or(format!("{}", stringify!(#name))).map(|it| #ty::try_from(it))??),
                        }
                    } else {                    
                        // pub #name : Option<Box<#ty>>,
                        quote! {
                            #name: match item.#name() {
                                Some(it) => Some(Box::new(#ty::try_from(it)?)),
                                None => None,
                            },
                        }
                    }
                }
            });

            quote! {
                impl TryFrom<super::nodes::#name> for #name {
                    type Error = String;
                    fn try_from(item: super::nodes::#name) -> Result<Self, Self::Error>  {
                        Ok(Self {
                            #(#fields)*
                        })
                    }
                }
            }
        })
        .collect_vec();

    // generate enum definitions
    let enum_defs: Vec<_> = grammar
        .enums
        .iter()
        .map(|en| {
            let variants: Vec<_> = en.variants.iter().map(|var| format_ident!("{}", var)).collect();
            let name = format_ident!("{}", en.name);
            let kinds: Vec<_> = variants
                .iter()
                .map(|name| format_ident!("{}", to_upper_snake_case(&name.to_string())))
                .collect();

            let traits = en.traits.iter().map(|trait_name| {
                let trait_name = format_ident!("{}", trait_name);
                quote!(impl ast::#trait_name for #name {})
            });

            quote! {
                #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                pub enum #name {
                    #(#variants(Box<#variants>),)*
                }
            }
        })
        .collect_vec();


    // CST to VST
    let from_node_to_vnode_enum:  Vec<_> = grammar
    .enums
    .iter()
    .map(|en| {
        let variants: Vec<_> = en.variants.iter().map(|var| format_ident!("{}", var)).collect();
        let name = format_ident!("{}", en.name);
        let kinds: Vec<_> = variants
            .iter()
            .map(|name| format_ident!("{}", to_upper_snake_case(&name.to_string())))
            .collect();
        
        let traits = en.traits.iter().map(|trait_name| {
            let trait_name = format_ident!("{}", trait_name);
            quote!(impl ast::#trait_name for #name {})
        });

        quote! {
            impl TryFrom<super::nodes::#name> for #name {
                type Error = String;
                fn try_from(item: super::nodes::#name) -> Result<Self, Self::Error> {
                    match item {
                        #(
                            super::nodes::#name::#variants(it) => Ok(Self::#variants(Box::new(it.try_into()?))),
                        )*
                    }
                }
            }
        }  
    })
    .collect_vec();

    let ast = quote! {
        #![allow(non_snake_case)]
        use crate::{
            SyntaxNode, SyntaxToken, SyntaxKind::{self, *},
            ast::{self, AstNode, AstChildren, support, traits::*},
            T,
        };

        #(#node_defs)*
        #(#enum_defs)*
        #(#from_node_to_vnode_struct)*
        #(#from_node_to_vnode_enum)*
    };

    // TODO: expr_ext
    // this file contains manual `impl`s that are not auto-generated.
    // VST should have all corresponding `impl`s

    // VST -> CST
    // TODO: generate display impls (this is to print VST and parse into CST)
    // #(#display_impls)*

    sourcegen::add_preamble("sourcegen_vst", sourcegen::reformat(ast.to_string()))
}

/*
below stuff are removed in "sourcege_ast" with "remove_field"
through "extract_struct_traits"

("HasAttrs", &["attrs"]),
("HasName", &["name"]),
("HasVisibility", &["visibility"]),
("HasGenericParams", &["generic_param_list", "where_clause"]),
("HasTypeBounds", &["type_bound_list", "colon_token"]),
("HasModuleItem", &["items"]),
("HasLoopBody", &["label", "loop_body"]),
("HasArgList", &["arg_list"]),
 */

/*
impl From<super::nodes::AssertExpr> for AssertExpr {
    fn from(item: super::nodes::AssertExpr) -> Self {
        Self {
            assert_token: item.assert_token().is_some(),
            l_paren_token: item.l_paren_token().is_some(),
            expr: item.expr().map(Expr::from).map(Box::new),
            r_paren_token: item.r_paren_token().is_some(),
            by_token: item.by_token().is_some(),
            requires_clause: item.requires_clause().map(RequiresClause::from).map(Box::new),
            block_expr: item.block_expr().map(BlockExpr::from).map(Box::new),
        }
    }
}

impl From<super::nodes::Name> for Name {
    fn from(item: super::nodes::Name) -> Self {
        Self {
            ident_token: item.ident_token().map(|it| it.text().to_string()),
            self_token: item.self_token().is_some(),
        }
    }
}

impl TryFrom<super::nodes::AssertExpr> for AssertExpr {
    type Error = ();

    fn try_from(item: super::nodes::AssertExpr) -> Result<Self, Self::Error> {
        let res = Self {
            assert_token: item.assert_token().is_some(),
            l_paren_token: item.l_paren_token().is_some(),
            expr: Some(Box::new(item.expr().try_into()?)),
            r_paren_token: item.r_paren_token().is_some(),
            by_token: item.by_token().is_some(),
            requires_clause: item.requires_clause().map(RequiresClause::try_from).map(Box::new),
            block_expr: item.block_expr.map(Box::new),
        };
        Ok(res)
    }
}
 */

/*



impl TryFrom<super::nodes::Attr> for Attr {


    type Error = ();
    fn try_from(item: super::nodes::Attr) -> Result<Self, Self::Error> {
    }
}

impl TryFrom<super::nodes::Expr> for Expr {


    type Error = ();
    fn try_from(item: super::nodes::Expr) -> Result<Self, Self::Error> {
    }
}
impl TryFrom<super::nodes::Name> for Name {


    type Error = ();
    fn try_from(item: super::nodes::Name) -> Result<Self, Self::Error> {
    }
}
impl TryFrom<super::nodes::RequiresClause> for RequiresClause {


    type Error = ();
    fn try_from(item: super::nodes::RequiresClause) -> Result<Self, Self::Error> {
    }
}
// pub struct AssertExpr {
//     pub attrs: Vec<Attr>,
//     assert_token: bool,
//     l_paren_token: bool,
//     pub expr: Box<Expr>,
//     r_paren_token: bool,
//     by_token: bool,
//     pub name: Option<Box<Name>>,
//     pub requires_clause: Option<Box<RequiresClause>>,
//     pub block_expr: Option<Box<BlockExpr>>,
// }
impl TryFrom<super::nodes::AssertExpr> for AssertExpr {
    type Error = ();

    fn try_from(item: super::nodes::AssertExpr) -> Result<Self, Self::Error> {
        let attrs = item.attrs().into_iter().map(Attr::try_from).collect()?;
        let assert_token = item.assert_token().is_some();
        let l_paren_token = item.l_paren_token().is_some();
        let expr = Box::new(item.expr().ok_or(()).map(|it| Expr::try_from(it))??);
        let r_paren_token= item.r_paren_token().is_some();
        let by_token = item.by_token().is_some();
        let name = match item.name() {
            Some(it) => Some(Box::new(Name::try_from(it)?)),
            None => None,
        };
        let requires_clause = match item.requires_clause() {
            Some(it) => Some(Box::new(RequiresClause::try_from(it)?)),
            None => None,
        };
        let block_expr = match item.block_expr() {
            Some(it) => Some(Box::new(BlockExpr::try_from(it)?)),
            None => None,
        };
        Ok (Self {
            attrs,
            assert_token,
            l_paren_token, 
            expr,
            r_paren_token,
            by_token,
            name,
            requires_clause,
            block_expr,
        })
    }
}
 */