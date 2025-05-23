//! Defines input for code generation process.

pub(crate) struct KindsSrc<'a> {
    pub(crate) punct: &'a [(&'a str, &'a str)],
    pub(crate) keywords: &'a [&'a str],
    pub(crate) contextual_keywords: &'a [&'a str],
    pub(crate) literals: &'a [&'a str],
    pub(crate) tokens: &'a [&'a str],
    pub(crate) nodes: &'a [&'a str],
}

pub(crate) const KINDS_SRC: KindsSrc<'_> = KindsSrc {
    punct: &[
        (";", "SEMICOLON"),
        (",", "COMMA"),
        ("(", "L_PAREN"),
        (")", "R_PAREN"),
        ("{", "L_CURLY"),
        ("}", "R_CURLY"),
        ("[", "L_BRACK"),
        ("]", "R_BRACK"),
        ("<", "L_ANGLE"),
        (">", "R_ANGLE"),
        ("@", "AT"),
        ("#", "POUND"),
        ("~", "TILDE"),
        ("?", "QUESTION"),
        ("$", "DOLLAR"),
        ("&", "AMP"),
        ("|", "PIPE"),
        ("+", "PLUS"),
        ("*", "STAR"),
        ("/", "SLASH"),
        ("^", "CARET"),
        ("%", "PERCENT"),
        ("_", "UNDERSCORE"),
        (".", "DOT"),
        ("..", "DOT2"),
        ("...", "DOT3"),
        ("..=", "DOT2EQ"),
        (":", "COLON"),
        ("::", "COLON2"),
        ("=", "EQ"),
        ("==", "EQ2"),
        ("=>", "FAT_ARROW"),
        ("!", "BANG"),
        ("!=", "NEQ"),
        ("-", "MINUS"),
        ("->", "THIN_ARROW"),
        ("<=", "LTEQ"),
        (">=", "GTEQ"),
        ("+=", "PLUSEQ"),
        ("-=", "MINUSEQ"),
        ("|=", "PIPEEQ"),
        ("&=", "AMPEQ"),
        ("^=", "CARETEQ"),
        ("/=", "SLASHEQ"),
        ("*=", "STAREQ"),
        ("%=", "PERCENTEQ"),
        ("&&", "AMP2"),
        ("||", "PIPE2"),
        ("<<", "SHL"),
        (">>", "SHR"),
        ("<<=", "SHLEQ"),
        (">>=", "SHREQ"),
        //verus
        ("&&&", "BIGAND"),
        ("|||", "BIGOR"),
        ("<==>", "EQUIV"),
        ("==>", "IMPLY"),
        ("<==", "EXPLY"),
        ("===", "EQEQEQ"),
        ("!==", "NEEQ"),
        ("=~=", "ExtEq"),
        ("!~=", "ExtNe"),
        ("=~~=", "ExtDeepEq"),
        ("!~~=", "ExtDeepNe"),
    ],
    keywords: &[
        "abstract",
        "as",
        "async",
        "await",
        "become",
        "box",
        "break",
        "const",
        "continue",
        "crate",
        "do",
        "dyn",
        "else",
        "enum",
        "extern",
        "false",
        "final",
        "fn",
        "for",
        "if",
        "impl",
        "in",
        "let",
        "loop",
        "macro",
        "match",
        "mod",
        "move",
        "mut",
        "override",
        "priv",
        "pub",
        "ref",
        "return",
        "self",
        "Self",
        "static",
        "struct",
        "super",
        "trait",
        "true",
        "try",
        "type",
        "typeof",
        "unsafe",
        "unsized",
        "use",
        "virtual",
        "where",
        "while",
        "yield",
        // verus keywords that currently *need* to be treated as 1st class keywords, based on the parser's current design
        "ghost",
        "tracked",
        "forall",
        "exists",
        "is",
        "has",
        "matches",
    ],
    contextual_keywords: &[
        "auto",
        "builtin",
        "default",
        "existential",
        "union",
        "raw",
        "macro_rules",
        "yeet",
        "offset_of",
        "asm",
        "format_args",
        // Most Verus keywords need to be conditional, since we otherwise preclude them from being used as function names, variables, etc.
        "verus",
        "group",    // verus: Only a key word when it follows 'broadcast'
        "any",      // verus: Only a key word when following 'open_invariants'
        "none",     // verus: Only a key word when following 'open_invariants'
        "no_unwind",// verus: Only a key word at the end of a function signature
        "requires",
        "ensures",
        "default_ensures",
        "returns",
        "checked",
        "recommends",
        "decreases",
        "invariant_except_break",
        "invariant",
        "assert",
        "assume",
        "choose",
        "implies",
        "exec",
        "spec",
        "proof",
        "axiom",
        "proof_fn",
        "by",
        "via",
        "when",
        "trigger",
        "global",
        "broadcast",
        "assume_specification",
        "open",
        "closed",
        "uninterp",
        "opens_invariants",
        // global-related keywords
        "size_of",
        "layout",
        "size",
        "align",
    ],
    literals: &["INT_NUMBER", "FLOAT_NUMBER", "CHAR", "BYTE", "STRING", "BYTE_STRING", "C_STRING"],
    tokens: &["ERROR", "IDENT", "WHITESPACE", "LIFETIME_IDENT", "COMMENT", "SHEBANG"],
    nodes: &[
        "SOURCE_FILE",
        "STRUCT",
        "UNION",
        "ENUM",
        "FN",
        "RET_TYPE",
        "EXTERN_CRATE",
        "MODULE",
        "USE",
        "STATIC",
        "CONST",
        "TRAIT",
        "TRAIT_ALIAS",
        "IMPL",
        "TYPE_ALIAS",
        "MACRO_CALL",
        "MACRO_RULES",
        "MACRO_ARM",
        "TOKEN_TREE",
        "MACRO_DEF",
        "PAREN_TYPE",
        "TUPLE_TYPE",
        "MACRO_TYPE",
        "NEVER_TYPE",
        "PATH_TYPE",
        "PTR_TYPE",
        "ARRAY_TYPE",
        "SLICE_TYPE",
        "REF_TYPE",
        "INFER_TYPE",
        "FN_PTR_TYPE",
        "FOR_TYPE",
        "IMPL_TRAIT_TYPE",
        "DYN_TRAIT_TYPE",
        "OR_PAT",
        "PAREN_PAT",
        "REF_PAT",
        "BOX_PAT",
        "IDENT_PAT",
        "WILDCARD_PAT",
        "REST_PAT",
        "PATH_PAT",
        "RECORD_PAT",
        "RECORD_PAT_FIELD_LIST",
        "RECORD_PAT_FIELD",
        "TUPLE_STRUCT_PAT",
        "TUPLE_PAT",
        "SLICE_PAT",
        "RANGE_PAT",
        "LITERAL_PAT",
        "MACRO_PAT",
        "CONST_BLOCK_PAT",
        // atoms
        "TUPLE_EXPR",
        "ARRAY_EXPR",
        "PAREN_EXPR",
        "PATH_EXPR",
        "CLOSURE_EXPR",
        "IF_EXPR",
        "WHILE_EXPR",
        "LOOP_EXPR",
        "FOR_EXPR",
        "CONTINUE_EXPR",
        "BREAK_EXPR",
        "LABEL",
        "BLOCK_EXPR",
        "STMT_LIST",
        "RETURN_EXPR",
        "BECOME_EXPR",
        "YIELD_EXPR",
        "YEET_EXPR",
        "LET_EXPR",
        "UNDERSCORE_EXPR",
        "MACRO_EXPR",
        "MATCH_EXPR",
        "MATCH_ARM_LIST",
        "MATCH_ARM",
        "MATCH_GUARD",
        "RECORD_EXPR",
        "RECORD_EXPR_FIELD_LIST",
        "RECORD_EXPR_FIELD",
        "OFFSET_OF_EXPR",
        "ASM_EXPR",
        "FORMAT_ARGS_EXPR",
        "FORMAT_ARGS_ARG",
        // postfix
        "CALL_EXPR",
        "INDEX_EXPR",
        "METHOD_CALL_EXPR",
        "FIELD_EXPR",
        "AWAIT_EXPR",
        "TRY_EXPR",
        "CAST_EXPR",
        // unary
        "REF_EXPR",
        "PREFIX_EXPR",
        "RANGE_EXPR", // just weird
        "BIN_EXPR",
        "EXTERN_BLOCK",
        "EXTERN_ITEM_LIST",
        "VARIANT",
        "RECORD_FIELD_LIST",
        "RECORD_FIELD",
        "TUPLE_FIELD_LIST",
        "TUPLE_FIELD",
        "VARIANT_LIST",
        "ITEM_LIST",
        "ASSOC_ITEM_LIST",
        "ATTR",
        "META",
        "USE_TREE",
        "USE_TREE_LIST",
        "PATH",
        "PATH_SEGMENT",
        "LITERAL",
        "RENAME",
        "VISIBILITY",
        "WHERE_CLAUSE",
        "WHERE_PRED",
        "ABI",
        "NAME",
        "NAME_REF",
        "LET_STMT",
        "LET_ELSE",
        "EXPR_STMT",
        "GENERIC_PARAM_LIST",
        "GENERIC_PARAM",
        "LIFETIME_PARAM",
        "TYPE_PARAM",
        "RETURN_TYPE_ARG",
        "CONST_PARAM",
        "GENERIC_ARG_LIST",
        "LIFETIME",
        "LIFETIME_ARG",
        "TYPE_ARG",
        "ASSOC_TYPE_ARG",
        "CONST_ARG",
        "PARAM_LIST",
        "PARAM",
        "SELF_PARAM",
        "ARG_LIST",
        "TYPE_BOUND",
        "TYPE_BOUND_LIST",
        // macro related
        "MACRO_ITEMS",
        "MACRO_STMTS",
        "MACRO_EAGER_INPUT",
        // verus
        "REQUIRES_CLAUSE",
        "DEFAULT_ENSURES_CLAUSE",
        "ENSURES_CLAUSE",
        "RETURNS_CLAUSE",
        "DECREASES_CLAUSE",
        "RECOMMENDS_CLAUSE",
        "OPENS_INVARIANTS_CLAUSE",
        "NO_UNWIND_CLAUSE",
        "LOOP_CLAUSE",
        "INVARIANT_EXCEPT_BREAK_CLAUSE",
        "INVARIANT_CLAUSE",
        "ASSERT_EXPR",
        "ASSERT_FORALL_EXPR",
        "ASSUME_EXPR",
        "VIEW_EXPR",
        "PUBLISH",
        "FN_MODE",
        "DATA_MODE",
        "MODE_SPEC_CHECKED",
        "PROOF_FN_TYPE",
        "PROOF_FN_CHARACTERISTICS",
        "PROOF_FN_WITH_CHARACTERISTICS",
        "PROVER",
        "SIGNATURE_DECREASES",
        "TRIGGER_ATTRIBUTE",
        "VERUS_GLOBAL",
        "BROADCAST_USE_LIST",
        "BROADCAST_USE",
        "BROADCAST_GROUP_IDENTIFIER",
        "BROADCAST_GROUP_MEMBER",
        "BROADCAST_GROUP_LIST",
        "BROADCAST_GROUP",
        "ASSUME_SPECIFICATION",
        "IS_EXPR",
        "HAS_EXPR",
        "ARROW_EXPR",
        "MATCHES_EXPR",
    ],
};

#[derive(Default, Debug)]
pub(crate) struct AstSrc {
    pub(crate) tokens: Vec<String>,
    pub(crate) nodes: Vec<AstNodeSrc>,
    pub(crate) enums: Vec<AstEnumSrc>,
}

#[derive(Debug)]
pub(crate) struct AstNodeSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) fields: Vec<Field>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Field {
    Token { name: String, cardinality: Cardinality },
    Node { name: String, ty: String, cardinality: Cardinality },
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Cardinality {
    Optional,
    Many,
    One,
}

#[derive(Debug)]
pub(crate) struct AstEnumSrc {
    pub(crate) doc: Vec<String>,
    pub(crate) name: String,
    pub(crate) traits: Vec<String>,
    pub(crate) variants: Vec<String>,
}
