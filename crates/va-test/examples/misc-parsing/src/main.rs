use vstd::prelude::*;

verus! {

fn test() -> u8
    returns 20u8,
{
    20u8
}

proof fn proof_test() -> u8
    returns 20u8,
{
    20u8
}


fn test2() {
    let j = test();
    assert(j == 20);
}

fn test3() -> u8
    returns 20u8, // FAILS
{
    19u8
}

fn test4() -> u8
    returns 20u8,
{
    return 19u8; // FAILS
}

fn test5(a: u8, b: u8) -> (k: u8)
    requires a + b < 256,
    ensures a + b < 257,
    returns (a + b) as u8,
{
    return a; // FAILS
}

fn test6(a: u8, b: u8) -> (k: u8)
    requires a + b < 256,
    ensures a + b < 250,
    returns (a + b) as u8,
{
    return a + b; // FAILS
}

proof fn proof_test5(a: u8, b: u8) -> (k: u8)
    requires a + b < 256,
    ensures a + b < 257,
    returns (a + b) as u8,
{
    return a; // FAILS
}

proof fn proof_test6(a: u8, b: u8) -> (k: u8)
    requires a + b < 256,
    ensures a + b < 250,
    returns (a + b) as u8,
{
    return (a + b) as u8; // FAILS
}

pub assume_specification<T, I>[ <[T]>::get::<I> ](slice: &[T], i: I) -> (b: bool) 
    where I: core::slice::SliceIndex<[T]>,
    returns
        spec_slice_get(slice, i),
;

pub assume_specification<T, I>[ <[T]>::get::<I> ](slice: &[T], i: I) -> (b: Option<
    &<I as core::slice::SliceIndex<[T]>>::Output,
>) where I: core::slice::SliceIndex<[T]>
    returns
        spec_slice_get(slice, i),
;

} // verus!

fn main() { }
