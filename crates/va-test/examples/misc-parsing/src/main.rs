verus! {
/*
fn is_nonnull(&self)
{
}

fn into_raw() -> (tracked points_to_raw: PointsToRaw)
{
}

fn inv1()
    opens_invariants none
{
}

fn inv2()
    opens_invariants any
{
}

fn inv3()
    opens_invariants [a, b, c]
{
}

fn kw_test() {
    let any = 5;
}

fn inv4()
    ensures true,
    opens_invariants any
{
}

fn put()
    requires
        self.id() === old(perm)@.pptr,
        old(perm)@.value === None,
    ensures
        perm@.pptr === old(perm)@.pptr,
        perm@.value === Some(v),
    opens_invariants none
    no_unwind
{
}

fn inv5()
    requires true,
    opens_invariants any
{
}

pub fn spawn(f: F) 
    requires
        f.requires(true),
    ensures
        f.ensures(ret),
{
}

pub fn clone_vec_u8() {
    let i = 0;
    while i < v.len()
        invariant_except_break
            i <= v.len(),
        invariant
            i <= v.len(),
            i == out.len(),
            forall |j| #![auto] 0 <= j < i  ==> out@[j] == v@[j],
        ensures
            i > 0,
        decreases
            72,
    {
        i = i + 1;
    }
}

fn reverse(v: &mut Vec<u64>)
    ensures
        forall|i: int| 0 <= i, 
{
    let length = v.len();
    let ghost v1 = v@;
    for n in 0..(length / 2)
        invariant
            length == v.len(),
            forall|i: int| 0 <= i < n ==> v[i] == v1[length - i - 1],
            forall|i: int| 0 <= i < n ==> v1[i] == v[length - i - 1],
            forall|i: int| n <= i && i + n < length ==> #[trigger] v[i] == v1[i],
    {
        let x = v[n];
        let y = v[length - 1 - n];
        v.set(n, y);
        v.set(length - 1 - n, x);
    }
}

fn test() {
    for x in iter: 0..end
        invariant
            end == 10,
    {
        n += 3;
    }
    let x = 2;
    for x in iter: vec_iter_copy(v)
        invariant
            b <==> (forall|i: int| 0 <= i < iter.cur ==> v[i] > 0),
    {
        b = b && x > 0;
    }
    let y = 3;
    for x in iter: 0..({
        let z = end;
        non_spec();
        z
    })
        invariant
            n == iter.cur * 3,
            end == 10,
    {
        n += 3;
        end = end + 0;  // causes end to be non-constant
    }
}

mod ring {
    use builtin::*;

    pub struct Ring {
        pub i: u64,
    }

    impl Ring {
        pub closed spec fn inv(&self) -> bool {
            self.i < 10
        }

        pub closed spec fn spec_succ(&self) -> Ring {
            Ring { i: if self.i == 9 { 0 } else { (self.i + 1) as u64 } }
        }

        pub closed spec fn spec_prev(&self) -> Ring {
            Ring { i: if self.i == 0 { 9 } else { (self.i - 1) as u64 } }
        }

        pub broadcast proof fn spec_succ_ensures(p: Ring)
            requires p.inv()
            ensures p.inv() && (#[trigger] p.spec_succ()).spec_prev() == p
        { }

        pub broadcast proof fn spec_prev_ensures(p: Ring)
            requires p.inv()
            ensures p.inv() && (#[trigger] p.spec_prev()).spec_succ() == p
        { }

        pub    broadcast    group    properties {
        Ring::spec_succ_ensures,
                Ring::spec_prev_ensures,
        }
    }

    #[verifier::prune_unless_this_module_is_used]
    pub    broadcast    group    properties {
    Ring::spec_succ_ensures,
            Ring::spec_prev_ensures,
    }
}

mod m2 {
    use builtin::*;
    use crate::ring::*;

    fn t2(p: Ring) requires p.inv() {
           broadcast    use     Ring::properties;
        assert(p.spec_succ().spec_prev() == p);
        assert(p.spec_prev().spec_succ() == p);
    }
}

mod m3 {
    use builtin::*;
    use crate::ring::*;

        broadcast   use    Ring::properties;
        
        fn a() { }
}

mod m4 {
    use builtin::*;
    use crate::ring::*;

        broadcast   use    
                    Ring::spec_succ_ensures,
            Ring::spec_prev_ensures;
}
*/

proof fn sufficiently_creamy() -> bool
    requires 
        bev is Coffee,
{
   bev->creamers
}

spec fn is_insect(l: Life) -> bool {
    l is Arthropod && l->Arthropod_legs == 6
}

spec fn rect_height(s: Shape) -> int
    recommends s is Rect
{
    s->1
}

spec fn cuddly(l: Life) -> bool
{
    ||| l matches Mammal{legs, ..} && legs == 4
    ||| l matches Arthropod{legs, wings} && legs == 8 && wings == 0
}

spec fn is_kangaroo(l: Life) -> bool
{
    &&& l matches Life::Mammal{legs, has_pocket}
    &&& legs == 2
    &&& has_pocket
}

spec fn walks_upright(l: Life) -> bool
{
    l matches Life::Mammal{legs, ..} ==> legs==2
}


} // verus!

fn main() { }
