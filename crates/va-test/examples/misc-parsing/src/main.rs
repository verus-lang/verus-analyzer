verus! {
/*
fn is_nonnull(&self)
{
}

fn into_raw() -> (tracked points_to_raw: PointsToRaw)
{
}
*/

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

} // verus!

fn main() { }
