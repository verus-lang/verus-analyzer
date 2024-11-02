use super::*;

pub(super) const ATTRIBUTE_FIRST: TokenSet = TokenSet::new(&[T![#]]);

pub(super) fn inner_attrs(p: &mut Parser<'_>) {
    while p.at(T![#]) && p.nth(1) == T![!] {
        attr(p, true);
    }
}

pub(super) fn outer_attrs(p: &mut Parser<'_>) {
    while p.at(T![#]) {
        attr(p, false);
    }
}

fn attr(p: &mut Parser<'_>, inner: bool) {
    assert!(p.at(T![#]));
    let attr = p.start();
    p.bump(T![#]);

    // REVIEW: This is more permissive than Rust/rust-analyzer,
    // because Verus uses inner-attribute syntax for #![trigger e]
    // in places where Rust only expects outer attributes
    let mut true_inner = inner;
    if p.at(T![!]) {
        p.bump(T![!]);
        true_inner = true;
    }
    // if inner {
    //     p.bump(T![!]);
    // }

    if p.eat(T!['[']) {
        if p.at(T![trigger]) {
            dbg!(true_inner);
            verus::trigger_attribute(p, true_inner);
        } else {
            meta(p);
        }

        if !p.eat(T![']']) {
            p.error("expected `]`");
        }
    } else {
        p.error("expected `[`");
    }
    attr.complete(p, ATTR);
}

// test metas
// #![simple_ident]
// #![simple::path]
// #![simple_ident_expr = ""]
// #![simple::path::Expr = ""]
// #![simple_ident_tt(a b c)]
// #![simple_ident_tt[a b c]]
// #![simple_ident_tt{a b c}]
// #![simple::path::tt(a b c)]
// #![simple::path::tt[a b c]]
// #![simple::path::tt{a b c}]
// #![unsafe(simple_ident)]
// #![unsafe(simple::path)]
// #![unsafe(simple_ident_expr = "")]
// #![unsafe(simple::path::Expr = "")]
// #![unsafe(simple_ident_tt(a b c))]
// #![unsafe(simple_ident_tt[a b c])]
// #![unsafe(simple_ident_tt{a b c})]
// #![unsafe(simple::path::tt(a b c))]
// #![unsafe(simple::path::tt[a b c])]
// #![unsafe(simple::path::tt{a b c})]
pub(super) fn meta(p: &mut Parser<'_>) {
    let meta = p.start();
    let is_unsafe = p.eat(T![unsafe]);
    if is_unsafe {
        p.expect(T!['(']);
    }
    paths::use_path(p);

    match p.current() {
        T![=] => {
            p.bump(T![=]);
            if expressions::expr(p).is_none() {
                p.error("expected expression");
            }
        }
        T!['('] | T!['['] | T!['{'] => items::token_tree(p),
        _ => {}
    }
    if is_unsafe {
        p.expect(T![')']);
    }

    meta.complete(p, META);
}
