verus! {

fn test() {
//    if exists|i: int| 0 <= i {
//        let x = 1;
//    }
    if exists|i: int| 0 <= start <= i < stop <= s.len() && s[i] == x {
        let index = choose|i: int| 0 <= start <= i < stop <= s.len() && s[i] == x;
        assert(s.subrange(start, stop)[index - start] == s[index]);
    }
}

} // verus!

fn main() { }
