use vstd::prelude::*;

verus! {

//spec fn test(a: bool, b:bool) -> bool {
//    ||| {
//        a
//    }
//    ||| b
//}
proof fn tester()
    requires
        ({ 
            let x = a;
            &&& a
        }),
{
}
//proof fn testp(a: bool, b:bool) 
//    requires 
//        ({
//            ||| { a }
//            ||| b
//        }),
//    ensures true,
//{
//}

} // verus!

fn main() { }
