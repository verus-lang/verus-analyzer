use vstd::prelude::*;

verus! {

fn uses_spec_has()
    requires
        s has 3,
        ms has 4,
{
    assert(s has 3);
    assert(s has 3 == true);
    assert(s has 3 == s has 3);
    assert(ms has 4);
    assert(ms has 4 == ms has 4);
}

/*
pub open spec fn ptr_null_mut<T: ?Sized + core::ptr::Pointee<Metadata = ()>>() -> *mut T {
    ptr_mut_from_data(PtrData { addr: 0, provenance: Provenance::null(), metadata: Metadata::Thin })
}

pub open spec fn ptr_null_mut<T: ?Sized + core::ptr::Pointee<Metadata = ()>>() -> *mut T {
    ptr_mut_from_data(PtrData { addr: 0, provenance: Provenance::null(), metadata: Metadata::Thin })
}
*/

} // verus!

fn main() { }
