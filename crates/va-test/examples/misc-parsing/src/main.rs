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

*/
fn inv5()
    requires true,
    opens_invariants any
{
}

} // verus!

fn main() { }
