verus! {
/*
fn lemma()
    ensures
        #[trigger]
        true,
{
}
*/

fn lemma_mul_by_zero_is_zero()
    ensures
        #![trigger x]
        true,
{
//    assert forall|x: int| #![trigger x * 0] #![trigger 0 * x] x * 0 == 0 && 0 * x == 0 by {
//        lemma_mul_basics(x);
//    }
}


}


fn main() { }
