use vstd::prelude::*;

verus! {
/*
proof fn testp(a: bool, b:bool) 
    requires 
        ({
            ||| { a }
            ||| b
        }),
    ensures true,
{
}
*/
spec fn test(a: bool, b:bool) -> bool {
    ||| {
        a
    }
    ||| b
//    ||| {
//        a
//    }
//    ||| a
//    ||| b
//    ({ a }) || b
//    ||| { a } ||| b
//    { a } ||| b
//    { a } || b
}

}


fn main() { }
