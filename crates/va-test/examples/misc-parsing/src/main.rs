use vstd::prelude::*;

verus! {

pub axiom fn foo(x: u8) requires x == 5; 

} // verus!

fn main() { }
