//! The `query!` proc-macro — the blessed Rust query sugar, downstream and
//! quarantined: hosts may
//! depend on this crate, the engine never depends back, and the engine's
//! own surface stays pure-data IR. The notation is the statement grammar's query side,
//! promoted:
//! ```text
//! query     := cq | reach
//! cq        := interior* main
//! reach     := interior* recblock main
//! interior  := 'interior' derived '(' head ')' '|' body ';'
//! recblock  := 'rec' derived '(' head ')' '|' body ';'
//! main      := barerule+
//! barerule  := '(' head ')' '|' body ';'
//!                                        // consecutive `interior derived(...)` lines
//!                                        //   union into one Interior; consecutive
//!                                        //   `rec derived(...)` lines union into
//!                                        //   one Rec (a line whose body has an atom
//!                                        //   naming derived is a rec arm, else base);
//!                                        //   an all-bare query is a CQ with empty
//!                                        //   interiors; a named head without
//!                                        //   `interior` / `rec` is a compile
//!                                        //   error (the former named-head sneak)
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
//!          | derived '(' var (',' var)* ')'
//!                                        // ordered dense: a body atom may name an
//!                                        //   interior or the rec where it names a
//!                                        //   relation; bare idents bind its head
//!                                        //   POSITIONS left to right from 0 —
//!                                        //   positional, never nominal
//!          | 'interior' integer '(' var (',' var)* ')'
//!          | 'interior' integer '(' pbind (',' pbind)* ')'
//!                                        // nameless: the renderer's `interior {id}`
//!                                        //   spelling of a derived-table atom
//!          | derived '(' pbind (',' pbind)* ')'
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
//! derived := lowercase ident             // macro-LOCAL: resolved at expansion, never
//!                                        //   in the IR or the fingerprint; relations
//!                                        //   are UpperCamel, so an interior/rec spelled
//!                                        //   like a relation is unwritable; `and`,
//!                                        //   `or`, `interior`, and `rec` are
//!                                        //   reserved
//! ```
//! **Condition trees are notation (ruled 2026-07-23, R9):** `and(...)`/
//! `or(...)` admit any boolean combination of comparisons as one item —
//! comparison leaves only, exactly the IR's `ConditionTree` (atoms,
//! negation, and the binding membership stay items) and an exact mirror
//! of the TS condition grammar. Validation distributes the trees to DNF
//! engine-side; the renderer's functional forms are grammar, so the
//! named after the field** — projection shorthand, Rust's struct-shorthand
//! after its referencing field; one named otherwise is written

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use std::fmt::Write as _;
use std::iter::Peekable;

type Tokens = Peekable<proc_macro::token_stream::IntoIter>;

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

// The surface AST — names and spans, resolved to ids after the parse.

#[derive(Clone)]
struct Name {
    text: String,
    span: Span,
}

/// Both spellings carry their token's span: every refusal points at the
/// offending param.
enum Param {
    Named(Name),
    Index { index: u16, span: Span },
}

struct Int {
    negative: bool,
    text: String,
    signed: bool,
}

enum Lit {
    Bool(bool),
    Int(Int),

    Interval {
        start: Int,
        end: Int,
    },

    Str(String),

    Bytes(String),
}

enum SelValue {
    Lit(Lit),
    Param(Param),

    Handle {
        qualifier: Option<Name>,
        handle: Name,
    },
}

enum Term {
    Var(Name),
    Param(Param),
    Lit(Lit),
}

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

enum Mask {
    Names(Vec<Name>),
}

enum Leaf {
    Allen {
        lhs: Term,
        mask: Mask,
        rhs: Term,
    },

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

enum Cond {
    Leaf(Leaf),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

enum Item {
    Atom(Atom),
    Negated(Atom),
    Cond(Cond),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AggOp {
    Sum,
    Min,
    Max,
    Count,
    Pack,
}

impl AggOp {
    fn fold_ir_name(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count | Self::Pack => {
                unreachable!("Count and Pack are sibling FindTerm constructors")
            }
        }
    }
}

enum HeadTerm {
    Var(Name),
    Count,
    Agg { op: AggOp, over: Name },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RuleKind {
    Interior,
    Rec,
}

enum ParsedRule {
    Bare {
        head: Vec<HeadTerm>,
        items: Vec<Item>,
    },
    Interior {
        name: Name,
        head: Vec<HeadTerm>,
        items: Vec<Item>,
    },
    Rec {
        name: Name,
        head: Vec<HeadTerm>,
        items: Vec<Item>,
    },
}

impl ParsedRule {
    fn head(&self) -> &[HeadTerm] {
        match self {
            Self::Bare { head, .. } | Self::Interior { head, .. } | Self::Rec { head, .. } => head,
        }
    }

    fn items(&self) -> &[Item] {
        match self {
            Self::Bare { items, .. } | Self::Interior { items, .. } | Self::Rec { items, .. } => {
                items
            }
        }
    }
}

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

fn nameless_interior_atom(tokens: &mut Tokens) -> bool {
    if peek_ident_text(tokens).as_deref() != Some("interior") {
        return false;
    }
    let mut ahead = tokens.clone();
    ahead.next();
    match ahead.next() {
        Some(TokenTree::Literal(lit)) if lit.to_string().chars().all(|c| c.is_ascii_digit()) => {
            matches!(ahead.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
        }
        _ => false,
    }
}

fn take_nameless_interior(tokens: &mut Tokens) -> Parse<Name> {
    let kw = expect_ident(tokens, "`interior`")?;
    match tokens.next() {
        Some(TokenTree::Literal(lit)) => {
            let text = lit.to_string();
            if text.chars().all(|c| c.is_ascii_digit()) {
                Ok(Name {
                    text,
                    span: lit.span(),
                })
            } else {
                fail(
                    lit.span(),
                    "query!: expected an interior id after `interior`",
                )
            }
        }
        Some(other) => fail(
            other.span(),
            "query!: expected an interior id after `interior`",
        ),
        None => fail(kw.span, "query!: expected an interior id after `interior`"),
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

/// Consumes `:` while refusing `:-` — the borrowed grammar must not parse,
/// anywhere.
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

fn is_int_text(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_digit()) && !text.contains('.')
}

/// The type suffix is stripped before this check, so no branch order can invert
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

fn finish_int(tokens: &mut Tokens, start: Int) -> Parse<Lit> {
    if peek_punct(tokens, '.') {
        tokens.next();
        expect_punct(tokens, '.', "the interval literal's `..`")?;
        let end = parse_int(tokens, "the interval literal's end bound")?;
        return Ok(Lit::Interval { start, end });
    }
    Ok(Lit::Int(start))
}

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

fn parse_agg(tokens: &mut Tokens, op: AggOp) -> Parse<HeadTerm> {
    if op == AggOp::Count {
        return Ok(HeadTerm::Count);
    }
    let (mut arg, _) = take_paren_group(tokens, "the aggregate's argument")?;
    let first = expect_ident(&mut arg, "a variable")?;
    if first.text == "Duration" && matches!(arg.peek(), Some(TokenTree::Group(_))) {
        return fail(
            first.span,
            "query!: Duration is gone — compute end − start on the host",
        );
    }
    let over = first;
    if let Some(extra) = arg.next() {
        return fail(extra.span(), "query!: the aggregate takes one argument");
    }
    Ok(HeadTerm::Agg { op, over })
}

/// Params are refused here: a param is an execution input, not a result column.
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
        return fail(
            name.span,
            "query!: Duration is gone — compute end − start on the host",
        );
    }
    Ok(HeadTerm::Var(name))
}

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

/// Which one is legal is the atom's source's business, decided at emission (the
/// derived-table list exists only after every rule has parsed — mutual
/// recursion reads forward).
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
            return fail(
                name.span,
                "query!: Duration is gone — compute end − start on the host",
            );
        }
        return Ok(Term::Var(name));
    }
    Ok(Term::Lit(parse_lit(tokens)?))
}

/// Mask params are refused — the mask is a literal.
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

/// The condition-tree refusal, one message for every non-comparison shape under
/// `and`/`or`.
fn tree_refusal<T>(span: Span) -> Parse<T> {
    fail(
        span,
        "query!: a condition tree takes comparisons only — atoms, negation, \
         and the binding membership stay body items \
         (docs/architecture/20-query-ir.md § the query notation)",
    )
}

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

fn parse_cond(tokens: &mut Tokens) -> Parse<Cond> {
    if peek_punct(tokens, '!') {

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
            "Duration" => fail(
                name.span,
                "query!: Duration is gone — compute end − start on the host",
            ),
            _ => tree_refusal(name.span),
        };
    }
    let lhs = parse_term(tokens)?;
    Ok(Cond::Leaf(finish_term_leaf(tokens, lhs)?))
}

/// Whether the token after a `Name (…)` shape continues a term item — i.e.
fn continues_as_term(tokens: &mut Tokens) -> bool {
    match tokens.peek() {
        Some(TokenTree::Punct(p)) => matches!(p.as_char(), '=' | '!' | '<' | '>'),
        Some(TokenTree::Ident(ident)) => ident.to_string() == "in",
        _ => false,
    }
}

fn parse_item(tokens: &mut Tokens) -> Parse<Item> {
    if peek_punct(tokens, '!') {

        tokens.next();
        if nameless_interior_atom(tokens) {
            let name = take_nameless_interior(tokens)?;
            return Ok(Item::Negated(parse_atom(tokens, name)?));
        }
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
    if nameless_interior_atom(tokens) {
        let name = take_nameless_interior(tokens)?;
        return Ok(Item::Atom(parse_atom(tokens, name)?));
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

            // `and(…)`/`or(…)` is always a tree (ruled 2026-07-23, R9).
            "and" => return Ok(Item::Cond(Cond::And(parse_tree_children(tokens, &name)?))),
            "or" => return Ok(Item::Cond(Cond::Or(parse_tree_children(tokens, &name)?))),
            _ => {}
        }

        let mut ahead = tokens.clone();
        ahead.next(); 
        if continues_as_term(&mut ahead) {
            return fail(
                name.span,
                "query!: Duration is gone — compute end − start on the host",
            );
        }
        return Ok(Item::Atom(parse_atom(tokens, name)?));
    }
    let lhs = parse_term(tokens)?;
    Ok(Item::Cond(Cond::Leaf(finish_term_leaf(tokens, lhs)?)))
}

fn validate_derived_name(name: &Name) -> Parse<()> {
    if name.text == "and" || name.text == "or" || name.text == "interior" || name.text == "rec" {
        return fail(
            name.span,
            format!(
                "query!: `{}` is reserved — `and`/`or` are the condition grammar, \
                 `interior`/`rec` introduce derived tables \
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

fn parse_derived_name(tokens: &mut Tokens, kind: RuleKind, kw_span: Span) -> Parse<Name> {
    match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let name = expect_ident(tokens, "a derived-table name")?;
            validate_derived_name(&name)?;
            Ok(name)
        }
        Some(TokenTree::Literal(lit)) if kind == RuleKind::Interior => {
            let text = lit.to_string();
            if text.chars().all(|c| c.is_ascii_digit()) {
                let span = lit.span();
                tokens.next();
                Ok(Name { text, span })
            } else {
                fail(
                    lit.span(),
                    "query!: expected an interior id after `interior`",
                )
            }
        }
        Some(TokenTree::Group(_)) if kind == RuleKind::Rec => Ok(Name {
            text: "rec".into(),
            span: kw_span,
        }),
        Some(other) => fail(
            other.span(),
            "query!: expected a derived-table name, an interior id, or a rec head",
        ),
        None => fail(
            kw_span,
            "query!: expected a derived-table name or a rec head",
        ),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one rule is one grammar production; splitting hides the keyword/head/body sequence"
)]
fn parse_rule(tokens: &mut Tokens) -> Parse<ParsedRule> {
    let intro = match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let ident = expect_ident(tokens, "a rule")?;
            if peek_punct(tokens, ':') {
                // `derived :- …` must not parse — the refusal fires before

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
                "interior" | "rec" => {
                    let kind = if ident.text == "interior" {
                        RuleKind::Interior
                    } else {
                        RuleKind::Rec
                    };
                    let derived = parse_derived_name(tokens, kind, ident.span)?;
                    Some((kind == RuleKind::Interior, derived))
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
                            "query!: named heads require `interior` or `rec` — \
                             write `interior {}(...)` or `rec {}(...)`",
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
        Some((false, name)) => ParsedRule::Rec { name, head, items },
    })
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BindingStyle {
    Empty,
    Bare,
    Numeric,
}

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
            Term::Lit(lit) => format!("::bumbledb::Term::Literal({})", Self::lit(lit)),
        })
    }

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

    /// and an explicitly indexed dense in-order variable list is refused

    fn interior_atom(&mut self, scope: &mut Scope, atom: &Atom, interior: u32) -> Parse<String> {
        let bindings = self.interior_bindings(scope, atom)?;
        Ok(format!(
            "::bumbledb::Atom {{ source: ::bumbledb::AtomSource::Interior(::bumbledb::InteriorId({interior})), bindings: ::std::vec![{bindings}] }}"
        ))
    }

    fn interior_bindings(&mut self, scope: &mut Scope, atom: &Atom) -> Parse<String> {
        if interior_style(atom)? == BindingStyle::Bare {

            let mut bindings = String::new();
            for (position, binding) in atom.bindings.iter().enumerate() {
                let Binding::Pun(name) = binding else {
                    unreachable!("the style split sealed an all-bare atom");
                };
                let term = Self::var(scope.intern(name)?);
                let _ = write!(bindings, "(::bumbledb::FieldId({position}), {term}),");
            }
            return Ok(bindings);
        }

        // ordered form's meaning respelled — refused, one spelling per

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
        Ok(bindings)
    }

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

    fn cond(&mut self, scope: &mut Scope, cond: &Cond) -> Parse<String> {
        Ok(match cond {
            Cond::Leaf(Leaf::Allen { lhs, mask, rhs }) => {
                let lhs = self.term(scope, lhs)?;
                let rhs = self.term(scope, rhs)?;
                let mask = Self::mask(mask);
                let op = format!("::bumbledb::CmpOp::Allen {{ mask: {mask} }}");
                Self::leaf(&op, &lhs, &rhs)
            }

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

    fn find(scope: &Scope, term: &HeadTerm) -> Parse<String> {
        Ok(match term {
            HeadTerm::Var(name) => format!(
                "::bumbledb::FindTerm::Var(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            HeadTerm::Count => "::bumbledb::FindTerm::Count".to_string(),
            HeadTerm::Agg {
                op: AggOp::Pack,
                over,
            } => format!(
                "::bumbledb::FindTerm::Pack {{ over: ::bumbledb::VarId({}) }}",
                scope.head_var(over)?
            ),
            HeadTerm::Agg { op, over } => format!(
                "::bumbledb::FindTerm::Aggregate {{ op: ::bumbledb::FoldOp::{}, \
                     over: ::bumbledb::VarId({}) }}",
                op.fold_ir_name(),
                scope.head_var(over)?
            ),
        })
    }

    fn projection_vars(scope: &Scope, head: &[HeadTerm]) -> Parse<String> {
        let mut finds = String::new();
        for term in head {
            match term {
                HeadTerm::Var(name) => {
                    let _ = write!(finds, "::bumbledb::VarId({}),", scope.head_var(name)?);
                }
                HeadTerm::Count => {
                    return fail(
                        Span::call_site(),
                        "query!: an interior/rec head projects bound variables only — \
                         Count is a fold over a finished set",
                    );
                }
                HeadTerm::Agg { over, .. } => {
                    return fail(
                        over.span,
                        "query!: an interior/rec head projects bound variables only — \
                         aggregates fold a finished set",
                    );
                }
            }
        }
        Ok(finds)
    }

    fn body_parts(&mut self, rule: &ParsedRule) -> Parse<(Scope, String, String, String)> {
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
        Ok((scope, atoms, negated, conditions))
    }

    fn rule(&mut self, rule: &ParsedRule) -> Parse<String> {
        let (scope, atoms, negated, conditions) = self.body_parts(rule)?;
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

    fn projection_rule(&mut self, rule: &ParsedRule) -> Parse<String> {
        let (scope, atoms, negated, conditions) = self.body_parts(rule)?;
        let finds = Self::projection_vars(&scope, rule.head())?;
        Ok(format!(
            "::bumbledb::ProjectionRule {{ \
                 finds: ::std::vec![{finds}], \
                 atoms: ::std::vec![{atoms}], \
                 negated: ::std::vec![{negated}], \
                 conditions: ::std::vec![{conditions}] }}"
        ))
    }

    fn rec_rule(&mut self, rule: &ParsedRule) -> Parse<String> {
        let (scope, atoms, negated, conditions) = self.body_parts(rule)?;
        if !negated.is_empty() {
            return fail(
                Span::call_site(),
                "query!: a rec rule negates no table — negation through \
                 the cycle is unrepresentable",
            );
        }
        let finds = Self::projection_vars(&scope, rule.head())?;
        Ok(format!(
            "::bumbledb::RecRule {{ \
                 finds: ::std::vec![{finds}], \
                 atoms: ::std::vec![{atoms}], \
                 conditions: ::std::vec![{conditions}] }}"
        ))
    }

    fn rec_step(&mut self, rule: &ParsedRule, rec_name: &str) -> Parse<String> {
        let mut scope = Scope::default();
        let mut self_bindings = None;
        let mut atoms = String::new();
        let mut conditions = String::new();
        for item in rule.items() {
            match item {
                Item::Negated(atom) => {
                    return fail(
                        atom.relation.span,
                        "query!: a rec rule negates no table — negation through \
                         the cycle is unrepresentable",
                    );
                }
                Item::Cond(cond) => {
                    let _ = write!(conditions, "{},", self.cond(&mut scope, cond)?);
                }
                Item::Atom(atom) if atom.relation.text == rec_name => {
                    if self_bindings.is_some() {
                        return fail(atom.relation.span, "query!: a rec step has one self-atom");
                    }
                    self_bindings = Some(self.interior_bindings(&mut scope, atom)?);
                }
                Item::Atom(atom) => {
                    let _ = write!(atoms, "{},", self.atom(&mut scope, atom)?);
                }
            }
        }
        let finds = Self::projection_vars(&scope, rule.head())?;
        let Some(self_bindings) = self_bindings else {
            return fail(
                Span::call_site(),
                "query!: a rec step is missing its self-atom",
            );
        };
        Ok(format!(
            "::bumbledb::RecStep {{ \
                 finds: ::std::vec![{finds}], \
                 self_bindings: ::std::vec![{self_bindings}], \
                 atoms: ::std::vec![{atoms}], \
                 conditions: ::std::vec![{conditions}] }}"
        ))
    }
}

/// Parses the leading theory path (`Theory` or `crate::path::Theory`) — spliced
/// verbatim before every `::CONST` — leaving the brace group as the next token.
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

fn names_derived(rule: &ParsedRule, derived: &str) -> bool {
    rule.items().iter().any(|item| match item {
        Item::Atom(atom) | Item::Negated(atom) => atom.relation.text == derived,
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

enum Classified {
    Cq {
        interiors: Vec<InteriorGroup>,
        main: Vec<ParsedRule>,
    },
    Reach {
        interiors: Vec<InteriorGroup>,
        rec: RecGroup,
        main: Vec<ParsedRule>,
    },
}

enum Phase {
    Interiors,
    Rec,
    Main,
}

#[expect(
    clippy::too_many_lines,
    reason = "phase machine plus exhaustive compile errors for this cut live in one walk"
)]
fn classify(parsed: Vec<ParsedRule>, block: Span) -> Parse<Classified> {
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
                        "query!: `interior` cannot follow `rec` — declaration \
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
                            "query!: `{0}` cannot be both `interior` and `rec` — \
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
            ParsedRule::Rec { name, head, items } => {
                let rule = ParsedRule::Rec {
                    name: name.clone(),
                    head,
                    items,
                };
                if matches!(phase, Phase::Main) {
                    return fail(
                        name.span,
                        "query!: `rec` cannot follow a bare rule — declaration \
                         order is interiors, then rec, then main",
                    );
                }
                phase = Phase::Rec;
                if interiors.iter().any(|group| group.name.text == name.text) {
                    return fail(
                        name.span,
                        format!(
                            "query!: `{0}` cannot be both `interior` and `rec` — \
                             derived names are unique",
                            name.text
                        ),
                    );
                }
                let self_atom = if name.text == "rec" {
                    interiors.len().to_string()
                } else {
                    name.text.clone()
                };
                let is_rec_arm = names_derived(&rule, &self_atom);
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
                            "query!: at most one `rec` name this cut — a \
                             second rec is unwritable",
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
            "query!: a query needs a bare main rule — `interior` / `rec` \
             declare derived tables; the answer is the unnamed rules",
        );
    }
    if let Some(group) = &rec {
        if group.base.is_empty() {
            return fail(
                group.name.span,
                format!(
                    "query!: `rec {}` has no base arm — a line whose body \
                     does not name `{}` is a base arm",
                    group.name.text, group.name.text
                ),
            );
        }
        if group.rec.is_empty() {
            return fail(
                group.name.span,
                format!(
                    "query!: `rec {}` has no rec arm — a line whose body \
                     names `{}` (positive or negated) is a rec arm",
                    group.name.text, group.name.text
                ),
            );
        }
    }
    Ok(match rec {
        None => Classified::Cq { interiors, main },
        Some(rec) => Classified::Reach {
            interiors,
            rec,
            main,
        },
    })
}

fn emit_projection_rules(emitter: &mut Emitter<'_>, rules: &[ParsedRule]) -> Parse<String> {
    let mut out = String::new();
    for rule in rules {
        let _ = write!(out, "{},", emitter.projection_rule(rule)?);
    }
    Ok(out)
}

fn emit_rec_base(emitter: &mut Emitter<'_>, rules: &[ParsedRule]) -> Parse<String> {
    let mut out = String::new();
    for rule in rules {
        let _ = write!(out, "{},", emitter.rec_rule(rule)?);
    }
    Ok(out)
}

fn emit_rec_steps(
    emitter: &mut Emitter<'_>,
    rules: &[ParsedRule],
    rec_name: &str,
) -> Parse<String> {
    let mut out = String::new();
    for rule in rules {
        let _ = write!(out, "{},", emitter.rec_step(rule, rec_name)?);
    }
    Ok(out)
}

fn nonempty(items: &str, what: &str) -> String {
    format!(
        "::bumbledb::NonEmpty::from_vec(::std::vec![{items}]).expect(\"query!: nonempty {what}\")"
    )
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
    let classified = classify(parsed, group.span())?;
    match classified {
        Classified::Cq { interiors, main } => emit_cq(&theory, &interiors, &main),
        Classified::Reach {
            interiors,
            rec,
            main,
        } => emit_reach(&theory, &interiors, &rec, &main),
    }
}

fn rec_derived_name(rec: &RecGroup, rec_id: usize) -> String {
    if rec.name.text == "rec" {
        rec_id.to_string()
    } else {
        rec.name.text.clone()
    }
}

fn emit_interiors(
    emitter: &mut Emitter<'_>,
    interiors: &[InteriorGroup],
) -> Parse<(String, String)> {
    let mut lets = String::new();
    let mut defs = String::new();
    for (index, group) in interiors.iter().enumerate() {
        let rules = emit_projection_rules(emitter, &group.rules)?;
        let _ = write!(lets, "let interior{index}_rules = ::std::vec![{rules}]; ");
        let _ = write!(
            defs,
            "::bumbledb::Interior {{ rules: interior{index}_rules }},"
        );
    }
    Ok((lets, defs))
}

fn emit_cq(theory: &str, interiors: &[InteriorGroup], main: &[ParsedRule]) -> Parse<String> {
    let derived: Vec<String> = interiors
        .iter()
        .map(|group| group.name.text.clone())
        .collect();
    let mut emitter = Emitter {
        theory,
        params: Params::default(),
        derived,
    };
    let (lets, interior_defs) = emit_interiors(&mut emitter, interiors)?;
    let main_rules = emit_rules(&mut emitter, main)?;
    Ok(format!(
        "{{ {lets}let rules = ::std::vec![{main_rules}]; \
             let head = ::bumbledb::Rule::head(&rules[0]); \
             ::bumbledb::Query::cq(::std::vec![{interior_defs}], head, rules) }}"
    ))
}

fn emit_reach(
    theory: &str,
    interiors: &[InteriorGroup],
    rec: &RecGroup,
    main: &[ParsedRule],
) -> Parse<String> {
    let mut derived: Vec<String> = interiors
        .iter()
        .map(|group| group.name.text.clone())
        .collect();
    derived.push(rec_derived_name(rec, interiors.len()));
    let mut emitter = Emitter {
        theory,
        params: Params::default(),
        derived,
    };
    let (mut lets, interior_defs) = emit_interiors(&mut emitter, interiors)?;
    let rec_name = rec_derived_name(rec, interiors.len());
    let base = emit_rec_base(&mut emitter, &rec.base)?;
    let step = emit_rec_steps(&mut emitter, &rec.rec, &rec_name)?;
    let _ = write!(
        lets,
        "let rec_base = {base}; let rec_step = {step}; ",
        base = nonempty(&base, "rec base"),
        step = nonempty(&step, "rec step"),
    );
    let main_rules = emit_rules(&mut emitter, main)?;
    Ok(format!(
        "{{ {lets}let rules = ::std::vec![{main_rules}]; \
             let head = ::bumbledb::Rule::head(&rules[0]); \
             ::bumbledb::Query::reach( \
                 ::std::vec![{interior_defs}], \
                 ::bumbledb::Rec {{ base: rec_base, rec: rec_step }}, \
                 head, \
                 rules) }}"
    ))
}

/// The query notation, lowered at compile time to the `ir::Query` value
/// . Names check through the
/// theory's id constants; derived-table names are macro-local and never
/// survive expansion (the IR carries bare `InteriorId`s); everything
/// semantic beyond names surfaces as the validation roster's typed
/// errors at `Db::prepare`.
/// ```ignore
/// let unavailable = bumbledb_query::query!(Calendar {
///     (person, during) | Busy(person, during), Allen(during, INTERSECTS, ?window);
///     (person, during) | Ooo(person, during),  Allen(during, INTERSECTS, ?window);
/// });
/// // `rec` / `interior` declare derived tables; a body atom may
/// // name one (bare idents bind head POSITIONS, ordered dense — left to
/// // right from 0); bare rules are the main query.
/// let reachable = bumbledb_query::query!(Ledger {
///     rec reach(c, a) | OrgParent(child: c, parent: a);
///     rec reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
///     (c, a) | reach(c, a);
/// });
/// ```
/// # Panics
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
