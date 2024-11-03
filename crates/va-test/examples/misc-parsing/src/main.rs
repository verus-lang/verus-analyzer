verus! {

fn closed_under_incl()
    requires
        Self::op(a, b).valid(),
    ensures
        a.valid(),
;

} // verus!

fn main() { }
