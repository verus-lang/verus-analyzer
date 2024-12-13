#![allow(non_snake_case)]
#![allow(dead_code)]
//use vstd::prelude::*;
use storage_macros::Hello;


//verus! {

#[derive(Hello)]
struct TestKey {
    val: u64,
}

//}

fn main() { 
    say_hello_TestKey();
}
