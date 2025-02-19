use vstd::prelude::*;

verus! {

pub(crate) proof fn binary_ops<A>(a: A, x: int) {
//    assert(false ==> true);
//    assert(true && false ==> false && false);
//    assert(!(true && (false ==> false) && false));
//    assert(false ==> false ==> false);
//    assert(false ==> (false ==> false));
//    assert(!((false ==> false) ==> false));
//    assert(false <== false <== false);
//    assert(!(false <== (false <== false)));
//    assert((false <== false) <== false);
    assert(2 + 2 !== 3);
//    assert(a == a);
//    assert(false <==> true && false);
}

} // verus!

fn main() { }
