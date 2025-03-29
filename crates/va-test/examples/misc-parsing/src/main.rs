use vstd::prelude::*;

verus! {

pub open (crate) spec fn test1() {}
pub open (in foo) spec fn test2() {}
pub open (in crate::m) spec fn test3() { }
pub open (super) spec fn test4() {}

} // verus!

fn main() { }
