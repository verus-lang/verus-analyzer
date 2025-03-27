use vstd::prelude::*;

verus! {

proof fn testfn() {
    let tracked f = proof_fn |y: u64| -> (z: u64)
        requires
            y == 2,
        ensures
            z == 2,
        { y };
    assert(f.requires((2,)));
    assert(!f.ensures((2,), 3));
    let t = f(2);
    assert(t == 2);
}
proof fn helper(tracked f: proof_fn(y: u64) -> u64)
    requires
        f.requires((2,)),
        forall|z: u64| f.ensures((2,), z) ==> z == 2,
{
    let t = f(2);
    assert(t == 2);
}
proof fn testfn() {
    let tracked f = proof_fn |y: u64| -> (z: u64)
        requires
            y == 2,
        ensures
            z == 2,
        { y };
    helper(f);
}
proof fn test() {
    let tracked f = proof_fn[Mut, Copy, Send, ReqEns<foo>, Sync] |y: u64| -> (z: u64) { y };
}
proof fn foo(x: proof_fn(a: u32) -> u64, y: proof_fn[Send](a: u32) -> u64) {
}

} // verus!

fn main() { }
