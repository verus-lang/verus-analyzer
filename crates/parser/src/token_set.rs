//! A bit-set of `SyntaxKind`s.

use crate::SyntaxKind;

const TOKEN_SET_WORDS: usize = (SyntaxKind::SHEBANG as usize / 64) + 1;

/// A bit-set of `SyntaxKind`s
#[derive(Clone, Copy)]
pub(crate) struct TokenSet([u64; TOKEN_SET_WORDS]);

/// `TokenSet`s should only include token `SyntaxKind`s, so the discriminant of any passed/included
/// `SyntaxKind` must *not* be greater than that of the last token `SyntaxKind`.
/// See #17037.
const LAST_TOKEN_KIND_DISCRIMINANT: usize = SyntaxKind::SHEBANG as usize;

impl TokenSet {
    pub(crate) const EMPTY: TokenSet = TokenSet([0; TOKEN_SET_WORDS]);

    pub(crate) const fn new(kinds: &[SyntaxKind]) -> TokenSet {
        let mut res = [0; TOKEN_SET_WORDS];
        let mut i = 0;
        while i < kinds.len() {
            let discriminant = kinds[i] as usize;
            debug_assert!(
                discriminant <= LAST_TOKEN_KIND_DISCRIMINANT,
                "Expected a token `SyntaxKind`"
            );
            let idx = discriminant / 64;
            res[idx] |= 1 << (discriminant % 64);
            i += 1;
        }
        TokenSet(res)
    }

    pub(crate) const fn union(self, other: TokenSet) -> TokenSet {
        let mut result = [0; TOKEN_SET_WORDS];
        let mut index = 0;
        while index < TOKEN_SET_WORDS {
            result[index] = self.0[index] | other.0[index];
            index += 1;
        }
        TokenSet(result)
    }

    pub(crate) const fn contains(&self, kind: SyntaxKind) -> bool {
        let discriminant = kind as usize;
        debug_assert!(
            discriminant <= LAST_TOKEN_KIND_DISCRIMINANT,
            "Expected a token `SyntaxKind`"
        );
        let idx = discriminant / 64;
        let mask = 1 << (discriminant % 64);
        self.0[idx] & mask != 0
    }
}

#[test]
fn token_set_works_for_tokens() {
    use crate::SyntaxKind::*;
    let ts = TokenSet::new(&[EOF, SHEBANG]);
    assert!(ts.contains(EOF));
    assert!(ts.contains(SHEBANG));
    assert!(!ts.contains(PLUS));
}
