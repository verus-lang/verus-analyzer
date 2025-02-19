use vstd::prelude::*;

verus! {

proof fn sufficiently_creamy() -> bool
    requires 
        bev !is Coffee,
{
    assert(s !has 3);
    assert(s !has 3 == true);
    assert(s !has 3 == s !has 3);
    assert(ms !has 4);
    assert(ms !has 4 == ms !has 4);
    true
}

} // verus!

fn main() { }
