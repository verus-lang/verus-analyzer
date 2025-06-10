use vstd::prelude::*;

verus! {

pub exec const BDF_DEVICE_MASK: u16
    ensures BDF_DEVICE_MASK == 31
{
    31
}

const fn e() -> (u: u64) ensures u == 1 { 1 }
exec const E: u64 ensures E == 2 { 1 + e() }

exec const F: u64 ensures true { 1 }

spec const SPEC_E: u64 = 7;
#[verifier::when_used_as_spec(SPEC_E)]
exec const E: u64 ensures E == SPEC_E { 7 }


//exec static E: u64 ensures false {
//    proof { let x = F; }
//    0
//}
//exec static F: u64 ensures false {
//    proof { let x = E; }
//    0
//}

exec const E: u64 ensures E == f() {
    proof {
        let x = e();
        assert(x == f());
        assert(x == 1);
    }
    assert(1 == f());
    1
}

} // verus!

fn main() { }
