//! A bit-set of `SyntaxKind`s.

use crate::SyntaxKind;

/// A bit-set of `SyntaxKind`s
#[derive(Clone, Copy)]
pub(crate) struct TokenSet(u128, u128);
//                         upper,lower
impl TokenSet {
    pub(crate) const EMPTY: TokenSet = TokenSet(0, 0);

    pub(crate) const fn new(kinds: &[SyntaxKind]) -> TokenSet {
        let mut upper = 0u128;
        let mut lower = 0u128;
        let mut i = 0;
        while i < kinds.len() {
            let (u, l) = mask(kinds[i]);
            upper |= u;
            lower |= l;
            i += 1;
        }
        TokenSet(upper, lower)
    }

    pub(crate) const fn union(self, other: TokenSet) -> TokenSet {
        TokenSet(self.0 | other.0, self.1 | other.1)
    }

    pub(crate) const fn contains(&self, kind: SyntaxKind) -> bool {
        let (upper, lower) = mask(kind);
        self.0 & upper != 0 || self.1 & lower != 0
    }
}

const fn mask(kind: SyntaxKind) -> (u128, u128) {
    let k = kind as usize;
    if k >= 128 {
        (1u128 << (k - 128) ,0u128)
    } else {
        (0, 1u128 << k)
    }
}

#[test]
fn token_set_works_for_tokens() {
    use crate::SyntaxKind::*;
    let ts = TokenSet::new(&[EOF, SHEBANG]);
    assert!(ts.contains(EOF));
    assert!(ts.contains(SHEBANG));
    // assert!(ts.contains(TYPE_ALIAS)); 
    assert!(!ts.contains(PLUS));
}
