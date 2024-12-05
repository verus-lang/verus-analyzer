use vstd::prelude::*;

verus! {

#[verus::line_count::ignore]
pub const A: u64 = 0;


/*
trait T { }

spec fn v<K>()
        where 
            K: T,
        recommends
            true,
{
()
}


global size_of usize == 4;

global size_of S == 8;

global size_of S<u64> == 8;

global size_of S<U> == 8;

global layout S is size == 8, align == 8;

global layout S<u64> is size == 16, align == 8;

global layout S<u32> is size == 8, align == 4;

proof fn tester(a: bool, b:bool)
    requires
        ({ 
            let x = a;
            ||| a == true
        }),
{
}

spec fn test(a: bool, b:bool) -> bool {
    let x = a;
    ||| a
    ||| b
}


fn test(a: u8) {
    assert(a & 0 == 0) by (bit_vector)
}


proof fn testp(a: bool, b:bool) 
    requires 
        ({
            ||| { a }
            ||| b
        }),
    ensures true,
{
}
spec fn test(a: bool, b:bool) -> bool {
    ||| {
        a
    }
    ||| b
}
*/

}


fn main() { }
