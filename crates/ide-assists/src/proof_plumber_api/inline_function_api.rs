//! Helpers for inlining a contract expression at a call site.

use crate::{AssistContext, handlers::inline_call::*};
use syntax::{
    AstNode,
    ast::{self, HasName, vst},
    syntax_editor::SyntaxEditor,
};

impl AssistContext<'_, '_> {
    pub(crate) fn vst_inline_call(
        &self,
        name_ref: vst::NameRef,
        expr_to_inline: vst::Expr,
    ) -> Option<vst::Expr> {
        let name_ref = name_ref.cst?;
        let krate = self.sema.scope(name_ref.syntax())?.krate().into();
        let call_info = CallInfo::from_name_ref(name_ref.clone(), krate)?;
        let function = match &call_info.node {
            ast::CallableExpr::Call(call) => {
                let path = match call.expr()? {
                    ast::Expr::PathExpr(path) => path.path(),
                    _ => None,
                }?;
                match self.sema.resolve_path(&path)? {
                    hir::PathResolution::Def(hir::ModuleDef::Function(function)) => function,
                    _ => return None,
                }
            }
            ast::CallableExpr::MethodCall(call) => self.sema.resolve_method_call(call)?,
        };

        let fn_source = self.sema.source(function)?.value;
        let param_list = fn_source.param_list()?;
        let mut param_names = Vec::new();
        if param_list.self_param().is_some() {
            param_names.push("self".to_owned());
        }
        for param in param_list.params() {
            let ast::Pat::IdentPat(pat) = param.pat()? else {
                // Replacing destructured parameters requires pattern-aware substitution.
                return None;
            };
            if !pat.is_simple_ident() {
                return None;
            }
            param_names.push(pat.name()?.to_string());
        }
        if param_names.len() != call_info.arguments.len() {
            return None;
        }
        let substitutions: Vec<_> =
            param_names.into_iter().zip(call_info.arguments.iter().cloned()).collect();

        let (editor, replacement) = SyntaxEditor::with_ast_node(&expr_to_inline.cst()?);
        let path_exprs: Vec<_> =
            replacement.syntax().descendants().filter_map(ast::PathExpr::cast).collect();
        for path_expr in path_exprs {
            let path = path_expr.path()?;
            if path.qualifier().is_some() {
                continue;
            }
            let referenced_name = path.segment()?.name_ref()?.to_string();
            let Some((_, argument)) =
                substitutions.iter().find(|(name, _)| name == &referenced_name)
            else {
                continue;
            };

            let mut argument = argument.clone();
            if let Some(parent) = path_expr.syntax().parent()
                && argument.needs_parens_in_place_of(&parent, path_expr.syntax())
            {
                argument = editor.make().expr_paren(argument).into();
            }
            editor.replace(path_expr.syntax(), argument.syntax());
        }

        let replacement = ast::Expr::cast(editor.finish().new_root().clone())?;
        vst::Expr::try_from(replacement).ok()
    }
}
