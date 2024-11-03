verus! {

//fn test(s: Set<int>) {
//    let x = s.choose();
//    choose|x: int| f1(x)
//}
//

fn simple() {
    let x_witness = choose|x: int| f1(x);
}
/*
fn test_choose() {
    assume(exists|x: int| f1(x) == 10);
    proof {
        let x_witness = choose|x: int| f1(x) == 10;
        assert(f1(x_witness) == 10);
    }

    assume(exists|x: int, y: int| f1(x) + f1(y) == 30);
    proof {
        let (x_witness, y_witness): (int, int) = choose|x: int, y: int| f1(x) + f1(y) == 30;
        assert(f1(x_witness) + f1(y_witness) == 30);
    }
}
*/


} // verus!

fn main() { }
