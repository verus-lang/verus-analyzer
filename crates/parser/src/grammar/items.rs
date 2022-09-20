mod consts;
mod adt;
mod traits;
mod use_item;

pub(crate) use self::{
    adt::{record_field_list, variant_list},
    expressions::{match_arm_list, record_expr_field_list},
    traits::assoc_item_list,
    use_item::use_tree_list,
};
use super::*;

// test mod_contents
// fn foo() {}
// macro_rules! foo {}
// foo::bar!();
// super::baz! {}
// struct S;
pub(super) fn mod_contents(p: &mut Parser<'_>, stop_on_r_curly: bool) {
    attributes::inner_attrs(p);
    while !p.at(EOF) && !(p.at(T!['}']) && stop_on_r_curly) {
        item_or_macro(p, stop_on_r_curly);
    }
}

pub(super) const ITEM_RECOVERY_SET: TokenSet = TokenSet::new(&[
    T![fn],
    T![struct],
    T![enum],
    T![impl],
    T![trait],
    T![const],
    T![static],
    T![let],
    T![mod],
    T![pub],
    T![crate],
    T![use],
    T![macro],
    T![;],
    //verus
    // T![proof],
]);

pub(super) fn item_or_macro(p: &mut Parser<'_>, stop_on_r_curly: bool) {
    
    let m = p.start();
    attributes::outer_attrs(p);

    let m = match opt_item(p, m) {
        Ok(()) => {
            if p.at(T![;]) {
                p.err_and_bump(
                    "expected item, found `;`\n\
                     consider removing this semicolon",
                );
            }
            return;
        }
        Err(m) => m,
    };

    if paths::is_use_path_start(p) {
        match macro_call(p) {
            BlockLike::Block => (),
            BlockLike::NotBlock => {
                p.expect(T![;]);
            }
        }
        m.complete(p, MACRO_CALL);
        return;
    }

    m.abandon(p);
    match p.current() {
        T!['{'] => error_block(p, "expected an item"),
        T!['}'] if !stop_on_r_curly => {
            let e = p.start();
            p.error("unmatched `}`");
            p.bump(T!['}']);
            e.complete(p, ERROR);
        }
        EOF | T!['}'] => p.error("expected an item"),
        _ => p.err_and_bump("expected an item"),
    }
}

/// Try to parse an item, completing `m` in case of success.
pub(super) fn opt_item(p: &mut Parser<'_>, m: Marker) -> Result<(), Marker> {
    // test_err pub_expr
    // fn foo() { pub 92; }

    dbg!("opt-item");
    let has_visibility = opt_visibility(p, false);

    let m = match opt_item_without_modifiers(p, m) {
        Ok(()) => return Ok(()),
        Err(m) => m,
    };

    let mut has_mods = false;
    let mut has_extern = false;

    if p.at(T![verus]) {
        dbg!("hi Verus"); 
        p.bump(T![verus]);
        p.bump(T![!]);
        // item_list(p);
        p.expect(T!['{']);
        m.abandon(p);
        while !p.at(EOF) && !(p.at(T!['}'])) {
            item_or_macro(p, true);
        }
        let m = p.start();
        p.expect(T!['}']);
        m.abandon(p);
        // m.complete(p, VERUS);
        // p.bump(T!['{']);
        // token_tree(p);
         
        // m.abandon(p);
        return Ok(());
    }
    if p.at(T![proof]) {
        p.bump(T![proof]);
        // m.complete(p, PROOF_KW);
        m.abandon(p);
        return Ok(());
    }
    if p.at(T![spec]) {
        p.bump(T![spec]);
        // m.complete(p, SPEC_KW);
        m.abandon(p);
        return Ok(());
    }
    if p.at(T![open]) {
        p.bump(T![open]);
        // m.complete(p, OPEN_KW);
        m.abandon(p);
        return Ok(());
    }
    if p.at(T![closed]) {
        p.bump(T![closed]);
        // m.complete(p, CLOSED_KW);
        m.abandon(p);
        return Ok(());
    }

    if p.at(T![assert]) {
        assert(p,m);
        return Ok(());
    }
    if p.at(T![assume]) {
        assume(p, m);
        return Ok(());
    }



    // modifiers
    if p.at(T![const]) && p.nth(1) != T!['{'] {
        p.eat(T![const]);
        has_mods = true;
    }

    // test_err async_without_semicolon
    // fn foo() { let _ = async {} }
    if p.at(T![async]) && !matches!(p.nth(1), T!['{'] | T![move] | T![|]) {
        p.eat(T![async]);
        has_mods = true;
    }

    // test_err unsafe_block_in_mod
    // fn foo(){} unsafe { } fn bar(){}
    if p.at(T![unsafe]) && p.nth(1) != T!['{'] {
        p.eat(T![unsafe]);
        has_mods = true;
    }

    if p.at(T![extern]) {
        has_extern = true;
        has_mods = true;
        abi(p);
    }
    if p.at_contextual_kw(T![auto]) && p.nth(1) == T![trait] {
        p.bump_remap(T![auto]);
        has_mods = true;
    }

    // test default_item
    // default impl T for Foo {}
    if p.at_contextual_kw(T![default]) {
        match p.nth(1) {
            T![fn] | T![type] | T![const] | T![impl] => {
                p.bump_remap(T![default]);
                has_mods = true;
            }
            // test default_unsafe_item
            // default unsafe impl T for Foo {
            //     default unsafe fn foo() {}
            // }
            T![unsafe] if matches!(p.nth(2), T![impl] | T![fn]) => {
                p.bump_remap(T![default]);
                p.bump(T![unsafe]);
                has_mods = true;
            }
            // test default_async_fn
            // impl T for Foo {
            //     default async fn foo() {}
            // }
            T![async] => {
                let mut maybe_fn = p.nth(2);
                let is_unsafe = if matches!(maybe_fn, T![unsafe]) {
                    // test default_async_unsafe_fn
                    // impl T for Foo {
                    //     default async unsafe fn foo() {}
                    // }
                    maybe_fn = p.nth(3);
                    true
                } else {
                    false
                };

                if matches!(maybe_fn, T![fn]) {
                    p.bump_remap(T![default]);
                    p.bump(T![async]);
                    if is_unsafe {
                        p.bump(T![unsafe]);
                    }
                    has_mods = true;
                }
            }
            _ => (),
        }
    }

    // test existential_type
    // existential type Foo: Fn() -> usize;
    if p.at_contextual_kw(T![existential]) && p.nth(1) == T![type] {
        p.bump_remap(T![existential]);
        has_mods = true;
    }

    // items
    match p.current() {
        T![fn] => fn_(p, m),

        T![const] if p.nth(1) != T!['{'] => consts::konst(p, m),

        T![trait] => traits::trait_(p, m),
        T![impl] => traits::impl_(p, m),

        T![type] => type_alias(p, m),

        // test extern_block
        // unsafe extern "C" {}
        // extern {}
        T!['{'] if has_extern => {
            extern_item_list(p);
            m.complete(p, EXTERN_BLOCK);
        }

        _ if has_visibility || has_mods => {
            if has_mods {
                p.error("expected existential, fn, trait or impl");
            } else {
                p.error("expected an item");
            }
            m.complete(p, ERROR);
        }

        _ => return Err(m),
    }
    Ok(())
}

fn opt_item_without_modifiers(p: &mut Parser<'_>, m: Marker) -> Result<(), Marker> {
    let la = p.nth(1);
    match p.current() {
        T![extern] if la == T![crate] => extern_crate(p, m),
        T![use] => use_item::use_(p, m),
        T![mod] => mod_item(p, m),

        T![type] => type_alias(p, m),
        T![struct] => adt::strukt(p, m),
        T![enum] => adt::enum_(p, m),
        IDENT if p.at_contextual_kw(T![union]) && p.nth(1) == IDENT => adt::union(p, m),

        T![macro] => {macro_def(p, m)},
        IDENT if p.at_contextual_kw(T![macro_rules]) && p.nth(1) == BANG => macro_rules(p, m),

        T![const] if (la == IDENT || la == T![_] || la == T![mut]) => consts::konst(p, m),
        T![static] if (la == IDENT || la == T![_] || la == T![mut]) => consts::static_(p, m),

        // T![proof] => {dbg!("hey"); panic!();}
        _ => return Err(m),
    };
    Ok(())
}

// test extern_crate
// extern crate foo;
fn extern_crate(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![extern]);
    p.bump(T![crate]);

    if p.at(T![self]) {
        // test extern_crate_self
        // extern crate self;
        let m = p.start();
        p.bump(T![self]);
        m.complete(p, NAME_REF);
    } else {
        name_ref(p);
    }

    // test extern_crate_rename
    // extern crate foo as bar;
    opt_rename(p);
    p.expect(T![;]);
    m.complete(p, EXTERN_CRATE);
}

// test mod_item
// mod a;
pub(crate) fn mod_item(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![mod]);
    name(p);
    if p.at(T!['{']) {
        // test mod_item_curly
        // mod b { }
        item_list(p);
    } else if !p.eat(T![;]) {
        p.error("expected `;` or `{`");
    }
    m.complete(p, MODULE);
}

// pub(crate) fn verus_item(p: &mut Parser<'_>, m: Marker) {
//     p.bump(T![mod]);
//     name(p);
//     if p.at(T!['{']) {
//         // test mod_item_curly
//         // mod b { }
//         item_list(p);
//     } else if !p.eat(T![;]) {
//         p.error("expected `;` or `{`");
//     }
//     m.complete(p, MODULE);


//     if p.at(T![verus]) {
//         dbg!("hi Verus"); 
//         p.bump(T![verus]);
//         p.bump(T![!]);
//         p.bump(T!['{']);
//         // token_tree(p);
//         m.complete(p, VERUS_KW);
//         return Ok(());
//     }
// }


// test type_alias
// type Foo = Bar;
fn type_alias(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![type]);

    name(p);

    // test type_item_type_params
    // type Result<T> = ();
    generic_params::opt_generic_param_list(p);

    if p.at(T![:]) {
        generic_params::bounds(p);
    }

    // test type_item_where_clause_deprecated
    // type Foo where Foo: Copy = ();
    generic_params::opt_where_clause(p);
    if p.eat(T![=]) {
        types::type_(p);
    }

    // test type_item_where_clause
    // type Foo = () where Foo: Copy;
    generic_params::opt_where_clause(p);

    p.expect(T![;]);
    m.complete(p, TYPE_ALIAS);
}

pub(crate) fn item_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    mod_contents(p, true);
    p.expect(T!['}']);
    m.complete(p, ITEM_LIST);
}

pub(crate) fn extern_item_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    mod_contents(p, true);
    p.expect(T!['}']);
    m.complete(p, EXTERN_ITEM_LIST);
}

fn macro_rules(p: &mut Parser<'_>, m: Marker) {
    assert!(p.at_contextual_kw(T![macro_rules]));
    p.bump_remap(T![macro_rules]);
    p.expect(T![!]);

    if p.at(IDENT) {
        name(p);
    }
    // Special-case `macro_rules! try`.
    // This is a hack until we do proper edition support

    // test try_macro_rules
    // macro_rules! try { () => {} }
    if p.at(T![try]) {
        let m = p.start();
        p.bump_remap(IDENT);
        m.complete(p, NAME);
    }

    match p.current() {
        // test macro_rules_non_brace
        // macro_rules! m ( ($i:ident) => {} );
        // macro_rules! m [ ($i:ident) => {} ];
        T!['['] | T!['('] => {
            token_tree(p);
            p.expect(T![;]);
        }
        T!['{'] => token_tree(p),
        _ => p.error("expected `{`, `[`, `(`"),
    }
    m.complete(p, MACRO_RULES);
}



// test macro_def
// macro m($i:ident) {}
fn macro_def(p: &mut Parser<'_>, m: Marker) {
    p.expect(T![macro]);
    name_r(p, ITEM_RECOVERY_SET);
    if p.at(T!['{']) {
        // test macro_def_curly
        // macro m { ($i:ident) => {} }
        token_tree(p);
    } else if p.at(T!['(']) {
        let m = p.start();
        token_tree(p);
        match p.current() {
            T!['{'] | T!['['] | T!['('] => token_tree(p),
            _ => p.error("expected `{`, `[`, `(`"),
        }
        m.complete(p, TOKEN_TREE);
    } else {
        p.error("unmatched `(`");
    }

    m.complete(p, MACRO_DEF);
}


fn assume(p: &mut Parser<'_>, m: Marker) {
    p.expect(T![assume]);
    p.expect(T!['(']);
    expressions::expr(p);
    p.expect(T![')']);
    m.complete(p, ASSUME_EXPR);
}

// AssertExpr =
//   'assert' '(' Expr ')' 'by'? ( '(' Name ')' )?  RequiresClause? BlockExpr?
fn assert(p: &mut Parser<'_>, m: Marker) {
    p.expect(T![assert]);
    
    if p.at(T!['(']) {
        // parse expression here
        p.expect(T!['(']);
        expressions::expr(p);
        p.expect(T![')']);
    } else {
        // TODO: make this a separate kind AssertForall
        // assert forall|x: int, y: int| f1(x) + f1(y) == x + y + 2 by {
        //     reveal(f1);
        // }
        p.error("TODO: make this a separate kind AssertForall");
        expressions::expr(p);
        if p.at(T![implies]) {
            p.bump(T![implies]);
            expressions::expr(p);
        }
        // p.error("expected function arguments");
    }
    
    // parse optional `by`
    // bit_vector, nonlinear_artih ...
    if p.at(T![by]) {
        p.expect(T![by]);
        if p.at(T!['(']) {
            p.expect(T!['(']);
            // p.bump_any();
            name_r(p, ITEM_RECOVERY_SET);
            p.expect(T![')']);
        }
    }

    // parse optional 'requires`
    if p.at(T![requires]) {
        requires(p);
    }

    if p.at(T![;]) {
        // test fn_decl
        // trait T { fn foo(); }
        dbg!("getting ;, but ignoring");
        // p.bump(T![;]);
    } else {
        dbg!("proof block ;");
        // parse optional 'proof block'
        expressions::block_expr(p);
    }

    m.complete(p, ASSERT_EXPR);
}



// see verus/dependencies/syn/src/items.rs, impl parse for Signature
// Fn =
//  Attr* Visibility? Publish?
//  'default'? 'const'? 'async'? 'unsafe'? Abi? FnMode?
//  'fn' Name GenericParamList? ParamList RetType? WhereClause? RequiresClause? EnsuresClause?
//  (body:BlockExpr | ';')
//
// TODO: parse properly 'publish', 'fnmode'
// Note: requires -> recommends -> ensures -> decreases 
// 
// test fn
// fn foo() {}
fn fn_(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![fn]);

    name_r(p, ITEM_RECOVERY_SET);
    // test function_type_params
    // fn foo<T: Clone + Copy>(){}
    generic_params::opt_generic_param_list(p);

    if p.at(T!['(']) {
        params::param_list_fn_def(p);
    } else {
        p.error("expected function arguments");
    }
    // test function_ret_type
    // fn foo() {}
    // fn bar() -> () {}
    
    // opt_ret_type(p);
    // verus specific return naming
    if p.at(T![->]) {
        let m = p.start();
        p.bump(T![->]);
        if p.at(T![tracked]) {
            p.expect(T![tracked]);
        }
        if p.at(T!['(']) {
            // verus named param    
            p.expect(T!['(']);
            patterns::pattern(p); 
            p.expect(T![:]);   
            types::type_no_bounds(p);
            p.expect(T![')']);
        } else {
            types::type_no_bounds(p);
        }
        m.complete(p, RET_TYPE);
    } 


    // test function_where_clause
    // fn foo<T>() where T: Copy {}
    generic_params::opt_where_clause(p);


    // Note: requires -> recommends -> ensures -> decreases 
    // optional parsing of `requires` and `ensures`
    if p.at(T![requires]) {
        requires(p);
    }
    if p.at(T![recommends]) {
        recommends(p);
    }
    if p.at(T![ensures]) {
        ensures(p);
    }
    if p.at(T![decreases]) {
        decreases(p);
    }



    if p.at(T![;]) {
        // test fn_decl
        // trait T { fn foo(); }
        p.bump(T![;]);
    } else {
        expressions::block_expr(p);
    }
    m.complete(p, FN);
}


// fn req_clause(p: &mut Parser<'_>) {
//     let m = p.start();
//     while !p.at(EOF) && !p.at(T![,]) {
//         p.bump_any();
//         expr_no_struct(p);
//         // if p.at(T![,]) {

//         //     break;
//         // }
//     }
//     p.expect(T![,]);
//     p.eat(T![,]);
//     m.complete(p, REQUIRES_CLAUSE);
// }

// fn ens_clause(p: &mut Parser<'_>) {
//     let m = p.start();
//     while !p.at(EOF) && !p.at(T![,]) {
//         p.bump_any();
//         // if p.at(T![,]) {

//         //     break;
//         // }
//     }
//     p.expect(T![,]);
//     p.eat(T![,]);
//     m.complete(p, ENSURES_CLAUSE);
// }





fn macro_call(p: &mut Parser<'_>) -> BlockLike {
    assert!(paths::is_use_path_start(p));
    paths::use_path(p);
    macro_call_after_excl(p)
}

pub(super) fn macro_call_after_excl(p: &mut Parser<'_>) -> BlockLike {
    p.expect(T![!]);

    match p.current() {
        T!['{'] => {
            token_tree(p);
            BlockLike::Block
        }
        T!['('] | T!['['] => {
            token_tree(p);
            BlockLike::NotBlock
        }
        _ => {
            p.error("expected `{`, `[`, `(`");
            BlockLike::NotBlock
        }
    }
}

pub(crate) fn token_tree(p: &mut Parser<'_>) {
    let closing_paren_kind = match p.current() {
        T!['{'] => T!['}'],
        T!['('] => T![')'],
        T!['['] => T![']'],
        _ => unreachable!(),
    };
    let m = p.start();
    p.bump_any();
    while !p.at(EOF) && !p.at(closing_paren_kind) {
        match p.current() {
            T!['{'] | T!['('] | T!['['] => token_tree(p),
            T!['}'] => {
                p.error("unmatched `}`");
                m.complete(p, TOKEN_TREE);
                return;
            }
            T![')'] | T![']'] => p.err_and_bump("unmatched brace"),
            _ => p.bump_any(),
        }
    }
    p.expect(closing_paren_kind);
    m.complete(p, TOKEN_TREE);
}













fn requires(p: &mut Parser<'_>) -> CompletedMarker {
    dbg!("requires");
    let m = p.start();
    p.expect(T![requires]);

    while !p.at(EOF) && !p.at(T![recommends]) && !p.at(T![ensures]) && !p.at(T![decreases]) && !p.at(T!['{']) {
        cond_comma(p);
        if p.at(T![recommends]) || p.at(T![ensures]) || p.at(T![decreases]) || p.at(T!['{']) {
            break;
        }
    }
    m.complete(p, REQUIRES_CLAUSE)
}


fn recommends(p: &mut Parser<'_>) -> CompletedMarker {
    dbg!("recommends");
    let m = p.start();
    p.expect(T![recommends]);
    while !p.at(EOF) && !p.at(T![ensures]) && !p.at(T![decreases]) && !p.at(T!['{']) {
        cond_comma(p);
        if p.at(T![recommends]) || p.at(T![ensures]) || p.at(T![decreases]) || p.at(T!['{']) {
            break;
        }
    }
    m.complete(p, RECOMMENDS_CLAUSE)
}


fn ensures(p: &mut Parser<'_>) -> CompletedMarker {
    dbg!("ensures");
    let m = p.start();
    p.expect(T![ensures]);

    while !p.at(EOF)  && !p.at(T![decreases]) && !p.at(T!['{']) {
        cond_comma(p);
        if p.at(T![recommends]) || p.at(T![ensures]) || p.at(T![decreases]) || p.at(T!['{']) {
            break;
        }
    }
    m.complete(p, ENSURES_CLAUSE)
}

fn decreases(p: &mut Parser<'_>) -> CompletedMarker {
    dbg!("decreases");
    let m = p.start();
    p.expect(T![decreases]);
    patterns::pattern(p); 
    while !p.at(EOF) && !p.at(T!['{']) {
        comma_pat(p);
        if p.at(T![recommends]) || p.at(T![ensures]) || p.at(T![decreases]) || p.at(T!['{']) {
            break;
        }
    }
    m.complete(p, DECREASES_CLAUSE)
}


fn cond_comma(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expressions::expr(p);
    p.expect(T![,]);
    m.complete(p, COND_AND_COMMA)
}

fn comma_pat(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    p.expect(T![,]);
    patterns::pattern(p); 
    m.complete(p, COMMA_AND_PAT)
}

// fn recommends_clause(p: &mut Parser<'_>) {
//     let m = p.start();
//     while !p.at(EOF) && !p.at(T![,]) {
//         p.bump_any();
//         // if p.at(T![,]) {

//         //     break;
//         // }
//     }
//     p.expect(T![,]);
//     p.eat(T![,]);
//     m.complete(p, RECOMMENDS_CLAUSE);
// }
