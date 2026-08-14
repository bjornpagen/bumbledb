//! The `query!` proc-macro — the blessed Rust query sugar, downstream and
//! quarantined (docs/architecture/70-api.md § host-side sugar): hosts may
//! depend on this crate, the engine never depends back, and the engine's
//! own surface stays pure-data IR (docs/architecture/20-query-ir.md, the
//! surface ruling). The notation is the statement grammar's query side,
//! promoted (docs/architecture/20-query-ir.md § the query notation —
//! the normative grammar block; `ir::render` emits it, this macro parses
//! it, round-trip goldens pin the two together):
//!
//! ```text
//! query     := interior* recblock? main
//! interior  := 'interior' pred '(' head ')' '|' body ';'
//! recblock  := 'recursive' pred '(' head ')' '|' body ';'
//! main      := barerule+
//! barerule  := '(' head ')' '|' body ';'
//!                                        // consecutive `interior pred(...)` lines
//!                                        //   union into one Interior; consecutive
//!                                        //   `recursive pred(...)` lines union into
//!                                        //   one Rec (a line whose body has an atom
//!                                        //   naming pred is a rec arm, else base);
//!                                        //   an all-bare query is Query with empty
//!                                        //   interiors and rec: None — text-level
//!                                        //   backward compatibility; a named head
//!                                        //   without `interior` / `recursive` is a
//!                                        //   compile error (the former named-head sneak)
//! head    := headterm (',' headterm)*
//! headterm:= var | [name ':'] agg        // named positions become result columns
//! agg     := Sum(t) | Min(t) | Max(t) | Count | Pack(v)
//!            where t := v | Duration(v)
//! body    := item (',' item)*
//! item    := atom                        // positive occurrence
//!          | '!' atom                    // negation (anti-probe; safety per roster)
//!          | cond                        // a condition tree; the list is a conjunction
//! cond    := term 'in' term              // membership: point ∈ interval
//!          | Allen '(' term ',' mask ',' term ')'
//!          | term cmp term               // ==  !=  <  <=  >  >=
//!          | 'and' '(' cond (',' cond)* ')'  // ConditionTree::And — comparison
//!          | 'or'  '(' cond (',' cond)* ')'  //   leaves only (ruled 2026-07-23, R9)
//! atom    := Relation '(' binding (',' binding)* ')'
//!          | pred '(' var (',' var)* ')'
//!                                        // ordered dense: a body atom may name an
//!                                        //   interior or the rec where it names a
//!                                        //   relation; bare idents bind its head
//!                                        //   POSITIONS left to right from 0 —
//!                                        //   positional, never nominal
//!          | pred '(' pbind (',' pbind)* ')'
//!                                        // indexed: the sparse/selection forms;
//!                                        //   never mixed with the bare form, and an
//!                                        //   explicit dense in-order `i: v` list is
//!                                        //   refused — the ordered form is the one
//!                                        //   dense spelling
//! binding := field                       // punning: binds a var named after the field
//!          | field ':' var               // explicit variable — the join spelling
//!          | field '==' value            // selection, schema-grammar-verbatim
//!          | field 'in' ?param           // set membership: field value ∈ the bound set
//! pbind   := position ':' var            // sparse explicit position
//!          | position '==' value         // position selection
//!          | position 'in' ?param        // position set membership
//! mask    := MASK ('|' MASK)*            // literal sets of basics; '|' is set union
//! term    := var | ?param | literal
//! pred    := lowercase ident             // macro-LOCAL: resolved at expansion, never
//!                                        //   in the IR or the fingerprint; relations
//!                                        //   are UpperCamel, so an interior/rec spelled
//!                                        //   like a relation is unwritable; `and`,
//!                                        //   `or`, `interior`, and `recursive` are
//!                                        //   reserved
//! ```
//!
//! **Condition trees are notation (ruled 2026-07-23, R9):** `and(...)`/
//! `or(...)` admit any boolean combination of comparisons as one item —
//! comparison leaves only, exactly the IR's `ConditionTree` (atoms,
//! negation, and the binding membership stay items) and an exact mirror
//! of the TS condition grammar. Validation distributes the trees to DNF
//! engine-side; the renderer's functional forms are grammar, so the
//! render→parse round trip closes over the full input grammar.
//!
//! Surface `Duration(iv)` lowers to IR `Measure(iv)`: it denotes the
//! point-set size `end − start`, and a ray has no measure.
//!
//! **Punning law (B, decided; the alternative is ledgered in
//! docs/architecture/70-api.md):** a bare field name binds a **rule-local variable
//! named after the field** — projection shorthand, Rust's struct-shorthand
//! instinct. The same punned name in two atoms of one rule is a compile
//! error, spanned at the second occurrence ("ambiguous punning — rename
//! explicitly"); joins are always written `field: v` on both ends.
//!
//! **Name checking without schema visibility — the id-constants trick.**
//! Proc macros cannot see each other's output, so `query!` cannot read the
//! theory. It does not need to: expansion emits paths to the `schema!`
//! macro's declaration-order id constants (`Calendar::BUSY`,
//! `Calendar::BUSY_PERSON`), and ordinary rustc name resolution does the
//! checking — a typo'd relation or field is a compile error at the query
//! literal. Mask names resolve the same way (`AllenMask::INTERSECTS` —
//! the 13 basics and the workload composites). Variable *type*
//! consistency stays the validation roster's (prepare-time, typed,
//! rendered) — the same split the foreign surfaces have.
//!
//! **Constant text only.** The macro consumes a literal token tree;
//! dynamic composition stays on the raw IR layer, which exists regardless
//! — text for the static 90%, data for the dynamic tail, both lowering to
//! the same IR. Expansion is compile-time lowering: the emitted code
//! constructs the `ir::Query` value directly; no runtime parser exists.
//!
//! Notation corners the schema cannot disambiguate for the macro (each a
//! consequence of "the macro cannot see the theory"):
//!
//! - **Integer literals** type by their own spelling: a bare unsigned
//!   integer is `u64`, a negative one `i64`, and Rust's `u64`/`i64`
//!   suffixes force the choice (`5i64` against an `i64` field). Interval
//!   literals `start..end` follow the same rule over both bounds. The
//!   magnitude is rustc's (ruled 2026-07-23, R8): an optional
//!   `0x`/`0o`/`0b` radix prefix and `_` separators, uniformly here and
//!   in the schema grammar; the renderer normalizes to canonical decimal,
//!   so the round-trip law is canonical-form, not verbatim.
//! - **Handle selection values**: a bare handle name (`kind == Focus`)
//!   resolves through the field-named host enum's welded row id
//!   (`Kind::Focus.id()`) — exact when the closed relation is named
//!   after its referencing field; one named otherwise is written
//!   qualified (`arm == ClaimKind::Busy` → `ClaimKind::Busy.id()`). The
//!   emitted `const fn id` yields the handle newtype, whose `.0` is a
//!   plain `u64` constant, and rustc polices the path.
//! - **Params** are one style per query: named (`?window`, dense ids by
//!   first occurrence, query-global) or positional (`?0`, the id
//!   verbatim — the renderer's own spelling, so rendered output reparses).
//! - **Item-position `in`** is point membership (`PointIn`): the right
//!   side is the interval — a variable, a `?param`, or a `start..end`
//!   literal. Set membership is the binding form `field in ?param`.
//!
//! Diagnostics carry spans: every parse error points at its token, and
//! the punning error points at the second occurrence. The refused Datalog
//! grammar (`head :- body`) does not parse, anywhere, by design — the
//! statement surface's query side is the one notational family.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use std::fmt::Write as _;
use std::iter::Peekable;

type Tokens = Peekable<proc_macro::token_stream::IntoIter>;

/// One spanned diagnostic; rendered as a `compile_error!` at the span.
struct Error {
    span: Span,
    message: String,
}

type Parse<T> = Result<T, Error>;

fn fail<T>(span: Span, message: impl Into<String>) -> Parse<T> {
    Err(Error {
        span,
        message: message.into(),
    })
}

/// The Datalog refusal, one message everywhere `:-` could be attempted.
fn datalog_refusal<T>(span: Span) -> Parse<T> {
    fail(
        span,
        "query!: `:-` is refused (borrowed Datalog grammar) — the notation is the \
         statement grammar's query side, promoted: write `(head) | body;` \
         (docs/architecture/20-query-ir.md § the query notation)",
    )
}

/// `compile_error!("message")`, every token spanned at the offense so
/// rustc points at the exact query token.
fn compile_error(error: &Error) -> TokenStream {
    let mut message = Literal::string(&error.message);
    message.set_span(error.span);
    let mut bang = Punct::new('!', Spacing::Alone);
    bang.set_span(error.span);
    let mut body = Group::new(
        Delimiter::Brace,
        std::iter::once(TokenTree::Literal(message)).collect(),
    );
    body.set_span(error.span);
    [
        TokenTree::Ident(Ident::new("compile_error", error.span)),
        TokenTree::Punct(bang),
        TokenTree::Group(body),
    ]
    .into_iter()
    .collect()
}

// ---------------------------------------------------------------------
// The surface AST — names and spans, resolved to ids after the parse.
// ---------------------------------------------------------------------

/// A source name with the span diagnostics point at.
#[derive(Clone)]
struct Name {
    text: String,
    span: Span,
}

/// One `?param`: named (dense ids by first occurrence) or positional
/// (the id verbatim — the renderer's spelling). Both spellings carry
/// their token's span: every refusal points at the offending param.
enum Param {
    Named(Name),
    Index { index: u16, span: Span },
}

/// A signed-integer literal's raw token text (suffix included; spliced
/// verbatim so rustc polices range and form) plus what its spelling
/// forces: `signed` when negative or `i64`-suffixed.
struct Int {
    negative: bool,
    text: String,
    signed: bool,
}

/// One literal, classified by its own syntax (the macro cannot see the
/// field's declared type; the spelling decides).
enum Lit {
    Bool(bool),
    Int(Int),
    /// `start..end`, half-open; signed when either bound is.
    Interval {
        start: Int,
        end: Int,
    },
    /// A string literal's raw token text, quotes included.
    Str(String),
    /// A byte-string literal's raw token text.
    Bytes(String),
}

/// One selection value (the right side of a binding's `==`).
enum SelValue {
    Lit(Lit),
    Param(Param),
    /// A closed relation's handle: bare (`Focus`) or qualified
    /// (`Kind::Focus`) — either way the host enum's welded row id.
    Handle {
        qualifier: Option<Name>,
        handle: Name,
    },
}

/// One term of a comparison, membership, or `Allen` position.
enum Term {
    Var(Name),
    Param(Param),
    Measure(Name),
    Lit(Lit),
}

/// One atom binding, per the grammar's four spellings.
enum Binding {
    Pun(Name),
    Var { field: Name, var: Name },
    Value { field: Name, value: SelValue },
    SetParam { field: Name, param: Param },
}

struct Atom {
    relation: Name,
    bindings: Vec<Binding>,
}

/// The `Allen` mask position: named masks joined by `|`.
enum Mask {
    Names(Vec<Name>),
}

/// One comparison — the condition grammar's leaf vocabulary (every
/// `CmpOp`, the TS mirror's own leaf set).
enum Leaf {
    Allen {
        lhs: Term,
        mask: Mask,
        rhs: Term,
    },
    /// `element in container` — point membership, container-side interval.
    Membership {
        element: Term,
        container: Term,
    },
    Cmp {
        op: &'static str,
        lhs: Term,
        rhs: Term,
    },
}

/// One condition tree (ruled 2026-07-23, R9): `and`/`or` over comparison
/// leaves — the IR's `ConditionTree`, spelled. Atoms, negation, and the
/// binding membership stay items; validation distributes any nested `Or`
/// to DNF engine-side.
enum Cond {
    Leaf(Leaf),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

/// One body item, in source order.
enum Item {
    Atom(Atom),
    Negated(Atom),
    Cond(Cond),
}

/// The aggregate ops admitted by both the head grammar and the IR renderer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AggOp {
    Sum,
    Min,
    Max,
    Count,
    Pack,
}

impl AggOp {
    fn ir_name(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count => "Count",
            Self::Pack => "Pack",
        }
    }
}

/// One head term. A named position (`total: Sum(x)`) keeps the name at
/// the call site only — result columns are positional in the IR, and
/// variable names are a debugging sidecar the engine never stores.
enum HeadTerm {
    Var(Name),
    Measure(Name),
    Count,
    Agg { op: AggOp, over: Name },
    AggMeasure { op: AggOp, over: Name },
}

/// One parsed rule: the keyword (or bare main) carries the name.
enum ParsedRule {
    Bare { head: Vec<HeadTerm>, items: Vec<Item> },
    Interior { name: Name, head: Vec<HeadTerm>, items: Vec<Item> },
    Recursive { name: Name, head: Vec<HeadTerm>, items: Vec<Item> },
}

impl ParsedRule {
    fn head(&self) -> &[HeadTerm] {
        match self {
            Self::Bare { head, .. } | Self::Interior { head, .. } | Self::Recursive { head, .. } => {
                head
            }
        }
    }

    fn items(&self) -> &[Item] {
        match self {
            Self::Bare { items, .. }
            | Self::Interior { items, .. }
            | Self::Recursive { items, .. } => items,
        }
    }
}

// ---------------------------------------------------------------------
// The cursor helpers (bumbledb-macros' precedent, Result-shaped: the
// punning law demands spanned diagnostics, so no helper panics).
// ---------------------------------------------------------------------

fn peek_span(tokens: &mut Tokens) -> Span {
    tokens.peek().map_or_else(Span::call_site, TokenTree::span)
}

fn expect_ident(tokens: &mut Tokens, what: &str) -> Parse<Name> {
    match tokens.next() {
        Some(TokenTree::Ident(ident)) => Ok(Name {
            text: ident.to_string(),
            span: ident.span(),
        }),
        Some(other) => fail(
            other.span(),
            format!("query!: expected {what}, found `{other}`"),
        ),
        None => fail(Span::call_site(), format!("query!: expected {what}")),
    }
}

fn peek_punct(tokens: &mut Tokens, ch: char) -> bool {
    matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
}

fn peek_ident_text(tokens: &mut Tokens) -> Option<String> {
    match tokens.peek() {
        Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
        _ => None,
    }
}

fn expect_punct(tokens: &mut Tokens, ch: char, what: &str) -> Parse<Span> {
    match tokens.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == ch => Ok(p.span()),
        Some(other) => fail(
            other.span(),
            format!("query!: expected {what}, found `{other}`"),
        ),
        None => fail(Span::call_site(), format!("query!: expected {what}")),
    }
}

/// Consumes `:` while refusing `:-` — the borrowed grammar must not
/// parse, anywhere.
fn expect_colon(tokens: &mut Tokens, what: &str) -> Parse<()> {
    let span = expect_punct(tokens, ':', what)?;
    if peek_punct(tokens, '-') {
        return datalog_refusal(span);
    }
    Ok(())
}

fn take_paren_group(tokens: &mut Tokens, what: &str) -> Parse<(Tokens, Span)> {
    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            Ok((group.stream().into_iter().peekable(), group.span()))
        }
        Some(other) => fail(
            other.span(),
            format!("query!: expected {what}, found `{other}`"),
        ),
        None => fail(Span::call_site(), format!("query!: expected {what}")),
    }
}

/// The integer-literal token shape: digits first, no float dot
/// (`bumbledb-macros`' rule; a `u64`/`i64` suffix rides in the text).
fn is_int_text(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_digit()) && !text.contains('.')
}

/// The radix law (ruled 2026-07-23, R8): an integer magnitude is what
/// rustc lexes — an optional `0x`/`0o`/`0b` prefix and `_` separators —
/// uniformly at every integer position of both macro grammars
/// (`bumbledb-macros`' `int_magnitude` is the schema twin). The type
/// suffix is stripped before this check, so no branch order can invert
/// the grammar.
fn is_int_magnitude(text: &str) -> bool {
    let (radix, digits) = match text.as_bytes() {
        [b'0', b'x', ..] => (16, &text[2..]),
        [b'0', b'o', ..] => (8, &text[2..]),
        [b'0', b'b', ..] => (2, &text[2..]),
        _ => (10, text),
    };
    digits.chars().any(|c| c != '_') && digits.chars().all(|c| c == '_' || c.is_digit(radix))
}

/// Parses one `[-] int`, classifying by spelling: negative or
/// `i64`-suffixed is signed; a `u64` suffix (or none) is unsigned. The
/// suffix is stripped first, then one magnitude rule judges the rest —
/// the value sum holds exactly two integer types, and every radix
/// spelling rustc lexes is notation (the renderer normalizes to
/// canonical decimal; the round-trip law is canonical-form).
fn parse_int(tokens: &mut Tokens, what: &str) -> Parse<Int> {
    let negative = peek_punct(tokens, '-');
    if negative {
        tokens.next();
    }
    match tokens.next() {
        Some(TokenTree::Literal(lit)) => {
            let text = lit.to_string();
            if !is_int_text(&text) {
                return fail(
                    lit.span(),
                    format!("query!: expected {what}, found `{text}`"),
                );
            }
            let (magnitude, suffix_signed) = if let Some(stripped) = text.strip_suffix("i64") {
                (stripped, Some(true))
            } else if let Some(stripped) = text.strip_suffix("u64") {
                (stripped, Some(false))
            } else {
                (text.as_str(), None)
            };
            if suffix_signed == Some(false) && negative {
                return fail(lit.span(), "query!: a negative literal cannot carry `u64`");
            }
            if !is_int_magnitude(magnitude) {
                return fail(
                    lit.span(),
                    format!(
                        "query!: `{text}` is not an integer literal — the value types \
                         are u64 and i64, spelled with an optional 0x/0o/0b radix, \
                         `_` separators, and the u64/i64 suffixes"
                    ),
                );
            }
            Ok(Int {
                negative,
                text,
                signed: suffix_signed.unwrap_or(negative),
            })
        }
        Some(other) => fail(
            other.span(),
            format!("query!: expected {what}, found `{other}`"),
        ),
        None => fail(Span::call_site(), format!("query!: expected {what}")),
    }
}

/// An integer already begun: a scalar, or on `..` the start of a
/// half-open `start..end` interval literal.
fn finish_int(tokens: &mut Tokens, start: Int) -> Parse<Lit> {
    if peek_punct(tokens, '.') {
        tokens.next();
        expect_punct(tokens, '.', "the interval literal's `..`")?;
        let end = parse_int(tokens, "the interval literal's end bound")?;
        return Ok(Lit::Interval { start, end });
    }
    Ok(Lit::Int(start))
}

/// Parses a literal whose first token is a `Literal` or a leading `-`.
fn parse_lit(tokens: &mut Tokens) -> Parse<Lit> {
    if peek_punct(tokens, '-') {
        let start = parse_int(tokens, "an integer literal")?;
        return finish_int(tokens, start);
    }
    match tokens.peek() {
        Some(TokenTree::Literal(_)) => {
            let Some(TokenTree::Literal(lit)) = tokens.next() else {
                unreachable!("peeked a literal");
            };
            let text = lit.to_string();
            if text.starts_with('"') {
                Ok(Lit::Str(text))
            } else if text.starts_with("b\"") {
                Ok(Lit::Bytes(text))
            } else if is_int_text(&text) {
                // Re-classify through the int rule (suffix policing).
                let mut rewound: Tokens = std::iter::once(TokenTree::Literal(lit))
                    .collect::<TokenStream>()
                    .into_iter()
                    .peekable();
                let start = parse_int(&mut rewound, "an integer literal")?;
                finish_int(tokens, start)
            } else {
                fail(lit.span(), format!("query!: unsupported literal `{text}`"))
            }
        }
        Some(other) => fail(
            other.span(),
            format!("query!: expected a literal, found `{other}`"),
        ),
        None => fail(Span::call_site(), "query!: expected a literal"),
    }
}

/// Parses one `?param` (the `?` already consumed): a name or a
/// positional index — the renderer's own `?N` spelling.
fn parse_param(tokens: &mut Tokens, question: Span) -> Parse<Param> {
    match tokens.peek() {
        Some(TokenTree::Ident(_)) => Ok(Param::Named(expect_ident(tokens, "a param name")?)),
        Some(TokenTree::Literal(lit)) => {
            let text = lit.to_string();
            let span = lit.span();
            let Ok(index) = text.parse::<u16>() else {
                return fail(
                    span,
                    format!("query!: `?{text}` is not a param name or index"),
                );
            };
            tokens.next();
            Ok(Param::Index { index, span })
        }
        _ => fail(question, "query!: `?` starts a param — `?name` or `?N`"),
    }
}

// ---------------------------------------------------------------------
// The grammar, production by production.
// ---------------------------------------------------------------------

const AGG_NAMES: [(&str, AggOp); 5] = [
    ("Sum", AggOp::Sum),
    ("Min", AggOp::Min),
    ("Max", AggOp::Max),
    ("Count", AggOp::Count),
    ("Pack", AggOp::Pack),
];

fn agg_op(name: &str) -> Option<AggOp> {
    AGG_NAMES
        .iter()
        .find(|(text, _)| *text == name)
        .map(|(_, op)| *op)
}

/// Parses one aggregate's argument group: `(v)` for every unary op,
/// `(Duration(v))` admitted under `Sum`/`Min`/`Max` only (the measure's
/// three folds — the grammar's `t := v | Duration(v)`).
fn parse_agg(tokens: &mut Tokens, op: AggOp) -> Parse<HeadTerm> {
    if op == AggOp::Count {
        return Ok(HeadTerm::Count);
    }
    let (mut arg, _) = take_paren_group(tokens, "the aggregate's argument")?;
    let first = expect_ident(&mut arg, "a variable")?;
    let measure = first.text == "Duration" && matches!(arg.peek(), Some(TokenTree::Group(_)));
    let over = if measure {
        if !matches!(op, AggOp::Sum | AggOp::Min | AggOp::Max) {
            return fail(
                first.span,
                "query!: the measure folds under Sum/Min/Max only \
                 (docs/architecture/20-query-ir.md § the measure)",
            );
        }
        let (mut inner, _) = take_paren_group(&mut arg, "the measured variable")?;
        let var = expect_ident(&mut inner, "a variable")?;
        if let Some(extra) = inner.next() {
            return fail(extra.span(), "query!: Duration takes one variable");
        }
        var
    } else {
        first
    };
    if let Some(extra) = arg.next() {
        return fail(extra.span(), "query!: the aggregate takes one argument");
    }
    Ok(if measure {
        HeadTerm::AggMeasure { op, over }
    } else {
        HeadTerm::Agg { op, over }
    })
}

/// Parses one head term: a variable, `Duration(v)`, an aggregate, or a
/// named aggregate (`name: agg` — the name stays at the call site;
/// result columns are positional). Params are refused here: a param is
/// an execution input, not a result column.
fn parse_head_term(tokens: &mut Tokens) -> Parse<HeadTerm> {
    if peek_punct(tokens, '?') {
        let span = peek_span(tokens);
        return fail(
            span,
            "query!: a ?param cannot appear in a head — params are execution \
             inputs, not result columns; bind the value in the body",
        );
    }
    let name = expect_ident(tokens, "a head term")?;
    // The optional column name: `name: agg`.
    if peek_punct(tokens, ':') {
        expect_colon(tokens, "the head column's `:`")?;
        let agg_name = expect_ident(tokens, "an aggregate")?;
        let Some(op) = agg_op(&agg_name.text) else {
            return fail(
                agg_name.span,
                format!(
                    "query!: `{}` is not an aggregate — a named head position \
                     takes Sum/Min/Max/Count/Pack",
                    agg_name.text
                ),
            );
        };
        return parse_agg(tokens, op);
    }
    if let Some(op) = agg_op(&name.text) {
        return parse_agg(tokens, op);
    }
    if name.text == "Duration" && matches!(tokens.peek(), Some(TokenTree::Group(_))) {
        let (mut inner, _) = take_paren_group(tokens, "the measured variable")?;
        let var = expect_ident(&mut inner, "a variable")?;
        if let Some(extra) = inner.next() {
            return fail(extra.span(), "query!: Duration takes one variable");
        }
        return Ok(HeadTerm::Measure(var));
    }
    Ok(HeadTerm::Var(name))
}

/// One comma-separated group list — head terms, atom bindings, tree
/// conditions. The separator is MANDATORY between items (the grammar's
/// `x (',' x)*`, exactly): one strictness regime, one loop, so the
/// parsed language cannot drift into a superset of the notation
/// (finding 055).
fn parse_separated<T>(
    mut tokens: Tokens,
    mut item: impl FnMut(&mut Tokens) -> Parse<T>,
) -> Parse<Vec<T>> {
    let mut items = Vec::new();
    while tokens.peek().is_some() {
        items.push(item(&mut tokens)?);
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        } else if let Some(extra) = tokens.next() {
            return fail(
                extra.span(),
                format!("query!: expected `,`, found `{extra}`"),
            );
        }
    }
    Ok(items)
}

fn parse_head(tokens: Tokens) -> Parse<Vec<HeadTerm>> {
    parse_separated(tokens, parse_head_term)
}

/// Parses one selection value (after a binding's `==`).
fn parse_sel_value(tokens: &mut Tokens) -> Parse<SelValue> {
    if peek_punct(tokens, '?') {
        let question = expect_punct(tokens, '?', "`?`")?;
        return Ok(SelValue::Param(parse_param(tokens, question)?));
    }
    if let Some(word) = peek_ident_text(tokens) {
        let name = expect_ident(tokens, "a value")?;
        return Ok(match word.as_str() {
            "true" => SelValue::Lit(Lit::Bool(true)),
            "false" => SelValue::Lit(Lit::Bool(false)),
            _ => {
                if peek_punct(tokens, ':') {
                    // Qualified handle: `Enum::Handle`.
                    expect_colon(tokens, "the handle path's `::`")?;
                    expect_punct(tokens, ':', "the handle path's `::`")?;
                    let handle = expect_ident(tokens, "a handle name")?;
                    SelValue::Handle {
                        qualifier: Some(name),
                        handle,
                    }
                } else {
                    SelValue::Handle {
                        qualifier: None,
                        handle: name,
                    }
                }
            }
        });
    }
    Ok(SelValue::Lit(parse_lit(tokens)?))
}

/// One binding's field label: a field name, or — in an interior/rec atom —
/// a head position (`2: x`, the sparse/selection spelling; `FieldId(i)`
/// is positional, never nominal). Which one is legal is the atom's
/// source's business, decided at emission (the derived-table list exists
/// only after every rule has parsed — mutual recursion reads forward).
fn expect_field_label(tokens: &mut Tokens) -> Parse<Name> {
    match tokens.peek() {
        Some(TokenTree::Literal(lit)) => {
            let text = lit.to_string();
            let span = lit.span();
            if text.parse::<u16>().is_err() {
                return fail(
                    span,
                    format!("query!: expected a field name or head position, found `{text}`"),
                );
            }
            tokens.next();
            Ok(Name { text, span })
        }
        _ => expect_ident(tokens, "a field name"),
    }
}

/// Parses one atom binding, per the grammar's four spellings.
fn parse_binding(tokens: &mut Tokens) -> Parse<Binding> {
    let field = expect_field_label(tokens)?;
    if peek_punct(tokens, ':') {
        expect_colon(tokens, "the binding's `:`")?;
        let var = expect_ident(tokens, "a variable")?;
        Ok(Binding::Var { field, var })
    } else if peek_punct(tokens, '=') {
        expect_punct(tokens, '=', "`==`")?;
        expect_punct(tokens, '=', "`==`")?;
        let value = parse_sel_value(tokens)?;
        Ok(Binding::Value { field, value })
    } else if peek_ident_text(tokens).as_deref() == Some("in") {
        let in_kw = expect_ident(tokens, "`in`")?;
        if !peek_punct(tokens, '?') {
            return fail(
                in_kw.span,
                "query!: a binding's `in` takes a ?param bound to a set — \
                 interval membership is the `==` typing rule or a body item",
            );
        }
        let question = expect_punct(tokens, '?', "`?`")?;
        let param = parse_param(tokens, question)?;
        Ok(Binding::SetParam { field, param })
    } else {
        Ok(Binding::Pun(field))
    }
}

fn parse_atom(tokens: &mut Tokens, relation: Name) -> Parse<Atom> {
    let (group, _) = take_paren_group(tokens, "the atom's bindings")?;
    Ok(Atom {
        relation,
        bindings: parse_separated(group, parse_binding)?,
    })
}

/// Parses one term: a variable, `Duration(v)`, a `?param`, or a literal.
fn parse_term(tokens: &mut Tokens) -> Parse<Term> {
    if peek_punct(tokens, '?') {
        let question = expect_punct(tokens, '?', "`?`")?;
        return Ok(Term::Param(parse_param(tokens, question)?));
    }
    if let Some(word) = peek_ident_text(tokens) {
        let name = expect_ident(tokens, "a term")?;
        if word == "true" {
            return Ok(Term::Lit(Lit::Bool(true)));
        }
        if word == "false" {
            return Ok(Term::Lit(Lit::Bool(false)));
        }
        if word == "Duration" && matches!(tokens.peek(), Some(TokenTree::Group(_))) {
            let (mut inner, _) = take_paren_group(tokens, "the measured variable")?;
            let var = expect_ident(&mut inner, "a variable")?;
            if let Some(extra) = inner.next() {
                return fail(extra.span(), "query!: Duration takes one variable");
            }
            return Ok(Term::Measure(var));
        }
        return Ok(Term::Var(name));
    }
    Ok(Term::Lit(parse_lit(tokens)?))
}

/// Parses the `Allen` mask position: mask names joined by
/// `|` (set union over the 13 basics; the names are `AllenMask`'s own
/// constants, so a typo is a compile error). Mask params are refused —
/// the mask is a literal.
fn parse_mask(tokens: &mut Tokens) -> Parse<Mask> {
    if peek_punct(tokens, '?') {
        let question = expect_punct(tokens, '?', "`?`")?;
        return fail(
            question,
            "query!: Allen masks are literals — `INTERSECTS`, `MEETS`, \
             `DISJOINT`, a basic, or a `|` union; not a param",
        );
    }
    let mut names = vec![expect_ident(tokens, "a mask name")?];
    while peek_punct(tokens, '|') {
        tokens.next();
        names.push(expect_ident(tokens, "a mask name")?);
    }
    Ok(Mask::Names(names))
}

/// The comparison operators, longest spelling first.
fn parse_cmp_op(tokens: &mut Tokens) -> Parse<&'static str> {
    let (first, span) = match tokens.next() {
        Some(TokenTree::Punct(p)) => (p.as_char(), p.span()),
        Some(other) => {
            return fail(
                other.span(),
                format!("query!: expected a comparison, found `{other}`"),
            );
        }
        None => return fail(Span::call_site(), "query!: expected a comparison"),
    };
    let eq_follows = peek_punct(tokens, '=');
    let op = match (first, eq_follows) {
        ('=', true) => {
            tokens.next();
            "Eq"
        }
        ('!', true) => {
            tokens.next();
            "Ne"
        }
        ('<', true) => {
            tokens.next();
            "Le"
        }
        ('>', true) => {
            tokens.next();
            "Ge"
        }
        ('<', false) => "Lt",
        ('>', false) => "Gt",
        (':', _) if peek_punct(tokens, '-') => return datalog_refusal(span),
        _ => {
            return fail(
                span,
                format!("query!: `{first}` is not a comparison operator"),
            );
        }
    };
    Ok(op)
}

/// Continues a leaf whose left term is already parsed: membership or a
/// comparison.
fn finish_term_leaf(tokens: &mut Tokens, lhs: Term) -> Parse<Leaf> {
    if peek_ident_text(tokens).as_deref() == Some("in") {
        tokens.next();
        let container = parse_term(tokens)?;
        return Ok(Leaf::Membership {
            element: lhs,
            container,
        });
    }
    let op = parse_cmp_op(tokens)?;
    let rhs = parse_term(tokens)?;
    Ok(Leaf::Cmp { op, lhs, rhs })
}

/// Parses `Allen`'s three positions (the name already consumed).
fn parse_allen_leaf(tokens: &mut Tokens) -> Parse<Leaf> {
    let (mut group, _) = take_paren_group(tokens, "Allen's three positions")?;
    let lhs = parse_term(&mut group)?;
    expect_punct(&mut group, ',', "`,`")?;
    let mask = parse_mask(&mut group)?;
    expect_punct(&mut group, ',', "`,`")?;
    let rhs = parse_term(&mut group)?;
    if let Some(extra) = group.next() {
        return fail(extra.span(), "query!: Allen takes exactly three positions");
    }
    Ok(Leaf::Allen { lhs, mask, rhs })
}

/// Parses `Duration(v)` compared or contained (the name already
/// consumed) — the one parenthesized term.
fn parse_measure_leaf(tokens: &mut Tokens) -> Parse<Leaf> {
    let (mut inner, _) = take_paren_group(tokens, "the measured variable")?;
    let var = expect_ident(&mut inner, "a variable")?;
    if let Some(extra) = inner.next() {
        return fail(extra.span(), "query!: Duration takes one variable");
    }
    finish_term_leaf(tokens, Term::Measure(var))
}

/// The condition-tree refusal, one message for every non-comparison
/// shape under `and`/`or`.
fn tree_refusal<T>(span: Span) -> Parse<T> {
    fail(
        span,
        "query!: a condition tree takes comparisons only — atoms, negation, \
         and the binding membership stay body items \
         (docs/architecture/20-query-ir.md § the query notation)",
    )
}

/// Parses one `and(…)`/`or(…)` node's children (the name already
/// consumed): one condition at least, comma-separated.
fn parse_tree_children(tokens: &mut Tokens, name: &Name) -> Parse<Vec<Cond>> {
    let (mut group, span) = take_paren_group(tokens, "the condition tree's conditions")?;
    if group.peek().is_none() {
        return fail(
            span,
            format!(
                "query!: `{}(…)` takes at least one condition — the empty \
                 combinations are not notation",
                name.text
            ),
        );
    }
    parse_separated(group, parse_cond)
}

/// Parses one condition of a tree: a comparison leaf or a nested
/// `and`/`or` node — never an atom, a negation, or a binding.
fn parse_cond(tokens: &mut Tokens) -> Parse<Cond> {
    if peek_punct(tokens, '!') {
        // `!=` never begins a leaf; a lone `!` is negation, an item shape.
        return tree_refusal(peek_span(tokens));
    }
    let call_shaped = match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let mut ahead = tokens.clone();
            ahead.next();
            matches!(ahead.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
        }
        _ => false,
    };
    if call_shaped {
        let name = expect_ident(tokens, "a condition")?;
        return match name.text.as_str() {
            "and" => Ok(Cond::And(parse_tree_children(tokens, &name)?)),
            "or" => Ok(Cond::Or(parse_tree_children(tokens, &name)?)),
            "Allen" => Ok(Cond::Leaf(parse_allen_leaf(tokens)?)),
            "Duration" => Ok(Cond::Leaf(parse_measure_leaf(tokens)?)),
            _ => tree_refusal(name.span),
        };
    }
    let lhs = parse_term(tokens)?;
    Ok(Cond::Leaf(finish_term_leaf(tokens, lhs)?))
}

/// Whether the token after a `Name (…)` shape continues a term item —
/// i.e. the parenthesized form is `Duration(v)` under comparison, not an
/// atom.
fn continues_as_term(tokens: &mut Tokens) -> bool {
    match tokens.peek() {
        Some(TokenTree::Punct(p)) => matches!(p.as_char(), '=' | '!' | '<' | '>'),
        Some(TokenTree::Ident(ident)) => ident.to_string() == "in",
        _ => false,
    }
}

fn parse_item(tokens: &mut Tokens) -> Parse<Item> {
    if peek_punct(tokens, '!') {
        // `!=` never begins an item; a lone `!` is negation.
        tokens.next();
        let relation = expect_ident(tokens, "the negated atom's relation")?;
        return Ok(Item::Negated(parse_atom(tokens, relation)?));
    }
    if peek_punct(tokens, ':') {
        let span = peek_span(tokens);
        tokens.next();
        if peek_punct(tokens, '-') {
            return datalog_refusal(span);
        }
        return fail(span, "query!: expected an atom, a comparison, or `in`");
    }
    let call_shaped = match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let mut ahead = tokens.clone();
            ahead.next();
            matches!(ahead.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
        }
        _ => false,
    };
    if call_shaped {
        let name = expect_ident(tokens, "an atom or a condition")?;
        match name.text.as_str() {
            "Allen" => return Ok(Item::Cond(Cond::Leaf(parse_allen_leaf(tokens)?))),
            // The condition grammar's reserved words: a body-position
            // `and(…)`/`or(…)` is always a tree (ruled 2026-07-23, R9).
            "and" => return Ok(Item::Cond(Cond::And(parse_tree_children(tokens, &name)?))),
            "or" => return Ok(Item::Cond(Cond::Or(parse_tree_children(tokens, &name)?))),
            _ => {}
        }
        // `Duration(v) >= …` is a term; everything else call-shaped is an
        // atom.
        let mut ahead = tokens.clone();
        ahead.next(); // the group
        if continues_as_term(&mut ahead) {
            if name.text != "Duration" {
                return fail(
                    name.span,
                    format!(
                        "query!: `{}(…)` cannot be compared — the only \
                         parenthesized term is Duration(v)",
                        name.text
                    ),
                );
            }
            return Ok(Item::Cond(Cond::Leaf(parse_measure_leaf(tokens)?)));
        }
        return Ok(Item::Atom(parse_atom(tokens, name)?));
    }
    let lhs = parse_term(tokens)?;
    Ok(Item::Cond(Cond::Leaf(finish_term_leaf(tokens, lhs)?)))
}

/// A derived-table name: lowercase, not a reserved word.
fn validate_pred_name(name: &Name) -> Parse<()> {
    if name.text == "and"
        || name.text == "or"
        || name.text == "interior"
        || name.text == "recursive"
    {
        return fail(
            name.span,
            format!(
                "query!: `{}` is reserved — `and`/`or` are the condition grammar, \
                 `interior`/`recursive` introduce derived tables \
                 (docs/architecture/20-query-ir.md § the query notation)",
                name.text
            ),
        );
    }
    if !name
        .text
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase())
    {
        return fail(
            name.span,
            format!(
                "query!: derived-table names begin lowercase (`{}`) — UpperCamel \
                 names are relations, so an interior/rec spelled like a relation \
                 is unwritable (docs/architecture/20-query-ir.md § the query \
                 notation)",
                name.text
            ),
        );
    }
    Ok(())
}

/// Parses one rule: `interior pred (head) | body ;`, `recursive pred
/// (head) | body ;`, or a bare `(head) | body ;`. A named head without
/// the keyword is the former named-head sneak — a spanned compile error.
fn parse_rule(tokens: &mut Tokens) -> Parse<ParsedRule> {
    let intro = match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let ident = expect_ident(tokens, "a rule")?;
            if peek_punct(tokens, ':') {
                // `pred :- …` must not parse — the refusal fires before
                // the head-group error would.
                let span = peek_span(tokens);
                tokens.next();
                if peek_punct(tokens, '-') {
                    return datalog_refusal(span);
                }
                return fail(span, "query!: expected the named rule's head `(…)`");
            }
            match ident.text.as_str() {
                "and" | "or" => {
                    return fail(
                        ident.span,
                        format!(
                            "query!: `{}` is the condition grammar's reserved word — \
                             an interior/rec cannot take either tree name \
                             (docs/architecture/20-query-ir.md § the query notation)",
                            ident.text
                        ),
                    );
                }
                "interior" | "recursive" => {
                    let pred = expect_ident(tokens, "a derived-table name")?;
                    validate_pred_name(&pred)?;
                    Some((ident.text == "interior", pred))
                }
                _ => {
                    if !ident
                        .text
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase())
                    {
                        return fail(
                            ident.span,
                            format!(
                                "query!: derived-table names begin lowercase (`{}`) — UpperCamel \
                                 names are relations, so an interior/rec spelled like a relation \
                                 is unwritable (docs/architecture/20-query-ir.md § the query \
                                 notation)",
                                ident.text
                            ),
                        );
                    }
                    return fail(
                        ident.span,
                        format!(
                            "query!: named heads require `interior` or `recursive` — \
                             write `interior {}(...)` or `recursive {}(...)`",
                            ident.text, ident.text
                        ),
                    );
                }
            }
        }
        _ => None,
    };
    let (head_group, head_span) = take_paren_group(tokens, "a rule head `(…)`")?;
    let head = parse_head(head_group)?;
    if head.is_empty() {
        return fail(head_span, "query!: a head needs at least one term");
    }
    match tokens.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == '|' => {}
        Some(TokenTree::Punct(p)) if p.as_char() == ':' && peek_punct(tokens, '-') => {
            return datalog_refusal(p.span());
        }
        Some(other) => {
            return fail(
                other.span(),
                format!("query!: expected `|` (*such that*) after the head, found `{other}`"),
            );
        }
        None => {
            return fail(
                head_span,
                "query!: expected `|` (*such that*) after the head",
            );
        }
    }
    let mut items = Vec::new();
    loop {
        if peek_punct(tokens, ';') {
            let span = peek_span(tokens);
            if items.is_empty() {
                return fail(span, "query!: a rule body needs at least one atom");
            }
            tokens.next();
            break;
        }
        if tokens.peek().is_none() {
            return fail(Span::call_site(), "query!: a rule ends with `;`");
        }
        items.push(parse_item(tokens)?);
        // The separator is mandatory between items (finding 055): `,`
        // continues the body, `;` ends the rule, anything else is the
        // grammar-superset this parser refuses.
        if peek_punct(tokens, ',') {
            tokens.next();
        } else if !peek_punct(tokens, ';') {
            return match tokens.next() {
                Some(extra) => fail(
                    extra.span(),
                    format!("query!: expected `,` or `;`, found `{extra}`"),
                ),
                None => fail(Span::call_site(), "query!: a rule ends with `;`"),
            };
        }
    }
    Ok(match intro {
        None => ParsedRule::Bare { head, items },
        Some((true, name)) => ParsedRule::Interior { name, head, items },
        Some((false, name)) => ParsedRule::Recursive { name, head, items },
    })
}

// ---------------------------------------------------------------------
// Resolution and emission — names to dense ids, the rest to constant
// paths rustc checks.
// ---------------------------------------------------------------------

/// A declaration name as a `SCREAMING_SNAKE` constant name — verbatim
/// `bumbledb-macros`' rule (`SavingsTerms` → `SAVINGS_TERMS`, `rate_bps`
/// → `RATE_BPS`), so the paths this macro emits land on the constants
/// that macro emits.
/// `claim_arm` → `ClaimArm`: the field-to-host-enum name convention for
/// bare handle selection values.
fn upper_camel(name: &str) -> String {
    let mut out = String::new();
    for word in name.split('_') {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn screaming_snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (index, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && index > 0 {
            let prev = chars[index - 1];
            let heads_word = chars.get(index + 1).is_some_and(char::is_ascii_lowercase);
            if prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && heads_word)
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// The query-global param table: one spelling style per query — named
/// (dense by first occurrence) or positional (`?N`, verbatim — the
/// renderer's spelling).
enum ParamStyle {
    Empty,
    Named(Vec<String>),
    Index,
}

struct Params {
    style: ParamStyle,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            style: ParamStyle::Empty,
        }
    }
}

impl Params {
    fn resolve(&mut self, param: &Param) -> Parse<u16> {
        match param {
            Param::Named(name) => {
                if matches!(self.style, ParamStyle::Index) {
                    return fail(
                        name.span,
                        "query!: named and positional ?params cannot mix — \
                         pick one spelling per query",
                    );
                }
                if matches!(self.style, ParamStyle::Empty) {
                    self.style = ParamStyle::Named(vec![name.text.clone()]);
                    return Ok(0);
                }
                let ParamStyle::Named(named) = &mut self.style else {
                    unreachable!("Empty and Index returned above");
                };
                let position = named
                    .iter()
                    .position(|existing| *existing == name.text)
                    .unwrap_or_else(|| {
                        named.push(name.text.clone());
                        named.len() - 1
                    });
                u16::try_from(position)
                    .map_or_else(|_| fail(name.span, "query!: too many params"), Ok)
            }
            Param::Index { index, span } => {
                if matches!(self.style, ParamStyle::Named(_)) {
                    return fail(
                        *span,
                        "query!: named and positional ?params cannot mix — \
                         pick one spelling per query",
                    );
                }
                self.style = ParamStyle::Index;
                Ok(*index)
            }
        }
    }
}

/// One rule's variable scope: dense ids by first occurrence, plus the
/// punning ledger (law B: the same punned name twice is ambiguous, and
/// the error points at the second occurrence).
#[derive(Default)]
struct Scope {
    vars: Vec<String>,
    punned: Vec<String>,
}

impl Scope {
    fn intern(&mut self, name: &Name) -> Parse<u16> {
        let position = self
            .vars
            .iter()
            .position(|existing| *existing == name.text)
            .unwrap_or_else(|| {
                self.vars.push(name.text.clone());
                self.vars.len() - 1
            });
        u16::try_from(position).map_or_else(|_| fail(name.span, "query!: too many variables"), Ok)
    }

    fn pun(&mut self, name: &Name) -> Parse<u16> {
        if self.punned.contains(&name.text) {
            return fail(name.span, "query!: ambiguous punning — rename explicitly");
        }
        self.punned.push(name.text.clone());
        self.intern(name)
    }

    fn head_var(&self, name: &Name) -> Parse<u16> {
        self.vars
            .iter()
            .position(|existing| *existing == name.text)
            .map_or_else(
                || {
                    fail(
                        name.span,
                        format!(
                            "query!: head variable `{}` is not bound in the rule body",
                            name.text
                        ),
                    )
                },
                |position| {
                    u16::try_from(position)
                        .map_or_else(|_| fail(name.span, "query!: too many variables"), Ok)
                },
            )
    }
}

/// An interior/rec atom's binding style. Mixing is unrepresentable —
/// the second style's first occurrence carries the mixing diagnostic.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BindingStyle {
    Empty,
    Bare,
    Numeric,
}

/// Walks an interior/rec atom's bindings into one style. Refuses a bare
/// digit (a variable is an ident), a named label (derived columns are
/// positions), and the two styles mixed.
fn interior_style(atom: &Atom) -> Parse<BindingStyle> {
    let mut style = BindingStyle::Empty;
    for binding in &atom.bindings {
        let (Binding::Pun(field)
        | Binding::Var { field, .. }
        | Binding::Value { field, .. }
        | Binding::SetParam { field, .. }) = binding;
        let bare = matches!(binding, Binding::Pun(_));
        let numeric = field
            .text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit());
        if bare && numeric {
            return fail(
                field.span,
                "query!: a bare interior/rec binding is a variable (`reach(m, a)` — \
                 ordered dense, positions left to right from 0); a position \
                 label takes a form (`2: x`, `0 == …`, `0 in ?p`)",
            );
        }
        if !bare && !numeric {
            return fail(
                field.span,
                format!(
                    "query!: `{}` — an interior/rec atom's bindings address head \
                     positions, never names: ordered dense is bare \
                     (`reach(m, a)`), sparse and selection are indexed \
                     (`2: x`, `0 == …`)",
                    field.text
                ),
            );
        }
        let next = if bare {
            BindingStyle::Bare
        } else {
            BindingStyle::Numeric
        };
        match style {
            BindingStyle::Empty => style = next,
            BindingStyle::Bare if !bare => {
                return fail(
                    field.span,
                    "query!: bare idents and indexed labels cannot mix in one \
                     interior/rec atom — ordered dense bindings are all bare \
                     (`reach(m, a)`); sparse and selection bindings are all \
                     indexed (`2: x`, `0 == …`)",
                );
            }
            BindingStyle::Numeric if bare => {
                return fail(
                    field.span,
                    "query!: bare idents and indexed labels cannot mix in one \
                     interior/rec atom — ordered dense bindings are all bare \
                     (`reach(m, a)`); sparse and selection bindings are all \
                     indexed (`2: x`, `0 == …`)",
                );
            }
            BindingStyle::Bare | BindingStyle::Numeric => {}
        }
    }
    Ok(style)
}

struct Emitter<'a> {
    theory: &'a str,
    params: Params,
    /// Macro-local derived-table names in InteriorId order: interiors
    /// first, then the rec if present (`InteriorId(interiors.len())`).
    /// Names never survive expansion — the emitted IR carries bare
    /// `InteriorId`s. Empty for an all-bare query.
    derived: Vec<String>,
}

impl Emitter<'_> {
    fn var(id: u16) -> String {
        format!("::bumbledb::Term::Var(::bumbledb::VarId({id}))")
    }

    fn param(&mut self, param: &Param) -> Parse<String> {
        let id = self.params.resolve(param)?;
        Ok(format!(
            "::bumbledb::Term::Param(::bumbledb::ParamId({id}))"
        ))
    }

    /// One literal as a `Value` expression, typed by its spelling
    /// (module doc); raw token text spliced verbatim so rustc polices
    /// the value itself.
    fn lit(lit: &Lit) -> String {
        let value = "::bumbledb::Value";
        let int_text = |int: &Int| {
            if int.negative {
                format!("-{}", int.text)
            } else {
                int.text.clone()
            }
        };
        match lit {
            Lit::Bool(v) => format!("{value}::Bool({v})"),
            Lit::Int(int) => {
                let variant = if int.signed { "I64" } else { "U64" };
                format!("{value}::{variant}({})", int_text(int))
            }
            Lit::Interval { start, end } => {
                let (variant, ty) = if start.signed || end.signed {
                    ("IntervalI64", "i64")
                } else {
                    ("IntervalU64", "u64")
                };
                format!(
                    "{value}::{variant}(::bumbledb::Interval::<{ty}>::new({}, {})\
                     .expect(\"query! interval literals are nonempty\"))",
                    int_text(start),
                    int_text(end)
                )
            }
            Lit::Str(text) => {
                format!("{value}::String(::std::boxed::Box::from({text}.as_bytes()))")
            }
            Lit::Bytes(text) => {
                format!("{value}::FixedBytes(::std::boxed::Box::from(&{text}[..]))")
            }
        }
    }

    fn term(&mut self, scope: &mut Scope, term: &Term) -> Parse<String> {
        Ok(match term {
            Term::Var(name) => Self::var(scope.intern(name)?),
            Term::Param(param) => self.param(param)?,
            Term::Measure(name) => format!(
                "::bumbledb::Term::Measure(::bumbledb::VarId({}))",
                scope.intern(name)?
            ),
            Term::Lit(lit) => format!("::bumbledb::Term::Literal({})", Self::lit(lit)),
        })
    }

    /// One selection value as the binding's term expression. A handle
    /// resolves through the host enum's welded row id: bare through the
    /// field-named host enum, qualified through the named one (module
    /// doc — the macro cannot see the theory; rustc checks the path and
    /// the emitted `const fn id` supplies the newtype, whose `.0` is the
    /// plain u64 row id).
    fn sel_value(&mut self, field: &Name, value: &SelValue) -> Parse<String> {
        Ok(match value {
            SelValue::Lit(lit) => format!("::bumbledb::Term::Literal({})", Self::lit(lit)),
            SelValue::Param(param) => self.param(param)?,
            SelValue::Handle { qualifier, handle } => {
                let host = qualifier
                    .as_ref()
                    .map_or_else(|| upper_camel(&field.text), |name| name.text.clone());
                format!(
                    "::bumbledb::Term::Literal(::bumbledb::Value::U64({host}::{}.id().0))",
                    handle.text
                )
            }
        })
    }

    /// An interior/rec atom as an `Atom` expression: an `Interior` source
    /// whose bindings address head positions (`FieldId(i)` is the target's
    /// column `i` — positional, never nominal). Two spellings, one
    /// meaning each: bare idents are ORDERED DENSE variable bindings,
    /// positions assigned left to right from 0 (`reach(m, a)` lowers to
    /// `[(0, m), (1, a)]`), and indexed labels are the sparse and
    /// selection forms (`2: x`, `0 == …`, `0 in ?p`). The two never mix,
    /// and an explicitly indexed dense in-order variable list is refused
    /// — canonical utterance, one spelling per meaning.
    fn interior_atom(&mut self, scope: &mut Scope, atom: &Atom, interior: u32) -> Parse<String> {
        if interior_style(atom)? == BindingStyle::Bare {
            // Ordered dense: positions assigned left to right from 0.
            let mut bindings = String::new();
            for (position, binding) in atom.bindings.iter().enumerate() {
                let Binding::Pun(name) = binding else {
                    unreachable!("the style split sealed an all-bare atom");
                };
                let term = Self::var(scope.intern(name)?);
                let _ = write!(bindings, "(::bumbledb::FieldId({position}), {term}),");
            }
            return Ok(format!(
                "::bumbledb::Atom {{ source: ::bumbledb::AtomSource::Interior(::bumbledb::InteriorId({interior})), bindings: ::std::vec![{bindings}] }}"
            ));
        }
        // Indexed labels. An explicit dense in-order variable list is the
        // ordered form's meaning respelled — refused, one spelling per
        // meaning.
        let dense_explicit = !atom.bindings.is_empty()
            && atom.bindings.iter().enumerate().all(|(index, binding)| {
                matches!(binding, Binding::Var { field, .. }
                    if field.text.parse::<usize>() == Ok(index))
            });
        if dense_explicit {
            let Binding::Var { field, .. } = &atom.bindings[0] else {
                unreachable!("dense_explicit is all explicit variables");
            };
            return fail(
                field.span,
                "query!: dense in-order interior/rec bindings are written bare — \
                 `reach(m, a)`, positions left to right from 0; `i: v` is the \
                 sparse spelling (`2: x`)",
            );
        }
        let mut bindings = String::new();
        for binding in &atom.bindings {
            let (field, term) = match binding {
                Binding::Pun(_) => {
                    unreachable!("the style split sealed an all-indexed atom")
                }
                Binding::Var { field, var } => (field, Self::var(scope.intern(var)?)),
                Binding::Value {
                    field: _,
                    value:
                        SelValue::Handle {
                            qualifier: None,
                            handle,
                        },
                } => {
                    return fail(
                        handle.span,
                        "query!: a bare handle resolves through the field-named host \
                         enum, and an interior/rec position has no field name — qualify \
                         it (`Kind::Focus`)",
                    );
                }
                Binding::Value { field, value } => (field, self.sel_value(field, value)?),
                Binding::SetParam { field, param } => {
                    let id = self.params.resolve(param)?;
                    (
                        field,
                        format!("::bumbledb::Term::ParamSet(::bumbledb::ParamId({id}))"),
                    )
                }
            };
            let position = field
                .text
                .parse::<u16>()
                .expect("the style split sealed numeric labels");
            let _ = write!(bindings, "(::bumbledb::FieldId({position}), {term}),");
        }
        Ok(format!(
            "::bumbledb::Atom {{ source: ::bumbledb::AtomSource::Interior(::bumbledb::InteriorId({interior})), bindings: ::std::vec![{bindings}] }}"
        ))
    }

    /// One atom as an `Atom` expression — a derived table of this query by
    /// macro-local name, else the relation and every field through the
    /// theory's id constants. The case partition is total (finding 054):
    /// a lowercase name IS a derived table, so one absent from the table is
    /// an unknown derived table, never a relation respelled — `parent(…)`
    /// must not resolve to `Parent`'s constants.
    fn atom(&mut self, scope: &mut Scope, atom: &Atom) -> Parse<String> {
        if let Some(interior) = self
            .derived
            .iter()
            .position(|name| *name == atom.relation.text)
            .and_then(|index| u32::try_from(index).ok())
        {
            return self.interior_atom(scope, atom, interior);
        }
        if atom
            .relation
            .text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
        {
            return fail(
                atom.relation.span,
                format!(
                    "query!: unknown derived table `{}` — lowercase names are \
                     interiors or the rec, resolved macro-locally; relations are \
                     UpperCamel (docs/architecture/20-query-ir.md § the \
                     query notation)",
                    atom.relation.text
                ),
            );
        }
        let relation = format!("{}::{}", self.theory, screaming_snake(&atom.relation.text));
        let theory = self.theory.to_owned();
        let field_const = move |field: &Name| {
            format!(
                "{theory}::{}_{}",
                screaming_snake(&atom.relation.text),
                screaming_snake(&field.text)
            )
        };
        let mut bindings = String::new();
        for binding in &atom.bindings {
            if let Binding::Pun(field)
            | Binding::Var { field, .. }
            | Binding::Value { field, .. }
            | Binding::SetParam { field, .. } = binding
                && field
                    .text
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
            {
                return fail(
                    field.span,
                    format!(
                        "query!: `{}` — numeric labels address an interior/rec atom's \
                         head positions; a relation's fields are named",
                        field.text
                    ),
                );
            }
            let (field, term) = match binding {
                Binding::Pun(field) => (field, Self::var(scope.pun(field)?)),
                Binding::Var { field, var } => (field, Self::var(scope.intern(var)?)),
                Binding::Value { field, value } => (field, self.sel_value(field, value)?),
                Binding::SetParam { field, param } => {
                    let id = self.params.resolve(param)?;
                    (
                        field,
                        format!("::bumbledb::Term::ParamSet(::bumbledb::ParamId({id}))"),
                    )
                }
            };
            let _ = write!(bindings, "({}, {term}),", field_const(field));
        }
        Ok(format!(
            "::bumbledb::Atom {{ source: ::bumbledb::AtomSource::Edb({relation}), bindings: ::std::vec![{bindings}] }}"
        ))
    }

    fn mask(mask: &Mask) -> String {
        match mask {
            Mask::Names(names) => names
                .iter()
                .map(|name| format!("::bumbledb::AllenMask::{}", name.text))
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

    fn leaf(op: &str, lhs: &str, rhs: &str) -> String {
        format!(
            "::bumbledb::ConditionTree::Leaf(::bumbledb::Comparison {{ \
                 op: {op}, lhs: {lhs}, rhs: {rhs} }})"
        )
    }

    /// One condition tree as a `ConditionTree` expression — nested
    /// `And`/`Or` verbatim (validation distributes to DNF engine-side).
    fn cond(&mut self, scope: &mut Scope, cond: &Cond) -> Parse<String> {
        Ok(match cond {
            Cond::Leaf(Leaf::Allen { lhs, mask, rhs }) => {
                let lhs = self.term(scope, lhs)?;
                let rhs = self.term(scope, rhs)?;
                let mask = Self::mask(mask);
                let op = format!("::bumbledb::CmpOp::Allen {{ mask: {mask} }}");
                Self::leaf(&op, &lhs, &rhs)
            }
            // `PointIn` is stored interval-first; the notation reads
            // point-first.
            Cond::Leaf(Leaf::Membership { element, container }) => {
                let element = self.term(scope, element)?;
                let container = self.term(scope, container)?;
                Self::leaf("::bumbledb::CmpOp::PointIn", &container, &element)
            }
            Cond::Leaf(Leaf::Cmp { op, lhs, rhs }) => {
                let lhs = self.term(scope, lhs)?;
                let rhs = self.term(scope, rhs)?;
                let op = format!("::bumbledb::CmpOp::{op}");
                Self::leaf(&op, &lhs, &rhs)
            }
            Cond::And(children) | Cond::Or(children) => {
                let variant = if matches!(cond, Cond::And(_)) {
                    "And"
                } else {
                    "Or"
                };
                let mut inner = String::new();
                for child in children {
                    let _ = write!(inner, "{},", self.cond(scope, child)?);
                }
                format!("::bumbledb::ConditionTree::{variant}(::std::vec![{inner}])")
            }
        })
    }

    /// One head position as a `FindTerm` expression; every variable must
    /// already be body-bound.
    fn find(scope: &Scope, term: &HeadTerm) -> Parse<String> {
        Ok(match term {
            HeadTerm::Var(name) => format!(
                "::bumbledb::FindTerm::Var(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            HeadTerm::Measure(name) => format!(
                "::bumbledb::FindTerm::Measure(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            HeadTerm::Count => {
                "::bumbledb::FindTerm::Aggregate { op: ::bumbledb::AggOp::Count, \
                     over: ::std::option::Option::None }"
                    .to_string()
            }
            HeadTerm::Agg { op, over } => format!(
                "::bumbledb::FindTerm::Aggregate {{ op: ::bumbledb::AggOp::{}, \
                     over: ::std::option::Option::Some(::bumbledb::VarId({})) }}",
                op.ir_name(),
                scope.head_var(over)?
            ),
            HeadTerm::AggMeasure { op, over } => format!(
                "::bumbledb::FindTerm::AggregateMeasure {{ op: ::bumbledb::AggOp::{}, \
                     over: ::bumbledb::VarId({}) }}",
                op.ir_name(),
                scope.head_var(over)?
            ),
        })
    }

    /// One parsed rule as a `Rule` expression. Items lower in source order;
    /// the IR buckets them (atoms, negated, conditions) — the renderer's
    /// normalized order.
    fn rule(&mut self, rule: &ParsedRule) -> Parse<String> {
        let mut scope = Scope::default();
        let mut atoms = String::new();
        let mut negated = String::new();
        let mut conditions = String::new();
        for item in rule.items() {
            match item {
                Item::Atom(atom) => {
                    let _ = write!(atoms, "{},", self.atom(&mut scope, atom)?);
                }
                Item::Negated(atom) => {
                    let _ = write!(negated, "{},", self.atom(&mut scope, atom)?);
                }
                Item::Cond(cond) => {
                    let _ = write!(conditions, "{},", self.cond(&mut scope, cond)?);
                }
            }
        }
        let mut finds = String::new();
        for term in rule.head() {
            let _ = write!(finds, "{},", Self::find(&scope, term)?);
        }
        Ok(format!(
            "::bumbledb::Rule {{ \
                 finds: ::std::vec![{finds}], \
                 atoms: ::std::vec![{atoms}], \
                 negated: ::std::vec![{negated}], \
                 conditions: ::std::vec![{conditions}] }}"
        ))
    }
}

/// Parses the leading theory path (`Theory` or `crate::path::Theory`) —
/// spliced verbatim before every `::CONST` — leaving the brace group as
/// the next token.
fn parse_theory(tokens: &mut Tokens) -> Parse<String> {
    let mut theory = String::new();
    loop {
        match tokens.peek() {
            Some(TokenTree::Ident(_)) => {
                let name = expect_ident(tokens, "the theory")?;
                theory.push_str(&name.text);
            }
            Some(TokenTree::Punct(p)) if p.as_char() == ':' => {
                expect_colon(tokens, "the theory path's `::`")?;
                expect_punct(tokens, ':', "the theory path's `::`")?;
                theory.push_str("::");
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => break,
            Some(other) => {
                return fail(
                    other.span(),
                    "query!: the shape is `query!(Theory { rules })`",
                );
            }
            None => {
                return fail(
                    Span::call_site(),
                    "query!: the shape is `query!(Theory { rules })`",
                );
            }
        }
    }
    if theory.is_empty() || theory.ends_with("::") {
        return fail(peek_span(tokens), "query!: name the theory first");
    }
    Ok(theory)
}

/// Whether a rule body names `pred` as a positive or negated atom.
fn names_pred(rule: &ParsedRule, pred: &str) -> bool {
    rule.items().iter().any(|item| match item {
        Item::Atom(atom) | Item::Negated(atom) => atom.relation.text == pred,
        Item::Cond(_) => false,
    })
}

struct InteriorGroup {
    name: Name,
    rules: Vec<ParsedRule>,
}

struct RecGroup {
    name: Name,
    base: Vec<ParsedRule>,
    rec: Vec<ParsedRule>,
}

enum Phase {
    Interiors,
    Rec,
    Main,
}

/// Groups consecutive same-name interiors / recursive lines and splits
/// rec arms by whether a body atom names the rec. Exhaustive
/// compile errors for this cut live here.
fn classify(
    parsed: Vec<ParsedRule>,
    block: Span,
) -> Parse<(Vec<InteriorGroup>, Option<RecGroup>, Vec<ParsedRule>)> {
    let mut interiors: Vec<InteriorGroup> = Vec::new();
    let mut rec: Option<RecGroup> = None;
    let mut main: Vec<ParsedRule> = Vec::new();
    let mut phase = Phase::Interiors;
    for rule in parsed {
        match rule {
            ParsedRule::Interior { name, head, items } => {
                let rule = ParsedRule::Interior {
                    name: name.clone(),
                    head,
                    items,
                };
                if matches!(phase, Phase::Rec) {
                    return fail(
                        name.span,
                        "query!: `interior` cannot follow `recursive` — declaration \
                         order is interiors, then rec, then main",
                    );
                }
                if matches!(phase, Phase::Main) {
                    return fail(
                        name.span,
                        "query!: `interior` cannot follow a bare rule — declaration \
                         order is interiors, then rec, then main",
                    );
                }
                if rec
                    .as_ref()
                    .is_some_and(|group| group.name.text == name.text)
                {
                    return fail(
                        name.span,
                        format!(
                            "query!: `{0}` cannot be both `interior` and `recursive` — \
                             derived names are unique",
                            name.text
                        ),
                    );
                }
                if let Some(last) = interiors.last_mut()
                    && last.name.text == name.text
                {
                    last.rules.push(rule);
                    continue;
                }
                if interiors.iter().any(|group| group.name.text == name.text) {
                    return fail(
                        name.span,
                        format!(
                            "query!: interior `{0}` is not consecutive — write every \
                             `interior {0}(...)` line together",
                            name.text
                        ),
                    );
                }
                interiors.push(InteriorGroup {
                    name,
                    rules: vec![rule],
                });
            }
            ParsedRule::Recursive { name, head, items } => {
                let rule = ParsedRule::Recursive {
                    name: name.clone(),
                    head,
                    items,
                };
                if matches!(phase, Phase::Main) {
                    return fail(
                        name.span,
                        "query!: `recursive` cannot follow a bare rule — declaration \
                         order is interiors, then rec, then main",
                    );
                }
                phase = Phase::Rec;
                if interiors.iter().any(|group| group.name.text == name.text) {
                    return fail(
                        name.span,
                        format!(
                            "query!: `{0}` cannot be both `interior` and `recursive` — \
                             derived names are unique",
                            name.text
                        ),
                    );
                }
                let is_rec_arm = names_pred(&rule, &name.text);
                match &mut rec {
                    None => {
                        let mut group = RecGroup {
                            name,
                            base: Vec::new(),
                            rec: Vec::new(),
                        };
                        if is_rec_arm {
                            group.rec.push(rule);
                        } else {
                            group.base.push(rule);
                        }
                        rec = Some(group);
                    }
                    Some(existing) if existing.name.text == name.text => {
                        if is_rec_arm {
                            existing.rec.push(rule);
                        } else {
                            existing.base.push(rule);
                        }
                    }
                    Some(_) => {
                        return fail(
                            name.span,
                            "query!: at most one `recursive` name this cut — a \
                             second recursive is unwritable",
                        );
                    }
                }
            }
            ParsedRule::Bare { head, items } => {
                phase = Phase::Main;
                main.push(ParsedRule::Bare { head, items });
            }
        }
    }
    if main.is_empty() {
        return fail(
            block,
            "query!: a query needs a bare main rule — `interior` / `recursive` \
             declare derived tables; the answer is the unnamed rules",
        );
    }
    if let Some(group) = &rec {
        if group.base.is_empty() {
            return fail(
                group.name.span,
                format!(
                    "query!: `recursive {}` has no base arm — a line whose body \
                     does not name `{}` is a base arm",
                    group.name.text, group.name.text
                ),
            );
        }
        if group.rec.is_empty() {
            return fail(
                group.name.span,
                format!(
                    "query!: `recursive {}` has no rec arm — a line whose body \
                     names `{}` (positive or negated) is a rec arm",
                    group.name.text, group.name.text
                ),
            );
        }
    }
    Ok((interiors, rec, main))
}

fn emit_rules(emitter: &mut Emitter<'_>, rules: &[ParsedRule]) -> Parse<String> {
    let mut out = String::new();
    for rule in rules {
        let _ = write!(out, "{},", emitter.rule(rule)?);
    }
    Ok(out)
}

fn expand(input: TokenStream) -> Parse<String> {
    let mut tokens: Tokens = input.into_iter().peekable();
    let theory = parse_theory(&mut tokens)?;
    let Some(TokenTree::Group(group)) = tokens.next() else {
        unreachable!("peeked the brace group");
    };
    if let Some(extra) = tokens.next() {
        return fail(extra.span(), "query!: nothing follows the rule block");
    }
    let mut rule_tokens: Tokens = group.stream().into_iter().peekable();
    let mut parsed: Vec<ParsedRule> = Vec::new();
    while rule_tokens.peek().is_some() {
        parsed.push(parse_rule(&mut rule_tokens)?);
    }
    if parsed.is_empty() {
        return fail(group.span(), "query!: a query needs at least one rule");
    }
    let (interiors, rec, main) = classify(parsed, group.span())?;
    let mut derived: Vec<String> = interiors
        .iter()
        .map(|group| group.name.text.clone())
        .collect();
    if let Some(group) = &rec {
        derived.push(group.name.text.clone());
    }
    let mut emitter = Emitter {
        theory: &theory,
        params: Params::default(),
        derived,
    };
    // Param ids mint in IR walk order: interiors, rec base, rec arms, main.
    let mut lets = String::new();
    let mut interior_defs = String::new();
    for (index, group) in interiors.iter().enumerate() {
        let rules = emit_rules(&mut emitter, &group.rules)?;
        let _ = write!(
            lets,
            "let p{index}_rules = ::std::vec![{rules}]; \
             let p{index}_head = ::bumbledb::Rule::head(&p{index}_rules[0]); "
        );
        let _ = write!(
            interior_defs,
            "::bumbledb::Interior {{ head: p{index}_head, rules: p{index}_rules }},"
        );
    }
    let rec_expr = if let Some(group) = &rec {
        let rec_index = interiors.len();
        let base = emit_rules(&mut emitter, &group.base)?;
        let step = emit_rules(&mut emitter, &group.rec)?;
        let _ = write!(
            lets,
            "let p{rec_index}_base = ::std::vec![{base}]; \
             let p{rec_index}_rec = ::std::vec![{step}]; \
             let p{rec_index}_head = ::bumbledb::Rule::head(&p{rec_index}_base[0]); "
        );
        format!(
            "::std::option::Option::Some(::bumbledb::Rec {{ \
                 head: p{rec_index}_head, base: p{rec_index}_base, rec: p{rec_index}_rec }})"
        )
    } else {
        "::std::option::Option::None".to_owned()
    };
    let main_rules = emit_rules(&mut emitter, &main)?;
    Ok(format!(
        "{{ {lets}let rules = ::std::vec![{main_rules}]; \
             let head = ::bumbledb::Rule::head(&rules[0]); \
             ::bumbledb::Query {{ \
                 interiors: ::std::vec![{interior_defs}], \
                 rec: {rec_expr}, \
                 head, \
                 rules }} }}"
    ))
}

/// The query notation, lowered at compile time to the `ir::Query` value
/// (docs/architecture/20-query-ir.md § the query notation — the grammar
/// is the module doc's block, normative there). Names check through the
/// theory's id constants; derived-table names are macro-local and never
/// survive expansion (the IR carries bare `InteriorId`s); everything
/// semantic beyond names surfaces as the validation roster's typed
/// errors at `Db::prepare`.
///
/// ```ignore
/// let unavailable = bumbledb_query::query!(Calendar {
///     (person, during) | Busy(person, during), Allen(during, INTERSECTS, ?window);
///     (person, during) | Ooo(person, during),  Allen(during, INTERSECTS, ?window);
/// });
/// // `recursive` / `interior` declare derived tables; a body atom may
/// // name one (bare idents bind head POSITIONS, ordered dense — left to
/// // right from 0); bare rules are the main query.
/// let reachable = bumbledb_query::query!(Ledger {
///     recursive reach(c, a) | OrgParent(child: c, parent: a);
///     recursive reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
///     (c, a) | reach(c, a);
/// });
/// ```
///
/// # Panics
///
/// Never on malformed input — every diagnostic is a spanned
/// `compile_error!` at the offending token. The one internal `expect`
/// ensures the generated code parsing as Rust, a bug in this macro if it
/// ever fires.
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    match expand(input) {
        Ok(code) => code.parse().expect("query!: generated code parses"),
        Err(error) => compile_error(&error),
    }
}
