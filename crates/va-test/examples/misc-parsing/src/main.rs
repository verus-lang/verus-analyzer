use vstd::prelude::*;

verus! {

mod m5 {
broadcast use
    super::raw_ptr::group_raw_ptr_axioms,
    super::set_lib::group_set_lib_axioms,
    super::set::group_set_axioms,
;
broadcast use
    super::raw_ptr::group_raw_ptr_axioms,
    super::set_lib::group_set_lib_axioms,
    super::set::group_set_axioms;
broadcast use super::raw_ptr::group_raw_ptr_axioms;
broadcast use super::set_lib::group_set_lib_axioms;
broadcast use super::set::group_set_axioms;
broadcast use {
    super::raw_ptr::group_raw_ptr_axioms,
    super::set_lib::group_set_lib_axioms,
    super::set::group_set_axioms};
broadcast use {
    super::raw_ptr::group_raw_ptr_axioms,
    super::set_lib::group_set_lib_axioms,
    super::set::group_set_axioms,};
broadcast use {super::set::group_set_axioms};
broadcast use {super::set::group_set_axioms,};

}

} // verus!

fn main() { }
