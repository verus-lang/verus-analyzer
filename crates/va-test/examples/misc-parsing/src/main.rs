use vstd::prelude::*;

verus! {

/*
pub assume_specification<T> [core::mem::swap::<T>] (a: &mut T, b: &mut T)
    ensures
        *a == *old(b),
        *b == *old(a),
    opens_invariants none
    no_unwind;

pub assume_specification<T>[Vec::<T>::new]() -> (v: Vec<T>)
    ensures
        v@ == Seq::<T>::empty();

pub assume_specification<T, A: Allocator>[Vec::<T, A>::clear](vec: &mut Vec<T, A>)
    ensures
        vec@ == Seq::<T>::empty();
*/

pub assume_specification [<bool as Clone>::clone](b: &bool) -> (res: bool)
    ensures res == b;
} // verus!

fn main() { }
