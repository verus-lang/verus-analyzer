use vstd::prelude::*;

verus! {

proof fn tester(a: bool, b:bool)
    requires
        ({ 
            let x = a;
            ||| a == true
        }),
{
}

/*
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
