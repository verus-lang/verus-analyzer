use super::*;

fn at_kw(p: &Parser<'_>, kw: SyntaxKind) -> bool {
    p.at_contextual_kw(kw)
}

fn eat_kw(p: &mut Parser<'_>, kw: SyntaxKind) -> bool {
    p.eat_contextual_kw(kw)
}

fn expect_kw(p: &mut Parser<'_>, kw: SyntaxKind) {
    p.expect_contextual_kw(kw);
}

pub(super) fn at_contract_boundary(p: &Parser<'_>) -> bool {
    p.at(EOF)
        || p.at(T!['{'])
        || p.at(T![;])
        || p.at(T![']'])
        || at_kw(p, T![requires])
        || at_kw(p, T![recommends])
        || at_kw(p, T![ensures])
        || at_kw(p, T![default_ensures])
        || at_kw(p, T![returns])
        || at_kw(p, T![decreases])
        || at_kw(p, T![opens_invariants])
        || at_kw(p, T![no_unwind])
        || at_kw(p, T![invariant])
        || at_kw(p, T![invariant_except_break])
        || at_kw(p, T![when])
        || at_kw(p, T![via])
}

fn expr_list(p: &mut Parser<'_>) {
    expressions::expr_no_struct(p);
    while p.eat(T![,]) {
        if at_contract_boundary(p) {
            break;
        }
        expressions::expr_no_struct(p);
    }
}

pub(super) fn ret_type(p: &mut Parser<'_>) -> bool {
    if !p.at(T![->]) {
        return false;
    }

    let m = p.start();
    p.bump(T![->]);
    eat_kw(p, T![tracked]);
    if p.at(T!['('])
        && (p.nth_at(1, IDENT) && p.nth_at(2, T![:]) && !p.nth_at(2, T![::])
            || p.nth_at_contextual_kw(1, T![tracked]) && p.nth_at(2, IDENT) && p.nth_at(3, T![:]))
    {
        p.bump(T!['(']);
        eat_kw(p, T![tracked]);
        patterns::pattern(p);
        p.expect(T![:]);
        types::type_no_bounds(p);
        p.expect(T![')']);
    } else {
        types::type_no_bounds(p);
    }
    m.complete(p, RET_TYPE);
    true
}

pub(super) fn proof_fn_type(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![proof_fn]);
    proof_fn_characteristics(p);
    generic_params::opt_generic_param_list(p);
    if p.at(T!['(']) {
        params::param_list_fn_ptr(p);
    } else {
        p.error("expected proof function parameters");
    }
    ret_type(p);
    m.complete(p, PROOF_FN_TYPE)
}

fn proof_fn_characteristics(p: &mut Parser<'_>) -> Option<CompletedMarker> {
    if !p.at(T!['[']) {
        return None;
    }
    let m = p.start();
    p.bump(T!['[']);
    while !p.at(EOF) && !p.at(T![']']) {
        paths::type_path(p);
        if !p.eat(T![,]) {
            break;
        }
    }
    p.expect(T![']']);
    Some(m.complete(p, PROOF_FN_CHARACTERISTICS))
}

pub(super) fn proof_fn(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![proof_fn]);
    proof_fn_characteristics(p);
    m.complete(p, PROOF_FN_WITH_CHARACTERISTICS)
}

pub(super) fn closure_expr(
    p: &mut Parser<'_>,
    m: Option<Marker>,
    forbid_structs: bool,
) -> CompletedMarker {
    let m = m.unwrap_or_else(|| p.start());
    eat_kw(p, T![forall]);
    eat_kw(p, T![exists]);
    eat_kw(p, T![choose]);
    if !p.at(T![|]) {
        p.error("expected `|`");
        return m.complete(p, CLOSURE_EXPR);
    }
    params::param_list_closure(p);
    attributes::inner_attrs(p);
    if forbid_structs {
        expressions::expr_no_struct(p);
    } else {
        expressions::expr(p);
    }
    m.complete(p, CLOSURE_EXPR)
}

pub(super) fn view_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    p.bump(T![@]);
    m.complete(p, VIEW_EXPR)
}

pub(super) fn is_expr(p: &mut Parser<'_>, lhs: CompletedMarker, negated: bool) -> CompletedMarker {
    let m = lhs.precede(p);
    if negated {
        p.bump(T![!]);
    }
    expect_kw(p, T![is]);
    types::type_no_bounds(p);
    m.complete(p, IS_EXPR)
}

pub(super) fn has_expr(p: &mut Parser<'_>, lhs: CompletedMarker, negated: bool) -> CompletedMarker {
    let m = lhs.precede(p);
    if negated {
        p.bump(T![!]);
    }
    expect_kw(p, T![has]);
    expressions::expr(p);
    m.complete(p, HAS_EXPR)
}

pub(super) fn matches_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    expect_kw(p, T![matches]);
    patterns::pattern(p);
    m.complete(p, MATCHES_EXPR)
}

pub(super) fn arrow_expr(p: &mut Parser<'_>, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    p.bump(T![->]);
    if p.at(IDENT) {
        let name = p.start();
        p.bump(IDENT);
        name.complete(p, NAME_REF);
    } else {
        p.expect(INT_NUMBER);
    }
    m.complete(p, ARROW_EXPR)
}

pub(super) fn publish(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    if eat_kw(p, T![open]) {
        if p.eat(T!['(']) {
            p.eat(T![in]);
            paths::use_path(p);
            p.expect(T![')']);
        }
    } else if !eat_kw(p, T![closed]) && !eat_kw(p, T![uninterp]) {
        p.error("expected `open`, `closed`, or `uninterp`");
    }
    m.complete(p, PUBLISH)
}

pub(super) fn fn_mode(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    if eat_kw(p, T![spec]) {
        if p.eat(T!['(']) {
            expect_kw(p, T![checked]);
            p.expect(T![')']);
        }
    } else if !eat_kw(p, T![proof]) && !eat_kw(p, T![exec]) && !eat_kw(p, T![axiom]) {
        p.error("expected a Verus function mode");
    }
    m.complete(p, FN_MODE)
}

pub(super) fn data_mode(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    if !eat_kw(p, T![ghost]) && !eat_kw(p, T![tracked]) {
        p.error("expected `ghost` or `tracked`");
    }
    m.complete(p, DATA_MODE)
}

pub(super) fn broadcast_group(p: &mut Parser<'_>, m: Marker) -> CompletedMarker {
    name(p);
    p.expect(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        attributes::outer_attrs(p);
        paths::use_path(p);
        if !p.eat(T![,]) {
            break;
        }
    }
    p.expect(T!['}']);
    m.complete(p, BROADCAST_GROUP)
}

pub(super) fn broadcast_use(p: &mut Parser<'_>, m: Marker) -> CompletedMarker {
    p.bump(T![use]);
    let braced = p.eat(T!['{']);
    while !p.at(EOF) && !p.at(T![;]) && !p.at(T!['}']) {
        paths::use_path(p);
        if !p.eat(T![,]) {
            break;
        }
    }
    if braced {
        p.expect(T!['}']);
    }
    p.expect(T![;]);
    m.complete(p, BROADCAST_USE)
}

pub(super) fn assume(p: &mut Parser<'_>, m: Marker) -> CompletedMarker {
    expect_kw(p, T![assume]);
    p.expect(T!['(']);
    expressions::expr(p);
    p.expect(T![')']);
    m.complete(p, ASSUME_EXPR)
}

pub(super) fn final_expr(p: &mut Parser<'_>, m: Marker) -> CompletedMarker {
    expect_kw(p, T![final]);
    p.expect(T!['(']);
    expressions::expr(p);
    p.expect(T![')']);
    m.complete(p, FINAL_EXPR)
}

pub(super) fn assert(p: &mut Parser<'_>, m: Marker) -> CompletedMarker {
    expect_kw(p, T![assert]);
    if at_kw(p, T![forall]) {
        closure_expr(p, None, true);
        if eat_kw(p, T![implies]) {
            expressions::expr(p);
        }
        expect_kw(p, T![by]);
        expressions::block_expr(p);
        return m.complete(p, ASSERT_FORALL_EXPR);
    }

    p.expect(T!['(']);
    expressions::expr(p);
    p.expect(T![')']);
    if eat_kw(p, T![by]) && p.eat(T!['(']) {
        name(p);
        p.expect(T![')']);
    }
    if at_kw(p, T![requires]) {
        requires(p);
    }
    if p.at(T!['{']) {
        expressions::block_expr(p);
    }
    m.complete(p, ASSERT_EXPR)
}

pub(super) fn prover(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![by]);
    p.expect(T!['(']);
    name(p);
    p.expect(T![')']);
    m.complete(p, PROVER)
}

macro_rules! clause {
    ($name:ident, $keyword:tt, $kind:ident) => {
        pub(super) fn $name(p: &mut Parser<'_>) -> CompletedMarker {
            let m = p.start();
            expect_kw(p, T![$keyword]);
            expr_list(p);
            m.complete(p, $kind)
        }
    };
}

clause!(requires, requires, REQUIRES_CLAUSE);
clause!(ensures, ensures, ENSURES_CLAUSE);
clause!(default_ensures, default_ensures, DEFAULT_ENSURES_CLAUSE);
clause!(invariant, invariant, INVARIANT_CLAUSE);
clause!(invariant_except_break, invariant_except_break, INVARIANT_EXCEPT_BREAK_CLAUSE);
clause!(decreases, decreases, DECREASES_CLAUSE);

pub(super) fn recommends(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![recommends]);
    expr_list(p);
    if eat_kw(p, T![via]) {
        expressions::expr_no_struct(p);
    }
    m.complete(p, RECOMMENDS_CLAUSE)
}

pub(super) fn returns(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![returns]);
    expressions::expr_no_struct(p);
    p.eat(T![,]);
    m.complete(p, RETURNS_CLAUSE)
}

pub(super) fn signature_decreases(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    decreases(p);
    if eat_kw(p, T![when]) {
        expressions::expr_no_struct(p);
    }
    if eat_kw(p, T![via]) {
        expressions::expr_no_struct(p);
    }
    m.complete(p, SIGNATURE_DECREASES)
}

pub(super) fn opens_invariants(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![opens_invariants]);
    if !eat_kw(p, T![any]) && !eat_kw(p, T![none]) {
        if p.eat(T!['[']) {
            if !p.at(T![']']) {
                expr_list(p);
            }
            p.expect(T![']']);
        } else {
            expressions::expr_no_struct(p);
        }
    }
    m.complete(p, OPENS_INVARIANTS_CLAUSE)
}

pub(super) fn no_unwind(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![no_unwind]);
    if eat_kw(p, T![when]) {
        expressions::expr_no_struct(p);
    }
    m.complete(p, NO_UNWIND_CLAUSE)
}

pub(super) fn loop_clauses(p: &mut Parser<'_>) {
    loop {
        if at_kw(p, T![invariant_except_break]) {
            invariant_except_break(p);
        } else if at_kw(p, T![invariant]) {
            invariant(p);
        } else if at_kw(p, T![ensures]) {
            ensures(p);
        } else if at_kw(p, T![decreases]) {
            decreases(p);
        } else {
            break;
        }
    }
}

pub(super) fn trigger_attribute(p: &mut Parser<'_>, inner: bool) -> CompletedMarker {
    let m = p.start();
    expect_kw(p, T![trigger]);
    if inner && !p.at(T![']']) {
        expr_list(p);
    }
    m.complete(p, TRIGGER_ATTRIBUTE)
}

pub(super) fn global_clause(p: &mut Parser<'_>, m: Marker) {
    expect_kw(p, T![global]);
    if eat_kw(p, T![size_of]) {
        types::type_no_bounds(p);
        p.expect(T![==]);
        p.expect(INT_NUMBER);
    } else {
        expect_kw(p, T![layout]);
        types::type_no_bounds(p);
        expect_kw(p, T![is]);
        expect_kw(p, T![size]);
        p.expect(T![==]);
        p.expect(INT_NUMBER);
        p.expect(T![,]);
        expect_kw(p, T![align]);
        p.expect(T![==]);
        p.expect(INT_NUMBER);
    }
    p.expect(T![;]);
    m.complete(p, VERUS_GLOBAL);
}
