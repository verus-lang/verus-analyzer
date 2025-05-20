use vstd::prelude::*;

verus! {

proof fn foo1() opens_invariants bar();
proof fn foo2() opens_invariants baz;
proof fn foo3() opens_invariants bar() {}
proof fn foo4() opens_invariants baz {}
proof fn foo5() opens_invariants Set::<int>::empty() {}
proof fn foo6() opens_invariants { let a = Set::<int>::empty(); let b = a.insert(c); b } {}

} // verus!

fn main() { }
