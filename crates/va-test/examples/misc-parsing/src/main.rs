use vstd::prelude::*;

verus! {

proof fn sufficiently_creamy() -> bool
    requires 
        bev !is Coffee,
{
   true
}

} // verus!

fn main() { }
