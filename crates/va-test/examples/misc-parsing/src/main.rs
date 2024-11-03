verus! {

pub fn write(in_v: V) where V: Copy
    requires
        old(perm).pptr() == self,
    ensures
        perm.pptr() === old(perm).pptr(),
        perm.mem_contents() === MemContents::Init(in_v),
    opens_invariants none
    no_unwind
{
}

} // verus!

fn main() { }
