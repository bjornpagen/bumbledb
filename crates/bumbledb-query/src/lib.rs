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
//! query   := clause+                     // two or more clauses denote set union
//! clause  := '(' head ')' '|' body ';'
//! head    := headterm (',' headterm)*
//! headterm:= var | [name ':'] agg        // named positions become result columns
//! agg     := Sum(t) | Min(t) | Max(t) | Count | CountDistinct(v) | Pack(v)
//!            where t := v | Duration(v)
//! body    := item (',' item)*
//! item    := atom                        // positive occurrence
//!          | '!' atom                    // negation (anti-probe; safety per roster)
//!          | term 'in' term              // membership: point ∈ interval
//!          | Allen '(' term ',' mask ',' term ')'
//!          | term cmp term               // ==  !=  <  <=  >  >=
//! atom    := Relation '(' binding (',' binding)* ')'
//! binding := field                       // punning: binds a var named after the field
//!          | field ':' var               // explicit variable — the join spelling
//!          | field '==' value            // selection, schema-grammar-verbatim
//!          | field 'in' ?param           // set membership: field value ∈ the bound set
//! mask    := MASK ('|' MASK)* | ?param   // masks are sets of basics; '|' is set union
//! term    := var | ?param | literal
//! ```
//!
//! **Punning law (B, decided; the alternative is ledgered in
//! docs/architecture/70-api.md):** a bare field name binds a **clause-local variable
//! named after the field** — projection shorthand, Rust's struct-shorthand
//! instinct. The same punned name in two atoms of one clause is a compile
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
//!   literals `start..end` follow the same rule over both bounds.
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
//! - **Item-position `in`** is point membership (`Contains`): the right
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
/// (the id verbatim — the renderer's spelling).
enum Param {
    Named(Name),
    Index(u16),
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
    Duration(Name),
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

/// The `Allen` mask position: named masks joined by `|`, or a param.
enum Mask {
    Names(Vec<Name>),
    Param(Param),
}

/// One body item, in source order.
enum Item {
    Atom(Atom),
    Negated(Atom),
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

/// The aggregate ops the head grammar admits (Arg terms are the
/// renderer's honest extension, not grammar; the raw IR carries them).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AggOp {
    Sum,
    Min,
    Max,
    Count,
    CountDistinct,
    Pack,
}

impl AggOp {
    fn ir_name(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count => "Count",
            Self::CountDistinct => "CountDistinct",
            Self::Pack => "Pack",
        }
    }
}

/// One head term. A named position (`total: Sum(x)`) keeps the name at
/// the call site only — result columns are positional in the IR, and
/// variable names are a debugging sidecar the engine never stores.
enum HeadTerm {
    Var(Name),
    Duration(Name),
    Agg {
        op: AggOp,
        over: Option<Name>,
        /// The aggregated term is `Duration(v)` (Sum/Min/Max only).
        measure: bool,
    },
}

struct Clause {
    head: Vec<HeadTerm>,
    items: Vec<Item>,
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

/// Parses one `[-] int`, classifying by spelling: negative or
/// `i64`-suffixed is signed; a `u64` suffix (or none) is unsigned. Any
/// other suffix is rejected — the value sum holds exactly two integer
/// types.
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
            let signed = if text.ends_with("i64") {
                true
            } else if text.ends_with("u64") {
                if negative {
                    return fail(lit.span(), "query!: a negative literal cannot carry `u64`");
                }
                false
            } else if text.chars().all(|c| c.is_ascii_digit() || c == '_') {
                negative
            } else {
                return fail(
                    lit.span(),
                    format!(
                        "query!: unsupported integer suffix on `{text}` — the value \
                         types are u64 and i64"
                    ),
                );
            };
            Ok(Int {
                negative,
                text,
                signed,
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
            Ok(Param::Index(index))
        }
        _ => fail(question, "query!: `?` starts a param — `?name` or `?N`"),
    }
}

// ---------------------------------------------------------------------
// The grammar, production by production.
// ---------------------------------------------------------------------

const AGG_NAMES: [(&str, AggOp); 6] = [
    ("Sum", AggOp::Sum),
    ("Min", AggOp::Min),
    ("Max", AggOp::Max),
    ("Count", AggOp::Count),
    ("CountDistinct", AggOp::CountDistinct),
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
        return Ok(HeadTerm::Agg {
            op,
            over: None,
            measure: false,
        });
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
    Ok(HeadTerm::Agg {
        op,
        over: Some(over),
        measure,
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
                     takes Sum/Min/Max/Count/CountDistinct/Pack",
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
        return Ok(HeadTerm::Duration(var));
    }
    Ok(HeadTerm::Var(name))
}

fn parse_head(mut tokens: Tokens) -> Parse<Vec<HeadTerm>> {
    let mut head = Vec::new();
    while tokens.peek().is_some() {
        head.push(parse_head_term(&mut tokens)?);
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    Ok(head)
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

/// Parses one atom's bindings out of its paren group.
fn parse_bindings(mut tokens: Tokens) -> Parse<Vec<Binding>> {
    let mut bindings = Vec::new();
    while tokens.peek().is_some() {
        let field = expect_ident(&mut tokens, "a field name")?;
        if peek_punct(&mut tokens, ':') {
            expect_colon(&mut tokens, "the binding's `:`")?;
            let var = expect_ident(&mut tokens, "a variable")?;
            bindings.push(Binding::Var { field, var });
        } else if peek_punct(&mut tokens, '=') {
            expect_punct(&mut tokens, '=', "`==`")?;
            expect_punct(&mut tokens, '=', "`==`")?;
            let value = parse_sel_value(&mut tokens)?;
            bindings.push(Binding::Value { field, value });
        } else if peek_ident_text(&mut tokens).as_deref() == Some("in") {
            let in_kw = expect_ident(&mut tokens, "`in`")?;
            if !peek_punct(&mut tokens, '?') {
                return fail(
                    in_kw.span,
                    "query!: a binding's `in` takes a ?param bound to a set — \
                     interval membership is the `==` typing rule or a body item",
                );
            }
            let question = expect_punct(&mut tokens, '?', "`?`")?;
            let param = parse_param(&mut tokens, question)?;
            bindings.push(Binding::SetParam { field, param });
        } else {
            bindings.push(Binding::Pun(field));
        }
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        } else if let Some(extra) = tokens.next() {
            return fail(
                extra.span(),
                format!("query!: expected `,`, found `{extra}`"),
            );
        }
    }
    Ok(bindings)
}

fn parse_atom(tokens: &mut Tokens, relation: Name) -> Parse<Atom> {
    let (group, _) = take_paren_group(tokens, "the atom's bindings")?;
    Ok(Atom {
        relation,
        bindings: parse_bindings(group)?,
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
            return Ok(Term::Duration(var));
        }
        return Ok(Term::Var(name));
    }
    Ok(Term::Lit(parse_lit(tokens)?))
}

/// Parses the `Allen` mask position: `?param`, or mask names joined by
/// `|` (set union over the 13 basics; the names are `AllenMask`'s own
/// constants, so a typo is a compile error).
fn parse_mask(tokens: &mut Tokens) -> Parse<Mask> {
    if peek_punct(tokens, '?') {
        let question = expect_punct(tokens, '?', "`?`")?;
        return Ok(Mask::Param(parse_param(tokens, question)?));
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

/// Continues an item whose left term is already parsed: membership or a
/// comparison.
fn finish_term_item(tokens: &mut Tokens, lhs: Term) -> Parse<Item> {
    if peek_ident_text(tokens).as_deref() == Some("in") {
        tokens.next();
        let container = parse_term(tokens)?;
        return Ok(Item::Membership {
            element: lhs,
            container,
        });
    }
    let op = parse_cmp_op(tokens)?;
    let rhs = parse_term(tokens)?;
    Ok(Item::Cmp { op, lhs, rhs })
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
        let name = expect_ident(tokens, "an atom or Allen")?;
        if name.text == "Allen" {
            let (mut group, _) = take_paren_group(tokens, "Allen's three positions")?;
            let lhs = parse_term(&mut group)?;
            expect_punct(&mut group, ',', "`,`")?;
            let mask = parse_mask(&mut group)?;
            expect_punct(&mut group, ',', "`,`")?;
            let rhs = parse_term(&mut group)?;
            if let Some(extra) = group.next() {
                return fail(extra.span(), "query!: Allen takes exactly three positions");
            }
            return Ok(Item::Allen { lhs, mask, rhs });
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
            let (mut inner, _) = take_paren_group(tokens, "the measured variable")?;
            let var = expect_ident(&mut inner, "a variable")?;
            if let Some(extra) = inner.next() {
                return fail(extra.span(), "query!: Duration takes one variable");
            }
            return finish_term_item(tokens, Term::Duration(var));
        }
        return Ok(Item::Atom(parse_atom(tokens, name)?));
    }
    let lhs = parse_term(tokens)?;
    finish_term_item(tokens, lhs)
}

/// Parses one clause: `(head) | body ;`.
fn parse_clause(tokens: &mut Tokens) -> Parse<Clause> {
    let (head_group, head_span) = take_paren_group(tokens, "a clause head `(…)`")?;
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
                return fail(span, "query!: a clause body needs at least one atom");
            }
            tokens.next();
            break;
        }
        if tokens.peek().is_none() {
            return fail(Span::call_site(), "query!: a clause ends with `;`");
        }
        items.push(parse_item(tokens)?);
        if peek_punct(tokens, ',') {
            tokens.next();
        }
    }
    Ok(Clause { head, items })
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
#[derive(Default)]
struct Params {
    named: Vec<String>,
    saw_named: bool,
    saw_index: bool,
}

impl Params {
    fn resolve(&mut self, param: &Param) -> Parse<u16> {
        match param {
            Param::Named(name) => {
                if self.saw_index {
                    return fail(
                        name.span,
                        "query!: named and positional ?params cannot mix — \
                         pick one spelling per query",
                    );
                }
                self.saw_named = true;
                let position = self
                    .named
                    .iter()
                    .position(|existing| *existing == name.text)
                    .unwrap_or_else(|| {
                        self.named.push(name.text.clone());
                        self.named.len() - 1
                    });
                u16::try_from(position)
                    .map_or_else(|_| fail(name.span, "query!: too many params"), Ok)
            }
            Param::Index(index) => {
                if self.saw_named {
                    return fail(
                        Span::call_site(),
                        "query!: named and positional ?params cannot mix — \
                         pick one spelling per query",
                    );
                }
                self.saw_index = true;
                Ok(*index)
            }
        }
    }
}

/// One clause's variable scope: dense ids by first occurrence, plus the
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
                            "query!: head variable `{}` is not bound in the clause body",
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

struct Emitter<'a> {
    theory: &'a str,
    params: Params,
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
                let variant = if start.signed || end.signed {
                    "IntervalI64"
                } else {
                    "IntervalU64"
                };
                format!("{value}::{variant}({}, {})", int_text(start), int_text(end))
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
            Term::Duration(name) => format!(
                "::bumbledb::Term::Duration(::bumbledb::VarId({}))",
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

    /// One atom as an `Atom` expression — the relation and every field
    /// through the theory's id constants.
    fn atom(&mut self, scope: &mut Scope, atom: &Atom) -> Parse<String> {
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
            "::bumbledb::Atom {{ relation: {relation}, bindings: ::std::vec![{bindings}] }}"
        ))
    }

    fn mask(&mut self, mask: &Mask) -> Parse<String> {
        Ok(match mask {
            Mask::Names(names) => {
                let union = names
                    .iter()
                    .map(|name| format!("::bumbledb::AllenMask::{}", name.text))
                    .collect::<Vec<_>>()
                    .join(" | ");
                format!("::bumbledb::MaskTerm::Literal({union})")
            }
            Mask::Param(param) => {
                let id = self.params.resolve(param)?;
                format!("::bumbledb::MaskTerm::Param(::bumbledb::ParamId({id}))")
            }
        })
    }

    fn leaf(op: &str, lhs: &str, rhs: &str) -> String {
        format!(
            "::bumbledb::PredicateTree::Leaf(::bumbledb::Comparison {{ \
                 op: {op}, lhs: {lhs}, rhs: {rhs} }})"
        )
    }

    /// One head position as a `FindTerm` expression; every variable must
    /// already be body-bound.
    fn find(scope: &Scope, term: &HeadTerm) -> Parse<String> {
        Ok(match term {
            HeadTerm::Var(name) => format!(
                "::bumbledb::FindTerm::Var(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            HeadTerm::Duration(name) => format!(
                "::bumbledb::FindTerm::Duration(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            HeadTerm::Agg { op, over, measure } => {
                let op_expr = format!("::bumbledb::AggOp::{}", op.ir_name());
                match over {
                    None => format!(
                        "::bumbledb::FindTerm::Aggregate {{ op: {op_expr}, \
                             over: ::std::option::Option::None }}"
                    ),
                    Some(name) if *measure => format!(
                        "::bumbledb::FindTerm::AggregateDuration {{ op: {op_expr}, \
                             over: ::bumbledb::VarId({}) }}",
                        scope.head_var(name)?
                    ),
                    Some(name) => format!(
                        "::bumbledb::FindTerm::Aggregate {{ op: {op_expr}, \
                             over: ::std::option::Option::Some(::bumbledb::VarId({})) }}",
                        scope.head_var(name)?
                    ),
                }
            }
        })
    }

    /// One clause as a `Rule` expression. Items lower in source order;
    /// the IR buckets them (atoms, negated, predicates) — the renderer's
    /// normalized order.
    fn rule(&mut self, clause: &Clause) -> Parse<String> {
        let mut scope = Scope::default();
        let mut atoms = String::new();
        let mut negated = String::new();
        let mut predicates = String::new();
        for item in &clause.items {
            match item {
                Item::Atom(atom) => {
                    let _ = write!(atoms, "{},", self.atom(&mut scope, atom)?);
                }
                Item::Negated(atom) => {
                    let _ = write!(negated, "{},", self.atom(&mut scope, atom)?);
                }
                Item::Allen { lhs, mask, rhs } => {
                    let lhs = self.term(&mut scope, lhs)?;
                    let rhs = self.term(&mut scope, rhs)?;
                    let mask = self.mask(mask)?;
                    let op = format!("::bumbledb::CmpOp::Allen {{ mask: {mask} }}");
                    let _ = write!(predicates, "{},", Self::leaf(&op, &lhs, &rhs));
                }
                Item::Membership { element, container } => {
                    // `Contains` is interval-first; the notation reads
                    // point-first.
                    let element = self.term(&mut scope, element)?;
                    let container = self.term(&mut scope, container)?;
                    let _ = write!(
                        predicates,
                        "{},",
                        Self::leaf("::bumbledb::CmpOp::Contains", &container, &element)
                    );
                }
                Item::Cmp { op, lhs, rhs } => {
                    let lhs = self.term(&mut scope, lhs)?;
                    let rhs = self.term(&mut scope, rhs)?;
                    let op = format!("::bumbledb::CmpOp::{op}");
                    let _ = write!(predicates, "{},", Self::leaf(&op, &lhs, &rhs));
                }
            }
        }
        let mut finds = String::new();
        for term in &clause.head {
            let _ = write!(finds, "{},", Self::find(&scope, term)?);
        }
        Ok(format!(
            "::bumbledb::Rule {{ \
                 finds: ::std::vec![{finds}], \
                 atoms: ::std::vec![{atoms}], \
                 negated: ::std::vec![{negated}], \
                 predicates: ::std::vec![{predicates}] }}"
        ))
    }
}

fn expand(input: TokenStream) -> Parse<String> {
    let mut tokens: Tokens = input.into_iter().peekable();
    // The theory path, spliced verbatim before every `::CONST`.
    let mut theory = String::new();
    loop {
        match tokens.peek() {
            Some(TokenTree::Ident(_)) => {
                let name = expect_ident(&mut tokens, "the theory")?;
                theory.push_str(&name.text);
            }
            Some(TokenTree::Punct(p)) if p.as_char() == ':' => {
                expect_colon(&mut tokens, "the theory path's `::`")?;
                expect_punct(&mut tokens, ':', "the theory path's `::`")?;
                theory.push_str("::");
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => break,
            Some(other) => {
                return fail(
                    other.span(),
                    "query!: the shape is `query!(Theory { clauses })`",
                );
            }
            None => {
                return fail(
                    Span::call_site(),
                    "query!: the shape is `query!(Theory { clauses })`",
                );
            }
        }
    }
    if theory.is_empty() || theory.ends_with("::") {
        return fail(peek_span(&mut tokens), "query!: name the theory first");
    }
    let Some(TokenTree::Group(group)) = tokens.next() else {
        unreachable!("peeked the brace group");
    };
    if let Some(extra) = tokens.next() {
        return fail(extra.span(), "query!: nothing follows the clause block");
    }
    let mut clause_tokens: Tokens = group.stream().into_iter().peekable();
    let mut emitter = Emitter {
        theory: &theory,
        params: Params::default(),
    };
    let mut rules = String::new();
    while clause_tokens.peek().is_some() {
        let clause = parse_clause(&mut clause_tokens)?;
        let _ = write!(rules, "{},", emitter.rule(&clause)?);
    }
    if rules.is_empty() {
        return fail(group.span(), "query!: a query needs at least one clause");
    }
    Ok(format!(
        "{{ let rules = ::std::vec![{rules}]; \
             let head = ::bumbledb::Rule::head(&rules[0]); \
             ::bumbledb::Query {{ head, rules }} }}"
    ))
}

/// The query notation, lowered at compile time to the `ir::Query` value
/// (docs/architecture/20-query-ir.md § the query notation — the grammar
/// is the module doc's block, normative there). Names check through the
/// theory's id constants; everything semantic beyond names surfaces as
/// the validation roster's typed errors at `Db::prepare`.
///
/// ```ignore
/// let unavailable = bumbledb_query::query!(Calendar {
///     (person, during) | Busy(person, during), Allen(during, INTERSECTS, ?window);
///     (person, during) | Ooo(person, during),  Allen(during, INTERSECTS, ?window);
/// });
/// ```
///
/// # Panics
///
/// Never on malformed input — every diagnostic is a spanned
/// `compile_error!` at the offending token. The one internal `expect`
/// guards the generated code parsing as Rust, a bug in this macro if it
/// ever fires.
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    match expand(input) {
        Ok(code) => code.parse().expect("query!: generated code parses"),
        Err(error) => compile_error(&error),
    }
}
