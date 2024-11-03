verus! {

fn test() {
    assert(p % p == 0) by (nonlinear_arith)
        requires
        p != 0,
    ;
}


} // verus!

fn main() { }
