verus! {

#[cfg(verus_keep_ghost)]
#[verifier::proof]
#[verifier::custom_req_err("unable to prove assertion safety condition")] /* vattr */
pub fn assert_safety(b: bool) {
    requires(b);
    ensures(b);
}

} // verus!

fn main() { }
