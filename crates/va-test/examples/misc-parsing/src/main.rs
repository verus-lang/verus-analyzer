use vstd::prelude::*;

verus! {

fn foo()
    ensures true,
    no_unwind when true
{ }

} // verus!

fn main() { }
