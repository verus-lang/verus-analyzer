use vstd::prelude::*;

verus! {

struct Foo { x: nat }

spec fn foo(f: Foo) -> bool {
    &&& f == Foo { x: 3 }
}

} // verus!

fn main() { }
