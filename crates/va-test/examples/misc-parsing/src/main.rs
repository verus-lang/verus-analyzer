use vstd::prelude::*;

verus! {

trait T1 {
    proof fn my_function_decl(&self, i: int, j: int) -> (r: int)
        requires
            0 <= i < 10,
            0 <= j < 10,
        ensures
            i <= r,
            j <= r,
    ;

    /// A trait function may have a default (provided) implementation,
    /// and this defaults may have additional ensures specified with default_ensures
    fn my_function_with_a_default(&self, i: u32, j: u32) -> (r: u32)
        requires
            0 <= i < 10,
            0 <= j < 10,
        ensures
            i <= r,
            j <= r,
        default_ensures
            i == r || j == r,
        {
            if i >= j { i } else { j }
        }
}

trait T2 {
    fn f(i: u32) -> (r: u32)
        requires
            (builtin::default_ensures)(true),
        default_ensures
            r <= i,
    {
        i / 2
    }
}

trait T3 {
    fn f(i: u32) -> (r: u32)
        default_ensures
            r <= i,
    {
        i / 2
    }
}

} // verus!

fn main() { }
