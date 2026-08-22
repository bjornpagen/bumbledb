//! The `schema!` proc-macro: bumbledb's declarative schema
//! surface. A small, rigid grammar — this is Rust-side declaration, not a
//! query language — hand-parsed over the raw token stream (no `syn`, no
//! `quote`: the grammar is not Rust syntax and the dependency would buy
//! nothing).
//! ```text
//! schema! {
//!     pub Ledger;
//!
//!     relation Holder  { id: u64 as HolderId, fresh, name: str }
//!     relation Account {
//!         id:     u64 as AccountId, fresh,
//!         holder: u64 as HolderId,
//!         kind:   u64 as KindId,
//!         active: interval<i64> as ActiveDuring,
//!     }
//!     relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }
//!
//!     Account(holder) <= Holder(id);
//!     Account(id | kind == Savings) == SavingsTerms(account);
//!     SavingsTerms(account) -> SavingsTerms;
//! }
//! ```
//! The header `pub Ledger;` is the invocation's first item and names the
//! schema: it expands to `pub struct Ledger;` implementing
//! `bumbledb::Theory`, the value `Db::create(path, Ledger)` takes and
//! the typestate `Db<Ledger>` carries. Multiple schemas coexist in one
//! module — their headers disambiguate.
//! Types: `bool`, `u64`, `i64`, `str`, `bytes<N>` (N ∈ 1..=64 — the
//! ```text
//! closed relation Status as StatusId = { Open, Frozen, Closed };
//! closed relation Kind as KindId {
//!     mastered: bool,
//! } = {
//!     DirectPass { mastered: true },
//!     Failed     { mastered: false },
//! };
//! ```
//! `as NewType` is required (the handle needs a host type); the column
//! block is optional; the extension block is non-empty, each row carrying
//! every declared column exactly once (missing/extra/duplicate columns,
//! duplicate handles, and type-mismatched literals are expansion panics
//! naming the offender). The emission per closed relation: the **host
//! enum** (an emission, not a type — the engine's vocabulary is
use bumbledb_theory::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FieldSpec, LiteralAt, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SchemaSpec, SideSpec, SpecIssue, StatementSide, StatementSpec,
    WeightSpec,
};
use bumbledb_theory::schema::{
    Generation, IntervalElement, LiteralSet, SchemaDescriptor, Side as SideDescriptor,
    StatementDescriptor, ValueType, Weight,
};
use bumbledb_theory::{Interval, Value};
use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::iter::Peekable;

fn element_rust(element: IntervalElement) -> &'static str {
    match element {
        IntervalElement::U64 => "u64",
        IntervalElement::I64 => "i64",
    }
}

fn element_suffix(element: IntervalElement) -> &'static str {
    match element {
        IntervalElement::U64 => "U64",
        IntervalElement::I64 => "I64",
    }
}

#[derive(Debug, Clone)]
enum FieldTy {
    Bool,
    U64,
    I64,
    Str,

    FixedBytes(u64),

    Interval(IntervalElement),
    FixedInterval(IntervalElement, u64),
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: FieldTy,
    newtype: Option<String>,
    fresh: bool,
}

#[derive(Debug, Clone)]
struct Relation {
    name: String,

    fields: Vec<Field>,

    newtype_span: Option<Span>,

    closed: Option<Closed>,
}

#[derive(Debug, Clone)]
struct Closed {
    rows: Vec<ClosedRow>,
}

#[derive(Debug, Clone)]
struct ClosedRow {
    handle: String,
    values: Vec<(String, Literal)>,
}

#[derive(Debug, Clone)]
enum Literal {
    Bool(bool),

    Int {
        negative: bool,
        text: String,
    },

    Handle(String, Span),

    Str(String),

    Bytes(String),

    Interval {
        start: (bool, String),
        end: (bool, String),
    },
}

#[derive(Debug, Clone)]
enum Literals {
    One(Literal),
    Many(Vec<Literal>),
}

#[derive(Debug, Clone)]
struct Binding {
    field: String,
    field_span: Span,
    literals: Literals,
    set_span: Option<Span>,
}

#[derive(Debug, Clone)]
struct Side {
    relation: String,
    relation_span: Span,
    projection: Vec<(String, Span)>,
    selection: Vec<Binding>,
}

#[derive(Debug, Clone)]
enum Statement {
    Functionality {
        relation: String,
        relation_span: Span,
        projection: Vec<(String, Span)>,
    },
    Containment {
        source: Side,
        target: Side,
        bidirectional: bool,
    },

    Capacity {
        source: Side,
        weight: WeightSpec,
        weight_span: Option<Span>,
        window: CapacityWindowSpec,
        window_span: Span,
        target: Side,
    },
}

struct SchemaAst {
    name: String,
    relations: Vec<Relation>,
    statements: Vec<Statement>,
}

type Tokens = Peekable<proc_macro::token_stream::IntoIter>;

fn expect_ident(tokens: &mut Tokens, what: &str) -> String {
    spanned_ident(tokens, what).0
}

fn spanned_ident(tokens: &mut Tokens, what: &str) -> (String, Span) {
    match tokens.next() {
        Some(TokenTree::Ident(ident)) => (ident.to_string(), ident.span()),
        other => panic!("schema!: expected {what}, found {other:?}"),
    }
}

fn expect_punct(tokens: &mut Tokens, ch: char) {
    match tokens.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == ch => {}
        other => panic!("schema!: expected `{ch}`, found {other:?}"),
    }
}

fn peek_ident(tokens: &mut Tokens) -> Option<String> {
    match tokens.peek() {
        Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
        _ => None,
    }
}

fn peek_punct(tokens: &mut Tokens, ch: char) -> bool {
    matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
}

fn peek_path_dot(tokens: &mut Tokens) -> bool {
    matches!(
        tokens.peek(),
        Some(TokenTree::Punct(p)) if p.as_char() == '.' && p.spacing() == Spacing::Alone
    )
}

fn take_group(tokens: &mut Tokens, delimiter: Delimiter, what: &str) -> TokenStream {
    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == delimiter => group.stream(),
        other => panic!("schema!: expected {what}, found {other:?}"),
    }
}

fn reject_deleted_word(word: &str) {
    assert!(
        !matches!(word, "unique" | "fk"),
        "schema!: field-level constraints do not exist; write a statement"
    );
    assert!(
        word != "enum",
        "schema!: the enum type is deleted — a vocabulary is a closed relation \
         (`closed relation K as KId = {{ A, B }};` plus `Rel(k) <= K(id);`)"
    );
}

fn parse_relation(name: String, body: TokenStream) -> Relation {
    let mut relation = Relation {
        name,
        fields: Vec::new(),
        newtype_span: None,
        closed: None,
    };
    let mut tokens = body.into_iter().peekable();
    while tokens.peek().is_some() {
        let ident = expect_ident(&mut tokens, "a field name");
        reject_deleted_word(&ident);
        expect_punct(&mut tokens, ':');
        relation.fields.push(parse_field(ident, &mut tokens));
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    relation
}

fn parse_field(name: String, tokens: &mut Tokens) -> Field {
    let ty_name = expect_ident(tokens, "a type (bool/u64/i64/str/bytes<N>/interval)");
    reject_deleted_word(&ty_name);
    let ty = match ty_name.as_str() {
        "bool" => FieldTy::Bool,
        "u64" => FieldTy::U64,
        "i64" => FieldTy::I64,
        "str" => FieldTy::Str,

        "bytes" => {
            assert!(
                peek_punct(tokens, '<'),
                "schema!: unknown type `bytes` — write `bytes<N>` (the width is the type; \
                 variable-width bytes does not exist)"
            );
            expect_punct(tokens, '<');
            let (negative, text) = parse_int(tokens, "the bytes<N> width");
            assert!(!negative, "schema!: bytes<N> width must be positive");
            expect_punct(tokens, '>');
            let width = u64_text(&text)
                .unwrap_or_else(|| panic!("schema!: malformed bytes<N> width `{text}`"));
            FieldTy::FixedBytes(width)
        }
        "interval" => {
            expect_punct(tokens, '<');
            let element = match expect_ident(tokens, "an interval element (i64/u64)").as_str() {
                "u64" => IntervalElement::U64,
                "i64" => IntervalElement::I64,
                other => panic!("schema!: interval element must be i64 or u64, found `{other}`"),
            };
            let ty = match parse_interval_width(&name, tokens) {
                None => FieldTy::Interval(element),
                Some(width) => FieldTy::FixedInterval(element, width),
            };
            expect_punct(tokens, '>');
            ty
        }
        other => panic!("schema!: unknown type `{other}`"),
    };
    let mut field = Field {
        name,
        ty,
        newtype: None,
        fresh: false,
    };
    if peek_ident(tokens).as_deref() == Some("as") {
        tokens.next();
        assert!(
            matches!(
                field.ty,
                FieldTy::U64
                    | FieldTy::I64
                    | FieldTy::FixedBytes(_)
                    | FieldTy::Interval(_)
                    | FieldTy::FixedInterval(..)
            ),
            "schema!: `as NewType` applies to u64/i64/bytes<N>/interval fields only"
        );
        field.newtype = Some(expect_ident(tokens, "a newtype name"));
    }

    if peek_punct(tokens, ',') {
        let mut lookahead = tokens.clone();
        lookahead.next();
        if let Some(TokenTree::Ident(ident)) = lookahead.peek() {
            let word = ident.to_string();
            lookahead.next();
            let is_field_name =
                matches!(lookahead.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ':');
            if !is_field_name {
                reject_deleted_word(&word);
                assert_eq!(
                    word, "fresh",
                    "schema!: unknown field modifier `{word}` (the only modifier is `fresh`)"
                );

                assert!(
                    matches!(field.ty, FieldTy::U64),
                    "schema!: fresh field `{}` must be u64 — fresh is the mint \
                     mark, and minted generations are u64",
                    field.name
                );
                assert!(
                    field.newtype.is_some(),
                    "schema!: fresh field `{}` needs `as NewType` — without it \
                     there is no typed alloc path (use the descriptor API for a \
                     raw-u64 fresh field)",
                    field.name
                );
                field.fresh = true;
                tokens.next();
                tokens.next();
            }
        }
    }
    field
}

fn parse_interval_width(name: &str, tokens: &mut Tokens) -> Option<u64> {
    if !peek_punct(tokens, ',') {
        return None;
    }
    tokens.next();
    assert!(
        !peek_punct(tokens, '>'),
        "schema!: field `{name}`: `interval<E, >` names no width — write \
         `interval<E>` (general) or `interval<E, w>` with w >= 1"
    );
    let (negative, text) = parse_int(tokens, "the interval width");
    assert!(
        !negative,
        "schema!: field `{name}`: an interval width is a point count — non-negative"
    );
    let width = u64_text(&text)
        .unwrap_or_else(|| panic!("schema!: field `{name}`: malformed interval width `{text}`"));
    assert!(
        width >= 1,
        "schema!: field `{name}`: interval<E, 0> denotes nothing — the width must be >= 1"
    );
    Some(width)
}

fn peek_brace(tokens: &mut Tokens) -> bool {
    matches!(tokens.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
}

fn peek_bracket(tokens: &mut Tokens) -> bool {
    matches!(tokens.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket)
}

fn parse_closed_relation(tokens: &mut Tokens) -> Relation {
    let name = expect_ident(tokens, "a relation name");
    assert_eq!(
        peek_ident(tokens).as_deref(),
        Some("as"),
        "schema!: closed relation `{name}` needs `as NewType` — the handle needs a host type"
    );
    tokens.next();
    let (newtype, newtype_span) = spanned_ident(tokens, "the handle newtype's name");
    let mut relation = if peek_brace(tokens) {
        let body = take_group(tokens, Delimiter::Brace, "a relation body");
        parse_relation(name, body)
    } else {
        Relation {
            name,
            fields: Vec::new(),
            newtype_span: None,
            closed: None,
        }
    };
    relation.newtype_span = Some(newtype_span);
    for field in &relation.fields {
        assert_ne!(
            field.name, "id",
            "schema!: closed relation `{}` declares a column `id` — the synthetic \
             handle-id field owns that name",
            relation.name
        );
    }
    expect_punct(tokens, '=');
    let body = take_group(tokens, Delimiter::Brace, "the extension block");
    relation.closed = Some(parse_extension(&relation, body));
    expect_punct(tokens, ';');
    relation.fields.insert(
        0,
        Field {
            name: "id".to_owned(),
            ty: FieldTy::U64,
            newtype: Some(newtype),
            fresh: false,
        },
    );
    relation
}

/// `declaration` still holds declared columns only (the synthetic id lands
/// after this returns).
fn parse_extension(declaration: &Relation, body: TokenStream) -> Closed {
    let mut tokens = body.into_iter().peekable();
    let mut rows: Vec<ClosedRow> = Vec::new();
    while tokens.peek().is_some() {
        let handle = expect_ident(&mut tokens, "a handle");
        assert!(
            rows.iter().all(|row| row.handle != handle),
            "schema!: closed relation `{}` declares the handle `{handle}` twice",
            declaration.name
        );
        let mut entries: Vec<(String, Literal)> = Vec::new();
        if peek_brace(&mut tokens) {
            let body = take_group(&mut tokens, Delimiter::Brace, "a row's column block");
            let mut row_tokens = body.into_iter().peekable();
            while row_tokens.peek().is_some() {
                let column = expect_ident(&mut row_tokens, "a column name");
                expect_punct(&mut row_tokens, ':');
                let literal = parse_literal(&mut row_tokens);
                assert!(
                    declaration.fields.iter().any(|f| f.name == column),
                    "schema!: row `{handle}` of closed relation `{}` names an extra \
                     column `{column}`",
                    declaration.name
                );
                assert!(
                    entries.iter().all(|(name, _)| *name != column),
                    "schema!: row `{handle}` of closed relation `{}` supplies the \
                     column `{column}` twice",
                    declaration.name
                );
                entries.push((column, literal));
                if peek_punct(&mut row_tokens, ',') {
                    row_tokens.next();
                }
            }
        }
        let values = declaration
            .fields
            .iter()
            .map(|field| {
                entries
                    .iter()
                    .find(|(name, _)| *name == field.name)
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!(
                            "schema!: row `{handle}` of closed relation `{}` is missing \
                             the column `{}`",
                            declaration.name, field.name
                        )
                    })
            })
            .collect();
        rows.push(ClosedRow { handle, values });
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    assert!(
        !rows.is_empty(),
        "schema!: closed relation `{}` declares an empty extension — rows are the \
         relation's ground axioms, and a vocabulary of nothing is no relation",
        declaration.name
    );
    Closed { rows }
}

fn is_int_text(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_digit()) && !text.contains('.')
}

fn parse_int(tokens: &mut Tokens, what: &str) -> (bool, String) {
    let negative = peek_punct(tokens, '-');
    if negative {
        tokens.next();
    }
    match tokens.next() {
        Some(TokenTree::Literal(lit)) => {
            let text = lit.to_string();
            assert!(
                is_int_text(&text),
                "schema!: expected {what}, found `{text}`"
            );
            (negative, text)
        }
        other => panic!("schema!: expected {what}, found {other:?}"),
    }
}

fn finish_int(tokens: &mut Tokens, negative: bool, text: String) -> Literal {
    if peek_punct(tokens, '.') {
        tokens.next();
        expect_punct(tokens, '.');
        let end = parse_int(tokens, "the interval literal's end bound");
        Literal::Interval {
            start: (negative, text),
            end,
        }
    } else {
        Literal::Int { negative, text }
    }
}

fn parse_literal(tokens: &mut Tokens) -> Literal {
    match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let (word, span) = spanned_ident(tokens, "a literal");
            match word.as_str() {
                "true" => Literal::Bool(true),
                "false" => Literal::Bool(false),
                _ => Literal::Handle(word, span),
            }
        }
        Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
            let (negative, text) = parse_int(tokens, "an integer literal");
            finish_int(tokens, negative, text)
        }
        Some(TokenTree::Literal(_)) => {
            let Some(TokenTree::Literal(lit)) = tokens.next() else {
                unreachable!("peeked a literal");
            };
            let text = lit.to_string();
            if text.starts_with('"') {
                Literal::Str(text)
            } else if text.starts_with("b\"") {
                Literal::Bytes(text)
            } else {
                assert!(is_int_text(&text), "schema!: unsupported literal `{text}`");
                finish_int(tokens, false, text)
            }
        }
        other => panic!("schema!: expected a literal, found {other:?}"),
    }
}

fn parse_literals(tokens: &mut Tokens) -> (Literals, Option<Span>) {
    if !peek_brace(tokens) {
        return (Literals::One(parse_literal(tokens)), None);
    }
    let Some(TokenTree::Group(group)) = tokens.next() else {
        unreachable!("peeked a brace group");
    };
    let span = group.span();
    let mut set_tokens = group.stream().into_iter().peekable();
    let mut literals = Vec::new();
    while set_tokens.peek().is_some() {
        literals.push(parse_literal(&mut set_tokens));
        if peek_punct(&mut set_tokens, ',') {
            set_tokens.next();
        }
    }
    (Literals::Many(literals), Some(span))
}

fn parse_side(relation: String, relation_span: Span, group: TokenStream) -> Side {
    let mut tokens = group.into_iter().peekable();
    let mut projection = Vec::new();
    while tokens.peek().is_some() && !peek_punct(&mut tokens, '|') {
        projection.push(spanned_ident(&mut tokens, "a field name"));
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    let mut selection = Vec::new();
    if peek_punct(&mut tokens, '|') {
        tokens.next();
        while tokens.peek().is_some() {
            let (field, field_span) = spanned_ident(&mut tokens, "a selected field name");
            expect_punct(&mut tokens, '=');
            expect_punct(&mut tokens, '=');
            let (literals, set_span) = parse_literals(&mut tokens);
            selection.push(Binding {
                field,
                field_span,
                literals,
                set_span,
            });
            if peek_punct(&mut tokens, ',') {
                tokens.next();
            }
        }
    }
    Side {
        relation,
        relation_span,
        projection,
        selection,
    }
}

fn parse_statement_side(tokens: &mut Tokens) -> Side {
    let (relation, relation_span) = spanned_ident(tokens, "a relation name");
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    parse_side(relation, relation_span, group)
}

struct ParseError {
    span: Span,
    message: String,
}

#[expect(
    clippy::too_many_lines,
    reason = "one arm per operator spelling — clearer kept together \
              (the `descriptor` precedent)"
)]
fn parse_statement(
    relation: String,
    relation_span: Span,
    tokens: &mut Tokens,
    statements: &mut Vec<Statement>,
) -> Result<(), ParseError> {
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    let left = parse_side(relation, relation_span, group);
    match tokens.next() {
        // (OWNER RULING 2026-07-18: the arrow is canon, never respelled) —
        Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
            expect_punct(tokens, '>');
            let (right, right_span) = spanned_ident(tokens, "the FD's relation name");
            assert!(
                left.selection.is_empty(),
                "schema!: an FD takes no selection — the FD form is `R(X) -> R`"
            );
            if let Some(error) = duplicate_determinant_field(&left) {
                return Err(error);
            }
            if right != left.relation {
                let fields: Vec<&str> = left
                    .projection
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                return Err(ParseError {
                    span: right_span,
                    message: format!(
                        "schema!: the key arrow closes over its own relation: \
                         `{rel}({proj}) -> {rel}` — the projection determines the \
                         tuple, and that closure is what makes a key a key (a \
                         functional dependency over the relation's own attributes); \
                         `-> {right}` is not a key statement",
                        rel = left.relation,
                        proj = fields.join(", "),
                    ),
                });
            }
            statements.push(Statement::Functionality {
                relation: left.relation,
                relation_span: left.relation_span,
                projection: left.projection,
            });
        }

        Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
            expect_punct(tokens, '=');
            let (weight, weight_span) = if peek_bracket(tokens) {
                let Some(TokenTree::Group(group)) = tokens.next() else {
                    unreachable!("peeked a bracket group");
                };
                let weight_span = group.span();
                let weight = parse_weight(group.stream());
                assert!(
                    peek_brace(tokens),
                    "schema!: `<=[w]` names a weight but no window — the capacity \
                     statement is `Parent(key) <=[w]{{lo..hi}} Child(field)`"
                );
                (weight, Some(weight_span))
            } else {
                (WeightSpec::Unit, None)
            };
            if peek_brace(tokens) {
                let Some(TokenTree::Group(group)) = tokens.next() else {
                    unreachable!("peeked a brace group");
                };
                let window_span = group.span();
                let spelling = parse_window(group.stream());
                let right = parse_statement_side(tokens);
                statements.push(Statement::Capacity {
                    source: right,
                    weight,
                    weight_span,
                    window: spelling,
                    window_span,
                    target: left,
                });
            } else {
                let right = parse_statement_side(tokens);
                statements.push(Statement::Containment {
                    source: left,
                    target: right,
                    bidirectional: false,
                });
            }
        }

        Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
            expect_punct(tokens, '=');
            let right = parse_statement_side(tokens);
            statements.push(Statement::Containment {
                source: left,
                target: right,
                bidirectional: true,
            });
        }

        Some(TokenTree::Ident(ident)) if ident.to_string() == "in" => {
            panic!(
                "schema!: the `in lo..hi per` window form is deleted — a window is \
                 B-family, target-left: `Parent(key) <={{lo..hi}} Child(field)`, \
                 with `{{n}}` the exact-count spelling"
            );
        }
        other => {
            panic!("schema!: expected `->`, `<=`, `<=[w]{{lo..hi}}`, or `==`, found {other:?}")
        }
    }
    expect_punct(tokens, ';');
    Ok(())
}

fn duplicate_determinant_field(side: &Side) -> Option<ParseError> {
    for (idx, (field, span)) in side.projection.iter().enumerate() {
        if side.projection[..idx]
            .iter()
            .any(|(prior, _)| prior == field)
        {
            let fields: Vec<&str> = side
                .projection
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            return Some(ParseError {
                span: *span,
                message: format!(
                    "schema!: `{field}` appears twice in the determinant of \
                     `{rel}({proj}) -> {rel}` — a determinant is a field set, \
                     duplicate-free",
                    rel = side.relation,
                    proj = fields.join(", "),
                ),
            });
        }
    }
    None
}

/// A path spelling (`{lo..a.b}` — the LONE dot; the range operator's first dot
/// is Joint) is the pinned-column refusal, the same verdict as the weight's,
/// the TS surface's, and the spec resolver's.
fn parse_bound(tokens: &mut Tokens, what: &str) -> BoundSpec {
    if matches!(tokens.peek(), Some(TokenTree::Ident(_))) {
        let (name, _) = spanned_ident(tokens, what);
        if name == "Duration" && matches!(tokens.peek(), Some(TokenTree::Group(_))) {
            let group = take_group(tokens, Delimiter::Parenthesis, "the Duration bound's field");
            let mut inner = group.into_iter().peekable();
            let field = expect_ident(&mut inner, "the Duration bound's field name");
            assert!(
                inner.peek().is_none(),
                "schema!: trailing tokens in `Duration({field})`"
            );
            return BoundSpec::Duration(field.into());
        }
        assert!(
            !peek_path_dot(tokens),
            "schema!: the bound path `{{..{name}.…}}` is refused — a dependent \
             bound names a field of the TARGET's own row, closed at the row \
             exactly like the weight (ruled 2026-07-24, ruling 6); state the \
             join as a law and read the local column (the pinned-column idiom): \
             `Pool(id, supply) <= Grid(pool, supply); \
             Pool(id) <=[watts]{{0..supply}} Device(pool);`"
        );
        return BoundSpec::Field(name.into());
    }
    let (negative, text) = parse_int(tokens, what);
    assert!(
        !negative,
        "schema!: a window bound is a measure — non-negative"
    );
    BoundSpec::Lit(
        u64_text(&text).unwrap_or_else(|| panic!("schema!: malformed window bound `{text}`")),
    )
}

fn parse_weight(body: TokenStream) -> WeightSpec {
    let mut tokens = body.into_iter().peekable();
    assert!(
        tokens.peek().is_some(),
        "schema!: the weight bracket `[]` names no measure — write `[field]` or \
         `[Duration(field)]` (an absent bracket is the unit weight: the count)"
    );
    let (name, _) = spanned_ident(&mut tokens, "the weight field");
    let weight = if name == "Duration" && matches!(tokens.peek(), Some(TokenTree::Group(_))) {
        let group = take_group(
            &mut tokens,
            Delimiter::Parenthesis,
            "the Duration weight's field",
        );
        let mut inner = group.into_iter().peekable();
        let field = expect_ident(&mut inner, "the Duration weight's field name");
        assert!(
            inner.peek().is_none(),
            "schema!: trailing tokens in `Duration({field})`"
        );
        WeightSpec::Duration(field.into())
    } else {
        WeightSpec::Field(name.as_str().into())
    };
    if peek_punct(&mut tokens, '.') {
        let field = match &weight {
            WeightSpec::Field(name) | WeightSpec::Duration(name) => name.clone(),
            WeightSpec::Unit => unreachable!("a parsed weight names a field"),
        };
        panic!(
            "schema!: the weight path `[{field}.…]` is refused — the weight vocabulary \
             is closed at the row (ruled 2026-07-24, ruling 6); state the join as a \
             law and read the local column (the pinned-column idiom): \
             `Device(model, watts) <= Model(id, watts); \
             Pool(id) <=[watts]{{0..supply}} Device(pool);`"
        );
    }
    assert!(
        tokens.peek().is_none(),
        "schema!: trailing tokens after the weight"
    );
    weight
}

fn parse_window(body: TokenStream) -> CapacityWindowSpec {
    let mut tokens = body.into_iter().peekable();
    assert!(
        tokens.peek().is_some(),
        "schema!: the window `{{}}` names no bounds — write `{{n}}`, `{{lo..hi}}`, or `{{lo..*}}`"
    );
    assert!(
        !peek_punct(&mut tokens, '.'),
        "schema!: `{{..hi}}` never parses — bounds are always explicit: a ceiling is \
         written `{{0..hi}}`"
    );
    let lo = parse_bound(&mut tokens, "the window's lower bound");
    if tokens.peek().is_none() {
        return CapacityWindowSpec::Exact(lo);
    }
    expect_punct(&mut tokens, '.');
    expect_punct(&mut tokens, '.');
    assert!(
        tokens.peek().is_some(),
        "schema!: `{{lo..}}` never parses — bounds are always explicit: a floor is \
         written `{{lo..*}}`"
    );
    let spelling = if peek_punct(&mut tokens, '*') {
        tokens.next();
        CapacityWindowSpec::Floor(lo)
    } else {
        CapacityWindowSpec::Range {
            lo,
            hi: parse_bound(&mut tokens, "the window's upper bound"),
        }
    };
    assert!(
        tokens.peek().is_none(),
        "schema!: trailing tokens after the window bounds"
    );
    spelling
}

fn parse_schema(input: TokenStream) -> Result<SchemaAst, ParseError> {
    let mut tokens = input.into_iter().peekable();
    match tokens.next() {
        Some(TokenTree::Ident(ident)) if ident.to_string() == "pub" => {}
        other => panic!(
            "schema!: the first item names the schema — `pub Name;` — found {other:?}"
        ),
    }
    let name = expect_ident(&mut tokens, "the schema name");
    expect_punct(&mut tokens, ';');
    let mut schema = SchemaAst {
        name,
        relations: Vec::new(),
        statements: Vec::new(),
    };
    while tokens.peek().is_some() {
        let (ident, ident_span) =
            spanned_ident(&mut tokens, "`relation`, `closed relation`, or a statement");
        if ident == "closed" {
            let keyword = expect_ident(&mut tokens, "`relation` after `closed`");
            assert_eq!(
                keyword, "relation",
                "schema!: expected `relation` after `closed`, found `{keyword}`"
            );
            schema.relations.push(parse_closed_relation(&mut tokens));
        } else if ident == "relation" {
            let name = expect_ident(&mut tokens, "a relation name");
            let body = take_group(&mut tokens, Delimiter::Brace, "a relation body");
            schema.relations.push(parse_relation(name, body));
        } else if ident == "order" {
            panic!(
                "schema!: `order` statements no longer exist — order is a derivation, \
                 not a dependency: use fractional indexing over a keyed position, or \
                 the exact-partition interval recipe"
            );
        } else {
            parse_statement(ident, ident_span, &mut tokens, &mut schema.statements)?;
        }
    }
    Ok(schema)
}

/// The declarative schema surface: expands to the header's `Theory`
/// unit struct, host-side newtypes and host enums, one typed fact struct
/// boundaries, and one generated key struct per declared key statement
/// on an ordinary relation (`{R}By{Fields}`, implementing `Key`). The
/// expansion builds a `SchemaSpec` plus a span table, runs
/// the ONE shared lowering (`SchemaSpec::descriptor` — name→id resolution
/// and the canonical-utterance ban table, the same pass the runtime spec
/// path runs), and emits the lowered `SchemaDescriptor` as const
/// # Panics
/// On malformed `schema!` grammar or a literal that does not fit its
/// field's declared type — a compile error at the macro call site.
/// Lowering issues (unresolvable names, banned spellings) are not panics:
/// each becomes a `compile_error!` at the offending token — as do the
/// parse's teaching errors (a key arrow whose right side names a foreign
/// relation, a determinant field spelled twice), each spanned at the
/// offending token.
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let schema = match parse_schema(input) {
        Ok(schema) => schema,
        Err(error) => return compile_error_tokens(error.span, &error.message),
    };
    let (spec, spans) = lower_input(&schema);
    let descriptor = match spec.descriptor() {
        Ok(descriptor) => descriptor,
        Err(error) => return spec_errors(error.issues(), &spec, &spans),
    };
    let mut out = String::new();
    emit_schema_def(&mut out, &schema.name, &descriptor);
    emit_id_constants(&mut out, &schema);
    emit_newtypes(&mut out, &schema.relations);
    emit_closed(&mut out, &schema.relations, &descriptor);

    let mut fresh_ordinal = 0usize;
    for (index, relation) in schema.relations.iter().enumerate() {
        let fresh_count = relation.fields.iter().filter(|field| field.fresh).count();

        if relation.closed.is_some() {
            fresh_ordinal += fresh_count;
            continue;
        }
        emit_fact_struct(&mut out, &schema.name, index, relation, fresh_ordinal);
        fresh_ordinal += fresh_count;
    }
    emit_key_structs(&mut out, &schema);
    out.parse().expect("schema!: generated code parses")
}

#[derive(Default)]
struct SpanTable {
    relations: BTreeMap<(usize, String), Vec<Span>>,

    fields: BTreeMap<(usize, String, String), Vec<Span>>,

    capacities: BTreeMap<usize, Span>,

    weights: BTreeMap<usize, Span>,

    sets: BTreeMap<(usize, String, usize), Vec<Span>>,

    literals: BTreeMap<LiteralAt, Span>,

    newtypes: BTreeMap<usize, Span>,
}

fn declared_type<'ast>(
    schema: &'ast SchemaAst,
    relation: &str,
    field: &str,
) -> Option<&'ast FieldTy> {
    schema
        .relations
        .iter()
        .find(|r| r.name == relation)?
        .fields
        .iter()
        .find(|f| f.name == field)
        .map(|f| &f.ty)
}

fn typed_or_placeholder(
    schema: &SchemaAst,
    relation: &str,
    field: &str,
    literal: &Literal,
) -> LiteralSpec {
    if let Literal::Handle(name, _) = literal {
        return LiteralSpec::Handle(name.as_str().into());
    }
    match declared_type(schema, relation, field) {
        Some(ty) => typed_literal(relation, field, ty, literal),
        None => LiteralSpec::Value(Value::U64(0)),
    }
}

fn typed_literal(relation: &str, field: &str, ty: &FieldTy, literal: &Literal) -> LiteralSpec {
    let value = match (ty, literal) {
        (_, Literal::Handle(name, _)) => return LiteralSpec::Handle(name.as_str().into()),
        (FieldTy::Bool, Literal::Bool(v)) => Value::Bool(*v),
        (
            FieldTy::U64,
            Literal::Int {
                negative: false,
                text,
            },
        ) => Value::U64(u64_text(text).unwrap_or_else(|| literal_mismatch(relation, field))),
        (FieldTy::I64, Literal::Int { negative, text }) => Value::I64(
            i64_text(*negative, text).unwrap_or_else(|| literal_mismatch(relation, field)),
        ),
        (FieldTy::Str, Literal::Str(text)) => Value::String(unescape_str(text)),

        (FieldTy::FixedBytes(len), Literal::Bytes(text)) => {
            let bytes = unescape_bytes(text);
            if u64::try_from(bytes.len()) != Ok(*len) {
                literal_mismatch(relation, field);
            }
            Value::FixedBytes(bytes.into())
        }
        (
            FieldTy::Interval(IntervalElement::U64),
            Literal::Interval {
                start: (false, start),
                end: (false, end),
            },
        ) => {
            let start = u64_text(start).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = u64_text(end).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<u64>::new(start, end));
            Value::IntervalU64(interval)
        }
        (
            FieldTy::FixedInterval(IntervalElement::U64, w),
            Literal::Interval {
                start: (false, start),
                end: (false, end),
            },
        ) => {
            let start = u64_text(start).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = u64_text(end).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<u64>::new(start, end));

            if interval.end() - interval.start() != *w || interval.is_ray() {
                literal_mismatch(relation, field);
            }
            Value::IntervalU64(interval)
        }
        (FieldTy::Interval(IntervalElement::I64), Literal::Interval { start, end }) => {
            let start =
                i64_text(start.0, &start.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = i64_text(end.0, &end.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<i64>::new(start, end));
            Value::IntervalI64(interval)
        }
        (FieldTy::FixedInterval(IntervalElement::I64, w), Literal::Interval { start, end }) => {
            let start =
                i64_text(start.0, &start.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = i64_text(end.0, &end.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<i64>::new(start, end));
            if interval.end().abs_diff(interval.start()) != *w || interval.is_ray() {
                literal_mismatch(relation, field);
            }
            Value::IntervalI64(interval)
        }
        _ => literal_mismatch(relation, field),
    };
    LiteralSpec::Value(value)
}

/// The typing seam's one refusal, shared by every arm.
fn literal_mismatch(relation: &str, field: &str) -> ! {
    panic!(
        "schema!: the literal for `{relation}.{field}` does not fit \
         the field's declared type"
    )
}

fn nonempty_interval<T>(relation: &str, field: &str, interval: Option<T>) -> T {
    interval.unwrap_or_else(|| {
        panic!(
            "schema!: the interval literal for `{relation}.{field}` is empty — \
             `start..end` is half-open, start < end"
        )
    })
}

fn int_magnitude(text: &str) -> Option<u128> {
    let text = text.replace('_', "");
    let (digits, radix) = match text.as_bytes() {
        [b'0', b'x', ..] => (&text[2..], 16),
        [b'0', b'o', ..] => (&text[2..], 8),
        [b'0', b'b', ..] => (&text[2..], 2),
        _ => (text.as_str(), 10),
    };
    u128::from_str_radix(digits, radix).ok()
}

fn u64_text(text: &str) -> Option<u64> {
    u64::try_from(int_magnitude(text)?).ok()
}

fn i64_text(negative: bool, text: &str) -> Option<i64> {
    let magnitude = i128::try_from(int_magnitude(text)?).ok()?;
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

fn unescape_str(text: &str) -> Box<str> {
    let body = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("rustc lexed the string literal");
    String::from_utf8(unescape(body, true))
        .expect("schema! string literals are UTF-8")
        .into_boxed_str()
}

fn unescape_bytes(text: &str) -> Vec<u8> {
    let body = text
        .strip_prefix("b\"")
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("rustc lexed the byte-string literal");
    unescape(body, false)
}

/// The cooked-literal escape decoder — the seam's token→bytes half.
/// `unicode` admits `\u{…}` (string literals only). Malformed escapes
/// are unreachable: the token came out of rustc's lexer.
fn unescape(body: &str, unicode: bool) -> Vec<u8> {
    let lexed = "rustc lexed the literal";
    let mut out = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut utf8 = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            continue;
        }
        match chars.next().expect(lexed) {
            'n' => out.push(b'\n'),
            'r' => out.push(b'\r'),
            't' => out.push(b'\t'),
            '\\' => out.push(b'\\'),
            '\'' => out.push(b'\''),
            '"' => out.push(b'"'),
            '0' => out.push(0),
            'x' => {
                let high = chars.next().and_then(|c| c.to_digit(16)).expect(lexed);
                let low = chars.next().and_then(|c| c.to_digit(16)).expect(lexed);
                out.push(u8::try_from(high * 16 + low).expect("two hex digits fit a byte"));
            }
            'u' if unicode => {
                assert_eq!(chars.next(), Some('{'), "{lexed}");
                let mut code = 0u32;
                loop {
                    let c = chars.next().expect(lexed);
                    if c == '}' {
                        break;
                    }
                    code = code * 16 + c.to_digit(16).expect(lexed);
                }
                let mut utf8 = [0u8; 4];
                let c = char::from_u32(code).expect(lexed);
                out.extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            }
            // The line-continuation escape: a backslash before a newline
            // swallows the newline and leading whitespace.
            '\n' => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            other => unreachable!("rustc lexed the literal; found escape `\\{other}`"),
        }
    }
    out
}

/// One field's declared structural type as the shared [`ValueType`].
fn field_value_type(relation: &str, field: &Field) -> ValueType {
    match &field.ty {
        FieldTy::Bool => ValueType::Bool,
        FieldTy::U64 => ValueType::U64,
        FieldTy::I64 => ValueType::I64,
        FieldTy::Str => ValueType::String,
        FieldTy::FixedBytes(len) => ValueType::FixedBytes {
            len: u16::try_from(*len).unwrap_or_else(|_| {
                // 65..=u16::MAX still flows to the validator's typed
                // range error; only the unrepresentable dies here.
                panic!(
                    "schema!: field `{relation}.{}`: bytes<{len}> does not fit the \
                     width's domain (1..=64)",
                    field.name
                )
            }),
        },
        FieldTy::Interval(element) => ValueType::Interval { element: *element },
        FieldTy::FixedInterval(element, width) => ValueType::FixedInterval {
            element: *element,
            width: *width,
        },
    }
}

/// The parse as the shared lowering's input: the [`SchemaSpec`] twin of
/// the invocation (declared columns only — the AST's synthetic closed
/// `id` field is the validator's to materialize) plus the span table
/// every lowering issue maps through.
fn lower_input(schema: &SchemaAst) -> (SchemaSpec, SpanTable) {
    let mut spans = SpanTable::default();
    let relations = lower_relations(schema, &mut spans);
    let statements = lower_statements(schema, &mut spans);
    (
        SchemaSpec {
            relations,
            statements,
        },
        spans,
    )
}

/// [`lower_input`]'s relation half: `RelationSpec`s in declaration
/// order, closed extensions through the token→`Value` seam, handle and
/// newtype spans recorded.
fn lower_relations(schema: &SchemaAst, spans: &mut SpanTable) -> Vec<RelationSpec> {
    let mut relations = Vec::with_capacity(schema.relations.len());
    for (rel_idx, relation) in schema.relations.iter().enumerate() {
        if let Some(span) = relation.newtype_span {
            spans.newtypes.insert(rel_idx, span);
        }
        let declared = &relation.fields[usize::from(relation.closed.is_some())..];
        // The fused closed half (ruled 2026-07-23, R7): the handle
        // newtype and the ground axioms travel together — the AST's
        // synthetic id field at index 0 carries the newtype.
        let closed = relation.closed.as_ref().map(|extension| ClosedSpec {
            newtype: relation.fields[0]
                .newtype
                .as_deref()
                .expect("closed relations carry the handle newtype")
                .into(),
            rows: extension
                .rows
                .iter()
                .enumerate()
                .map(|(row_idx, row)| RowSpec {
                    handle: row.handle.as_str().into(),
                    values: row
                        .values
                        .iter()
                        .enumerate()
                        .map(|(column, (column_name, literal))| {
                            if let Literal::Handle(_, span) = literal {
                                let at = LiteralAt::Row {
                                    relation: rel_idx,
                                    row: row_idx,
                                    column,
                                };
                                spans.literals.insert(at, *span);
                            }
                            typed_literal(
                                &relation.name,
                                column_name,
                                &declared[column].ty,
                                literal,
                            )
                        })
                        .collect(),
                })
                .collect(),
        });
        relations.push(RelationSpec {
            name: relation.name.as_str().into(),
            fields: declared
                .iter()
                .map(|field| FieldSpec {
                    name: field.name.as_str().into(),
                    value_type: field_value_type(&relation.name, field),
                    newtype: field.newtype.as_deref().map(Into::into),
                    fresh: field.fresh,
                })
                .collect(),
            closed,
        });
    }
    relations
}

/// [`lower_input`]'s statement half, over the relations already lowered.
fn lower_statements(schema: &SchemaAst, spans: &mut SpanTable) -> Vec<StatementSpec> {
    let mut statements = Vec::with_capacity(schema.statements.len());
    for (index, statement) in schema.statements.iter().enumerate() {
        match statement {
            Statement::Functionality {
                relation,
                relation_span,
                projection,
            } => {
                spans
                    .relations
                    .entry((index, relation.clone()))
                    .or_default()
                    .push(*relation_span);
                for (field, span) in projection {
                    spans
                        .fields
                        .entry((index, relation.clone(), field.clone()))
                        .or_default()
                        .push(*span);
                }
                statements.push(StatementSpec::Fd {
                    relation: relation.as_str().into(),
                    projection: projection
                        .iter()
                        .map(|(field, _)| field.as_str().into())
                        .collect(),
                });
            }
            Statement::Containment {
                source,
                target,
                bidirectional,
            } => {
                statements.push(StatementSpec::Containment {
                    source: lower_side(schema, index, StatementSide::Source, source, spans),
                    target: lower_side(schema, index, StatementSide::Target, target, spans),
                    bidirectional: *bidirectional,
                });
            }
            Statement::Capacity {
                source,
                weight,
                weight_span,
                window,
                window_span,
                target,
            } => {
                spans.capacities.insert(index, *window_span);
                if let Some(span) = weight_span {
                    spans.weights.insert(index, *span);
                }
                // Weight and dependent-bound TYPING, judged at expansion
                // (the `fresh`-typing precedent: the macro holds the
                // declared types, so the mistake dies at the invocation,
                // never deferred to `Db::create`). Name RESOLUTION stays
                // the shared lowering's — an unknown weight or bound
                // ident lands as its `UnknownField` compile error
                // through the span table.
                check_weight_typing(schema, &source.relation, weight);
                check_bound_typing(schema, &target.relation, weight, window);
                // Weight and bound idents are field occurrences: record
                // their spans under the fields multimap so the lowering's
                // `UnknownField` marks the offending token itself.
                if let (Some(span), WeightSpec::Field(name) | WeightSpec::Duration(name)) =
                    (weight_span, weight)
                {
                    spans
                        .fields
                        .entry((index, source.relation.clone(), name.to_string()))
                        .or_default()
                        .push(*span);
                }
                for bound in std::iter::once(window_bound_lo(window)).chain(window_bound_hi(window))
                {
                    if let BoundSpec::Field(name) | BoundSpec::Duration(name) = bound {
                        spans
                            .fields
                            .entry((index, target.relation.clone(), name.to_string()))
                            .or_default()
                            .push(*window_span);
                    }
                }
                statements.push(StatementSpec::Capacity {
                    target: lower_side(schema, index, StatementSide::Target, target, spans),
                    weight: weight.clone(),
                    window: window.clone(),
                    source: lower_side(schema, index, StatementSide::Source, source, spans),
                });
            }
        }
    }
    statements
}

/// A capacity window's floor-slot bound (`Exact` and `Floor` occupy
/// the floor slot; `Range` reads its `lo`).
fn window_bound_lo(window: &CapacityWindowSpec) -> &BoundSpec {
    match window {
        CapacityWindowSpec::Exact(bound) | CapacityWindowSpec::Floor(bound) => bound,
        CapacityWindowSpec::Range { lo, .. } => lo,
    }
}

/// A capacity window's ceiling-slot bound, if one is spelled.
fn window_bound_hi(window: &CapacityWindowSpec) -> Option<&BoundSpec> {
    match window {
        CapacityWindowSpec::Range { hi, .. } => Some(hi),
        CapacityWindowSpec::Exact(_) | CapacityWindowSpec::Floor(_) => None,
    }
}

/// Weight TYPING at expansion — the `fresh`-typing precedent: `[field]`
/// measures a u64-encoded SOURCE position (a signed encoding is the
/// polarity refusal: a negative weight would let an insert lower a
/// sum), `[Duration(field)]` an interval one. Unknown names fall
/// through silently — the shared lowering reports them as
/// `UnknownField` at the recorded span. The engine's
/// `validate_capacity` judges the same rules for runtime descriptors;
/// the messages mirror its `Display` arms.
fn check_weight_typing(schema: &SchemaAst, source_relation: &str, weight: &WeightSpec) {
    match weight {
        WeightSpec::Unit => {}
        WeightSpec::Field(name) => match declared_type(schema, source_relation, name) {
            None | Some(FieldTy::U64) => {}
            Some(FieldTy::I64) => panic!(
                "schema!: weight field `{name}` on `{source_relation}` is signed — a \
                     `[field]` weight measures a u64 position, and a signed encoding is \
                     refused by polarity: a negative weight would let an insert lower a \
                     sum"
            ),
            Some(_) => panic!(
                "schema!: weight field `{name}` on `{source_relation}` is not \
                     u64-encoded — a `[field]` weight measures a u64 SOURCE position"
            ),
        },
        WeightSpec::Duration(name) => match declared_type(schema, source_relation, name) {
            None | Some(FieldTy::Interval(_) | FieldTy::FixedInterval(..)) => {}
            Some(_) => panic!(
                "schema!: weight field `{name}` on `{source_relation}` is not \
                     interval-typed — `[Duration(field)]` reads an interval position's \
                     measure"
            ),
        },
    }
}

/// Dependent-bound TYPING at expansion, plus the C18 dimension gate: a
/// bound ident is a u64 or interval position of TARGET's row (by name
/// against the whole roster — C1), and a unit (count) window against a
/// `Duration(field)` bound mixes dimensions (ruled 2026-07-24, C18).
/// Unknown names fall through to the lowering's `UnknownField`.
fn check_bound_typing(
    schema: &SchemaAst,
    target_relation: &str,
    weight: &WeightSpec,
    window: &CapacityWindowSpec,
) {
    for bound in std::iter::once(window_bound_lo(window)).chain(window_bound_hi(window)) {
        match bound {
            BoundSpec::Lit(_) => {}
            BoundSpec::Field(name) => match declared_type(schema, target_relation, name) {
                None | Some(FieldTy::U64) => {}
                Some(FieldTy::I64) => panic!(
                    "schema!: bound field `{name}` on `{target_relation}` is signed — a \
                     dependent bound reads a u64 field of the TARGET's row (a signed \
                     encoding cannot bound a non-negative measure)"
                ),
                Some(_) => panic!(
                    "schema!: bound field `{name}` on `{target_relation}` is not \
                     u64-encoded — a dependent bound reads a u64 field of the TARGET's \
                     row"
                ),
            },
            BoundSpec::Duration(name) => {
                if !matches!(
                    declared_type(schema, target_relation, name),
                    None | Some(FieldTy::Interval(_) | FieldTy::FixedInterval(..))
                ) {
                    panic!(
                        "schema!: bound field `{name}` on `{target_relation}` is not \
                         interval-typed — `{{..Duration(field)}}` bounds by a TARGET \
                         interval's measure"
                    );
                }
                if matches!(weight, WeightSpec::Unit) {
                    panic!(
                        "schema!: a unit (count) window against the Duration bound \
                         `Duration({name})` — a count of facts bounded by a span of \
                         time is a dimension error (ruled 2026-07-24, C18): weigh the \
                         source with `[Duration(field)]`, or bound by a u64 field or \
                         literal"
                    );
                }
            }
        }
    }
}

/// One parsed side into its [`SideSpec`], every name's span recorded
/// under the keys the lowering's issues carry.
fn lower_side(
    schema: &SchemaAst,
    statement: usize,
    which: StatementSide,
    side: &Side,
    spans: &mut SpanTable,
) -> SideSpec {
    spans
        .relations
        .entry((statement, side.relation.clone()))
        .or_default()
        .push(side.relation_span);
    let mut field_span = |field: &str, span: Span| {
        spans
            .fields
            .entry((statement, side.relation.clone(), field.to_owned()))
            .or_default()
            .push(span);
    };
    let mut projection = Vec::with_capacity(side.projection.len());
    for (field, span) in &side.projection {
        field_span(field, *span);
        projection.push(field.as_str().into());
    }
    let mut selection = Vec::with_capacity(side.selection.len());
    for (binding_idx, binding) in side.selection.iter().enumerate() {
        field_span(&binding.field, binding.field_span);
        let mut handle_span = |literal_idx: usize, literal: &Literal| {
            if let Literal::Handle(_, span) = literal {
                let at = LiteralAt::Selection {
                    statement,
                    side: which,
                    binding: binding_idx,
                    literal: literal_idx,
                };
                spans.literals.insert(at, *span);
            }
        };
        let typed = |literal: &Literal| {
            typed_or_placeholder(schema, &side.relation, &binding.field, literal)
        };
        let literals = match &binding.literals {
            Literals::One(literal) => {
                handle_span(0, literal);
                LiteralSetSpec::One(typed(literal))
            }
            Literals::Many(many) => {
                if let Some(span) = binding.set_span {
                    spans
                        .sets
                        .entry((statement, binding.field.clone(), many.len()))
                        .or_default()
                        .push(span);
                }
                LiteralSetSpec::Many(
                    many.iter()
                        .enumerate()
                        .map(|(literal_idx, literal)| {
                            handle_span(literal_idx, literal);
                            typed(literal)
                        })
                        .collect(),
                )
            }
        };
        selection.push((binding.field.as_str().into(), literals));
    }
    SideSpec {
        relation: side.relation.as_str().into(),
        projection,
        selection,
    }
}

/// Every lowering issue as a `compile_error!` at its offending token —
/// each message naming the canonical form, text unchanged from the
/// macro's panic era. Identical issues collapse (one issue per
/// occurrence is the lowering's contract; the span table's multimap
/// already marks every occurrence under a key).
fn spec_errors(issues: &[SpecIssue], spec: &SchemaSpec, spans: &SpanTable) -> TokenStream {
    let mut out = TokenStream::new();
    let mut seen: Vec<&SpecIssue> = Vec::new();
    for issue in issues {
        if seen.contains(&issue) {
            continue;
        }
        seen.push(issue);
        let message = issue_message(issue, spec);
        for span in issue_spans(issue, spans) {
            out.extend(compile_error_tokens(span, &message));
        }
    }
    out
}

/// The spans one issue marks — through the issue's own structural key
/// where it carries one, through the name-keyed multimaps otherwise.
/// The call site is the (unreachable) fallback: every issue the lowering
/// can raise names tokens the parse recorded.
fn issue_spans(issue: &SpecIssue, spans: &SpanTable) -> Vec<Span> {
    let multi =
        |found: Option<&Vec<Span>>| found.map_or_else(|| vec![Span::call_site()], Clone::clone);
    let one = |found: Option<&Span>| vec![found.copied().unwrap_or_else(Span::call_site)];
    match issue {
        SpecIssue::UnknownRelation {
            statement,
            relation,
        } => multi(spans.relations.get(&(*statement, relation.to_string()))),
        SpecIssue::UnknownField {
            statement,
            relation,
            field,
        } => multi(
            spans
                .fields
                .get(&(*statement, relation.to_string(), field.to_string())),
        ),
        SpecIssue::NotAHandleField { at, .. } | SpecIssue::UnknownHandle { at, .. } => {
            one(spans.literals.get(at))
        }
        // Two invocation-marked issues: `parse_extension` enforces exact
        // column coverage, so an over-wide row never reaches lowering
        // from the macro (the `SchemaSpec` bindings surface is that
        // issue's only producer); and the parse records no span for a
        // relation's own declared name (only statement occurrences), so
        // the sealed-roster cap has no token to mark.
        SpecIssue::RowArityExcess { .. } | SpecIssue::RelationTooManyFields { .. } => {
            vec![Span::call_site()]
        }
        SpecIssue::DuplicateHandleNewtype {
            second_relation, ..
        } => one(spans.newtypes.get(second_relation)),
        SpecIssue::CapacityInverted { statement, .. }
        | SpecIssue::CapacityExactRespelled { statement, .. }
        | SpecIssue::CapacityExclusionRespelled { statement }
        | SpecIssue::CapacityVacuous { statement }
        | SpecIssue::CapacityContainmentRespelled { statement }
        | SpecIssue::CapacityDependentFloor { statement }
        | SpecIssue::CapacityUnitFloor { statement }
        | SpecIssue::BoundPathRefused { statement, .. } => one(spans.capacities.get(statement)),
        SpecIssue::WeightPathRefused { statement, .. } => one(spans.weights.get(statement)),
        SpecIssue::DegenerateLiteralSet {
            statement,
            field,
            len,
        } => multi(spans.sets.get(&(*statement, field.to_string(), *len))),
        // The coherence check cites both faces — each disagrees with the
        // other, so both field idents are offending and both are marked.
        SpecIssue::StatementNewtypeMismatch {
            statement,
            source,
            target,
            ..
        } => {
            let face = |relation: &str, field: &str| {
                spans
                    .fields
                    .get(&(*statement, relation.to_owned(), field.to_owned()))
                    .cloned()
                    .unwrap_or_default()
            };
            let mut marked = face(&source.relation, &source.field);
            marked.extend(face(&target.relation, &target.field));
            if marked.is_empty() {
                vec![Span::call_site()]
            } else {
                marked
            }
        }
    }
}

/// One issue's message — the macro's own dialect: the panic-era text,
/// verbatim, each naming the canonical form (the ban table's law). The
/// containment-respelled window composes the paste-back containment from
/// the spec's own statement.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per issue, each a teaching message — \
              clearer kept together (the `descriptor` precedent)"
)]
fn issue_message(issue: &SpecIssue, spec: &SchemaSpec) -> String {
    match issue {
        SpecIssue::UnknownRelation { relation, .. } => {
            format!("schema!: relation `{relation}` is not declared in this invocation")
        }
        SpecIssue::UnknownField {
            relation, field, ..
        } => format!("schema!: relation `{relation}` has no field `{field}`"),
        SpecIssue::NotAHandleField {
            relation,
            field,
            handle,
            ..
        } => format!(
            "schema!: `{relation}.{field}` is not a closed-relation reference — \
             the handle literal `{handle}` is legal only on a field whose newtype \
             is a closed relation's handle newtype"
        ),
        SpecIssue::UnknownHandle { closed, handle, .. } => {
            format!("schema!: closed relation `{closed}` has no handle `{handle}`")
        }
        SpecIssue::RowArityExcess {
            row,
            name,
            declared,
            supplied,
            ..
        } => format!(
            "schema!: closed relation `{name}`, row {row}: {supplied} values for \
             {declared} declared columns"
        ),
        SpecIssue::RelationTooManyFields { name, fields, .. } => format!(
            "schema!: relation `{name}` seals {fields} fields — the u16 field-id \
             space caps a relation at 65,535 sealed fields (a closed relation's \
             synthetic `id` included)"
        ),
        SpecIssue::DuplicateHandleNewtype {
            newtype,
            first,
            second,
            ..
        } => format!(
            "schema!: handle newtype `{newtype}` is declared by two closed relations \
             (`{first}` and `{second}`) — a handle newtype names exactly one closed relation"
        ),
        SpecIssue::CapacityInverted { lo, hi, .. } => format!(
            "schema!: the window `{{{lo}..{hi}}}` is inverted — no measure satisfies it; \
             bounds are `{{lo..hi}}` with lo < hi (an exact measure is `{{n}}`)"
        ),
        SpecIssue::CapacityExactRespelled { count, .. } => {
            format!("schema!: `{{{count}..{count}}}` — an exact measure is written `{{{count}}}`")
        }
        SpecIssue::CapacityExclusionRespelled { .. } => {
            "schema!: `{0..0}` — the exclusion is written `{0}`".to_owned()
        }
        SpecIssue::CapacityVacuous { .. } => "schema!: the `{0..*}` window is vacuous — it \
             provably says nothing (`lean/Bumbledb/Capacity.lean: capacity_zero_star`); \
             delete the statement"
            .to_owned(),
        SpecIssue::CapacityContainmentRespelled { statement } => {
            let StatementSpec::Capacity { target, source, .. } = &spec.statements[*statement]
            else {
                unreachable!("the containment-respelled window rides a capacity statement");
            };
            format!(
                "schema!: `{{1..*}}` says only what the bare containment says — drop the \
                 annotation and write `{}(…) <= {}(…)`",
                target.relation, source.relation
            )
        }
        SpecIssue::CapacityDependentFloor { .. } => {
            "schema!: a dependent bound in the floor slot — dependent bounds are hi-slot \
             only (ruled 2026-07-24, C6): a dependent floor has no use case; write a \
             literal floor"
                .to_owned()
        }
        SpecIssue::CapacityUnitFloor { .. } => {
            "schema!: `{N..*}` on the unit instance — a bare count floor is refused; \
             weigh the source (`<=[w]{N..*}` stays legal) or drop the bound"
                .to_owned()
        }
        SpecIssue::WeightPathRefused { path, .. } => format!(
            "schema!: the weight path `[{path}]` is refused — the weight vocabulary is \
             closed at the row (ruled 2026-07-24, ruling 6); state the join as a law and \
             read the local column (the pinned-column idiom): \
             `Device(model, watts) <= Model(id, watts); \
             Pool(id) <=[watts]{{0..supply}} Device(pool);`"
        ),
        SpecIssue::BoundPathRefused { path, .. } => format!(
            "schema!: the bound path `{{..{path}}}` is refused — a dependent bound names \
             a field of the TARGET's own row, closed at the row exactly like the weight \
             (ruled 2026-07-24, ruling 6); state the join as a law and read the local \
             column (the pinned-column idiom): \
             `Pool(id, supply) <= Grid(pool, supply); \
             Pool(id) <=[watts]{{0..supply}} Device(pool);`"
        ),
        SpecIssue::DegenerateLiteralSet { field, len: 0, .. } => format!(
            "schema!: the literal set for `{field}` is empty — an empty set selects \
             nothing; write no binding"
        ),
        SpecIssue::DegenerateLiteralSet { field, .. } => format!(
            "schema!: the literal set for `{field}` has one element — a one-element \
             set is the bare literal: write `{field} == L`, no braces"
        ),
        SpecIssue::StatementNewtypeMismatch {
            statement,
            source,
            target,
            ..
        } => {
            let form = match &spec.statements[*statement] {
                StatementSpec::Containment {
                    bidirectional: false,
                    ..
                } => "containment",
                StatementSpec::Containment {
                    bidirectional: true,
                    ..
                } => "set equality",
                StatementSpec::Capacity { .. } => "capacity",
                StatementSpec::Fd { .. } => {
                    unreachable!(
                        "an FD has no paired faces — the arrow closes over its own relation"
                    )
                }
            };
            format!(
                "schema!: the {form} pairs {} with {} — the faces of a dependency \
                 agree on their newtype, or neither carries one",
                source.cite(),
                target.cite()
            )
        }
    }
}

/// `::core::compile_error!{"…"}` with every token spanned at the
/// offender — the diagnostic lands on the token itself, not the
/// invocation.
fn compile_error_tokens(span: Span, message: &str) -> TokenStream {
    let mut literal = proc_macro::Literal::string(message);
    literal.set_span(span);
    let mut group = Group::new(
        Delimiter::Brace,
        TokenStream::from(TokenTree::Literal(literal)),
    );
    group.set_span(span);
    [
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Alone)),
        TokenTree::Ident(Ident::new("core", span)),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Alone)),
        TokenTree::Ident(Ident::new("compile_error", span)),
        TokenTree::Punct(Punct::new('!', Spacing::Alone)),
        TokenTree::Group(group),
    ]
    .into_iter()
    .map(|mut tree| {
        tree.set_span(span);
        tree
    })
    .collect()
}

/// Renders one structural type as its `ValueType` expression.
fn value_type_tokens(value_type: &ValueType) -> String {
    let path = "::bumbledb::schema::ValueType";
    match value_type {
        ValueType::Bool => format!("{path}::Bool"),
        ValueType::U64 => format!("{path}::U64"),
        ValueType::I64 => format!("{path}::I64"),
        ValueType::String => format!("{path}::String"),
        ValueType::FixedBytes { len } => format!("{path}::FixedBytes {{ len: {len} }}"),
        ValueType::Interval { element } => {
            format!(
                "{path}::Interval {{ element: ::bumbledb::schema::IntervalElement::{} }}",
                element_suffix(*element)
            )
        }
        ValueType::FixedInterval { element, width } => {
            format!(
                "{path}::FixedInterval {{ element: ::bumbledb::schema::IntervalElement::{}, \
                 width: {width} }}",
                element_suffix(*element)
            )
        }
    }
}

/// Renders one lowered literal as its `Value` expression. String and
/// byte content re-escapes through std's escapers, so the emitted
/// literal round-trips the seam's decoded bytes exactly.
fn value_tokens(value: &Value) -> String {
    let path = "::bumbledb::Value";
    match value {
        Value::Bool(v) => format!("{path}::Bool({v})"),
        Value::U64(v) => format!("{path}::U64({v})"),
        Value::I64(v) => format!("{path}::I64({v})"),
        Value::String(text) => {
            format!(
                "{path}::String(::std::boxed::Box::from(\"{}\"))",
                text.escape_default()
            )
        }
        Value::FixedBytes(bytes) => format!(
            "{path}::FixedBytes(::std::boxed::Box::from(&b\"{}\"[..]))",
            bytes.escape_ascii()
        ),
        Value::IntervalU64(interval) => {
            let (start, end) = interval.bounds();
            format!(
                "{path}::IntervalU64(::bumbledb::Interval::<u64>::new({start}, {end})\
                 .expect(\"schema! interval literals are nonempty\"))"
            )
        }
        Value::IntervalI64(interval) => {
            let (start, end) = interval.bounds();
            format!(
                "{path}::IntervalI64(::bumbledb::Interval::<i64>::new({start}, {end})\
                 .expect(\"schema! interval literals are nonempty\"))"
            )
        }
    }
}

fn literal_set_tokens(set: &LiteralSet) -> String {
    match set {
        LiteralSet::One(value) => format!(
            "::bumbledb::schema::LiteralSet::One({})",
            value_tokens(value)
        ),
        LiteralSet::Many(values) => {
            let mut rendered = String::new();
            for value in values {
                let _ = write!(rendered, "{},", value_tokens(value));
            }
            format!("::bumbledb::schema::LiteralSet::Many(::std::boxed::Box::new([{rendered}]))")
        }
    }
}

fn side_tokens(side: &SideDescriptor) -> String {
    let projection = side
        .projection
        .iter()
        .map(|field| format!("::bumbledb::schema::FieldId({})", field.0))
        .collect::<Vec<_>>()
        .join(", ");
    let mut selection = String::new();
    for (field, set) in &side.selection {
        let _ = write!(
            selection,
            "(::bumbledb::schema::FieldId({}), {}),",
            field.0,
            literal_set_tokens(set)
        );
    }
    format!(
        "::bumbledb::schema::Side {{ \
             relation: ::bumbledb::schema::RelationId({}), \
             projection: ::std::boxed::Box::new([{projection}]), \
             selection: ::std::boxed::Box::new([{selection}]) }}",
        side.relation.0,
    )
}

fn statement_tokens(statement: &StatementDescriptor) -> String {
    match statement {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => {
            let fields = projection
                .iter()
                .map(|field| format!("::bumbledb::schema::FieldId({})", field.0))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "::bumbledb::schema::StatementDescriptor::Functionality {{ \
                     relation: ::bumbledb::schema::RelationId({}), \
                     projection: ::std::boxed::Box::new([{fields}]) }},",
                relation.0,
            )
        }
        StatementDescriptor::Containment { source, target } => format!(
            "::bumbledb::schema::StatementDescriptor::Containment {{ source: {}, target: {} }},",
            side_tokens(source),
            side_tokens(target),
        ),
        StatementDescriptor::Capacity {
            target,
            weight,
            lo,
            hi,
            source,
        } => {
            let weight = match weight {
                Weight::Unit => "::bumbledb::schema::Weight::Unit".to_owned(),
                Weight::Field(field) => format!(
                    "::bumbledb::schema::Weight::Field(::bumbledb::schema::FieldId({}))",
                    field.0
                ),
                Weight::DurationOf(field) => format!(
                    "::bumbledb::schema::Weight::DurationOf(::bumbledb::schema::FieldId({}))",
                    field.0
                ),
            };
            let hi = match hi {
                None => "::std::option::Option::None".to_owned(),
                Some(bound) => format!("::std::option::Option::Some({})", bound_tokens(*bound)),
            };
            format!(
                "::bumbledb::schema::StatementDescriptor::Capacity {{ \
                     target: {}, weight: {weight}, lo: {lo}u64, hi: {hi}, source: {} }},",
                side_tokens(target),
                side_tokens(source),
            )
        }
    }
}

fn bound_tokens(bound: bumbledb_theory::schema::Bound) -> String {
    let path = "::bumbledb::schema::Bound";
    match bound {
        bumbledb_theory::schema::Bound::Lit(n) => format!("{path}::Lit({n}u64)"),
        bumbledb_theory::schema::Bound::TargetField(field) => format!(
            "{path}::TargetField(::bumbledb::schema::FieldId({}))",
            field.0
        ),
        bumbledb_theory::schema::Bound::TargetDuration(field) => format!(
            "{path}::TargetDuration(::bumbledb::schema::FieldId({}))",
            field.0
        ),
    }
}

fn descriptor_tokens(descriptor: &SchemaDescriptor) -> String {
    let mut relations = String::new();
    for relation in &descriptor.relations {
        let mut fields = String::new();
        for field in &relation.fields {
            let _ = write!(
                fields,
                "::bumbledb::schema::FieldDescriptor {{ \
                     name: ::std::boxed::Box::from(\"{}\"), \
                     value_type: {}, \
                     generation: ::bumbledb::schema::Generation::{} }},",
                field.name,
                value_type_tokens(&field.value_type),
                match field.generation {
                    Generation::Fresh => "Fresh",
                    Generation::None => "None",
                },
            );
        }
        let extension = match &relation.extension {
            None => "::std::option::Option::None".to_owned(),
            Some(rows) => {
                let mut rendered = String::new();
                for row in rows {
                    let mut values = String::new();
                    for value in &row.values {
                        let _ = write!(values, "{},", value_tokens(value));
                    }
                    let _ = write!(
                        rendered,
                        "::bumbledb::schema::Row {{ \
                             handle: ::std::boxed::Box::from(\"{}\"), \
                             values: ::std::boxed::Box::new([{values}]) }},",
                        row.handle,
                    );
                }
                format!("::std::option::Option::Some(::std::boxed::Box::new([{rendered}]))")
            }
        };
        let _ = write!(
            relations,
            "::bumbledb::schema::RelationDescriptor {{ \
                 name: ::std::boxed::Box::from(\"{}\"), \
                 fields: ::std::vec![{fields}], \
                 extension: {extension} }},",
            relation.name,
        );
    }
    let mut statements = String::new();
    for statement in &descriptor.statements {
        let _ = write!(statements, "{}", statement_tokens(statement));
    }
    format!(
        "::bumbledb::schema::SchemaDescriptor {{\n\
             relations: ::std::vec![{relations}],\n\
             statements: ::std::vec![{statements}],\n\
         }}"
    )
}

fn emit_schema_def(out: &mut String, name: &str, descriptor: &SchemaDescriptor) {
    let _ = write!(
        out,
        "/// The `{name}` schema definition: the value `Db::create`/`Db::open` \
         take and the typestate `Db<{name}>` carries. Validation runs at \
         open, surfacing declaration errors as the typed `SchemaError`.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub struct {name};\n\
         impl ::bumbledb::Theory for {name} {{\n\
             fn descriptor(self) -> ::bumbledb::schema::SchemaDescriptor {{\n\
                 {}\n\
             }}\n\
         }}\n",
        descriptor_tokens(descriptor),
    );
}

/// A declaration name as a `SCREAMING_SNAKE` constant name: `SavingsTerms` →
/// `SAVINGS_TERMS`, `rate_bps` → `RATE_BPS` — an underscore lands before an
/// uppercase letter that starts a new word (after a lowercase/digit, or heading
/// a lowercase run after an uppercase run).
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

fn emit_id_constants(out: &mut String, schema: &SchemaAst) {
    let mut claimed: BTreeMap<String, String> = BTreeMap::new();
    let mut claim = |name: String, names: String| {
        if let Some(existing) = claimed.get(&name) {
            panic!(
                "schema!: id constants collide: `{name}` would name both {existing} \
                 and {names} — rename one declaration"
            );
        }
        claimed.insert(name.clone(), names);
        name
    };
    let mut body = String::new();
    for (rel_idx, relation) in schema.relations.iter().enumerate() {
        let rel_const = claim(
            screaming_snake(&relation.name),
            format!("relation `{}`", relation.name),
        );
        let _ = write!(
            body,
            "/// `{}` — the declaration-order relation id.\n\
             pub const {rel_const}: ::bumbledb::schema::RelationId = \
             ::bumbledb::schema::RelationId({rel_idx});\n",
            relation.name,
        );
        for (field_idx, field) in relation.fields.iter().enumerate() {
            let field_const = claim(
                format!(
                    "{}_{}",
                    screaming_snake(&relation.name),
                    screaming_snake(&field.name)
                ),
                format!("field `{}.{}`", relation.name, field.name),
            );
            let _ = write!(
                body,
                "/// `{}.{}` — the declaration-order field id.\n\
                 pub const {field_const}: ::bumbledb::schema::FieldId = \
                 ::bumbledb::schema::FieldId({field_idx});\n",
                relation.name, field.name,
            );
        }
    }
    let _ = write!(out, "impl {} {{\n{body}}}\n", schema.name);
}

fn emit_newtypes(out: &mut String, relations: &[Relation]) {
    let mut newtypes: BTreeMap<String, (String, String, bool)> = BTreeMap::new();
    for relation in relations {
        for field in &relation.fields {
            let Some(name) = &field.newtype else {
                continue;
            };

            // is an encoding artifact (the order-on-bytes refusal).

            let inner = match field.ty {
                FieldTy::U64 => ("u64".to_owned(), "u64".to_owned(), false),
                FieldTy::I64 => ("i64".to_owned(), "i64".to_owned(), false),
                FieldTy::FixedBytes(len) => (format!("bytes<{len}>"), format!("[u8; {len}]"), true),
                FieldTy::Interval(element) => (
                    format!("interval<{}>", element_rust(element)),
                    format!("::bumbledb::Interval<{}>", element_rust(element)),
                    true,
                ),
                FieldTy::FixedInterval(element, w) => (
                    format!("interval<{}, {w}>", element_rust(element)),
                    format!("::bumbledb::Interval<{}>", element_rust(element)),
                    true,
                ),
                _ => unreachable!("parser restricts `as` to u64/i64/bytes<N>/interval"),
            };
            if let Some((existing, ..)) = newtypes.get(name) {
                assert!(
                    *existing == inner.0,
                    "schema!: newtype `{name}` declared twice with different \
                     encodings: {existing} vs {}",
                    inner.0
                );
                continue;
            }
            newtypes.insert(name.clone(), inner);
        }
    }
    for (name, (_, inner, order_free)) in newtypes {
        let order = if order_free { "" } else { ", PartialOrd, Ord" };
        let _ = write!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash{order})]\n\
             pub struct {name}(pub {inner});\n",
        );
    }
}

fn emit_closed(out: &mut String, relations: &[Relation], descriptor: &SchemaDescriptor) {
    for (rel_idx, relation) in relations.iter().enumerate() {
        let Some(extension) = &relation.closed else {
            continue;
        };
        let name = &relation.name;
        let newtype = relation.fields[0]
            .newtype
            .as_deref()
            .expect("closed relations carry the handle newtype");
        let handles: Vec<&str> = extension
            .rows
            .iter()
            .map(|row| row.handle.as_str())
            .collect();
        let list = handles.join(", ");
        let mut id_arms = String::new();
        let mut from_arms = String::new();
        let mut weld = String::new();
        for (id, handle) in handles.iter().enumerate() {
            let _ = write!(id_arms, "Self::{handle} => {newtype}({id}),");
            let _ = write!(from_arms, "{id} => Some(Self::{handle}),");
            let _ = write!(
                weld,
                "assert_eq!(super::{name}::{handle}.id(), super::{newtype}({id}));\
                 assert_eq!(super::{name}::from_id(super::{newtype}({id})), \
                            Some(super::{name}::{handle}));"
            );
        }
        // The column accessors (ruled 2026-07-23, R14): one const fn per

        let lowered = descriptor.relations[rel_idx]
            .extension
            .as_deref()
            .expect("the lowering carries every closed extension");
        let mut accessors = String::new();
        for (column, field) in relation.fields[1..].iter().enumerate() {
            assert_ne!(
                field.name, "from_id",
                "schema!: closed relation `{name}` declares a column `from_id` — \
                 the emitted handle weld owns that name"
            );
            // `str` columns are refused on closed relations at

            let ty = if matches!(field.ty, FieldTy::Str) {
                "&'static str".to_owned()
            } else {
                rust_field_ty(field)
            };
            let mut arms = String::new();
            for (row, handle) in lowered.iter().zip(&handles) {
                let _ = write!(
                    arms,
                    "Self::{handle} => {},",
                    const_value_tokens(&row.values[column], field)
                );
            }
            let _ = write!(
                accessors,
                "/// The `{column}` ground-axiom column — an expansion-time \
                 constant per handle.\n\
                 #[must_use] pub const fn {column}(self) -> {ty} {{\n\
                     match self {{ {arms} }}\n\
                 }}\n",
                column = field.name,
            );
        }
        let _ = write!(
            out,
            "/// The host enum of the closed relation `{name}` — an emission, not a\n\
             /// type: variants are the handles in declaration order, welded to the\n\
             /// engine's row ids by [`{name}::id`]/[`{name}::from_id`].\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
             pub enum {name} {{ {list} }}\n\
             impl {name} {{\n\
                 /// The handle's declaration-order row id.\n\
                 #[must_use] pub const fn id(self) -> {newtype} {{\n\
                     match self {{ {id_arms} }}\n\
                 }}\n\
                 /// The handle a row id names; `None` beyond the extension.\n\
                 #[must_use] pub const fn from_id(id: {newtype}) -> Option<Self> {{\n\
                     match id.0 {{ {from_arms} _ => None }}\n\
                 }}\n\
                 {accessors}\
             }}\n",
        );
        let beyond = handles.len();
        let _ = write!(
            out,
            "#[cfg(test)]\n\
             mod __bumbledb_weld_{module} {{\n\
                 /// The emitted weld test: `from_id(h.id()) == Some(h)` for every\n\
                 /// handle, exhaustively, plus the beyond-roster miss — emitted per\n\
                 /// closed relation so the weld cannot be forgotten for a new theory.\n\
                 #[test]\n\
                 fn host_enum_weld() {{\n\
                     {weld}\n\
                     assert_eq!(super::{name}::from_id(super::{newtype}({beyond})), None);\n\
                 }}\n\
             }}\n",
            module = snake(name),
        );
    }
}

fn const_value_tokens(value: &Value, field: &Field) -> String {
    let raw = match value {
        Value::Bool(v) => format!("{v}"),
        Value::U64(v) => format!("{v}u64"),
        Value::I64(v) => format!("{v}i64"),
        Value::String(text) => {
            format!("\"{}\"", text.escape_default())
        }
        Value::FixedBytes(bytes) => format!("*b\"{}\"", bytes.escape_ascii()),
        Value::IntervalU64(interval) => {
            let (start, end) = interval.bounds();
            format!("::bumbledb::Interval::<u64>::__ground_axiom({start}, {end})")
        }
        Value::IntervalI64(interval) => {
            let (start, end) = interval.bounds();
            format!("::bumbledb::Interval::<i64>::__ground_axiom({start}, {end})")
        }
    };
    match &field.newtype {
        Some(newtype) => format!("{newtype}({raw})"),
        None => raw,
    }
}

fn snake(name: &str) -> String {
    screaming_snake(name).to_ascii_lowercase()
}

fn is_borrowed(field: &Field) -> bool {
    matches!(field.ty, FieldTy::Str)
}

fn rust_field_ty(field: &Field) -> String {
    if let Some(newtype) = &field.newtype {
        return newtype.clone();
    }
    match &field.ty {
        FieldTy::Bool => "bool".to_owned(),
        FieldTy::U64 => "u64".to_owned(),
        FieldTy::I64 => "i64".to_owned(),
        FieldTy::Str => "&'a str".to_owned(),
        FieldTy::FixedBytes(len) => format!("[u8; {len}]"),
        FieldTy::Interval(element) | FieldTy::FixedInterval(element, _) => {
            format!("::bumbledb::Interval<{}>", element_rust(*element))
        }
    }
}

struct EncodeCx<'s> {
    relation: &'s str,
}

fn value_type_expr(field: &Field) -> String {
    match &field.ty {
        FieldTy::Bool => "::bumbledb::schema::ValueType::Bool".to_owned(),
        FieldTy::U64 => "::bumbledb::schema::ValueType::U64".to_owned(),
        FieldTy::I64 => "::bumbledb::schema::ValueType::I64".to_owned(),
        FieldTy::Str => "::bumbledb::schema::ValueType::String".to_owned(),
        FieldTy::FixedBytes(len) => {
            format!("::bumbledb::schema::ValueType::FixedBytes {{ len: {len} }}")
        }
        FieldTy::Interval(element) => format!(
            "::bumbledb::schema::ValueType::Interval {{ element: ::bumbledb::schema::IntervalElement::{} }}",
            element_suffix(*element)
        ),
        FieldTy::FixedInterval(element, width) => format!(
            "::bumbledb::schema::ValueType::FixedInterval {{ element: ::bumbledb::schema::IntervalElement::{}, width: {width} }}",
            element_suffix(*element)
        ),
    }
}

fn encode_value(field: &Field, idx: usize, cx: &EncodeCx<'_>, insert: bool) -> String {
    let access = if field.newtype.is_some() {
        format!("self.{}.0", field.name)
    } else {
        format!("self.{}", field.name)
    };
    match &field.ty {
        FieldTy::Bool => format!("::bumbledb::__private::ValueRef::Bool({access})"),
        FieldTy::U64 => format!("::bumbledb::__private::ValueRef::U64({access})"),
        FieldTy::I64 => format!("::bumbledb::__private::ValueRef::I64({access})"),
        FieldTy::Interval(element) => format!(
            "::bumbledb::__private::ValueRef::Interval{}({access})",
            element_suffix(*element)
        ),
        FieldTy::FixedInterval(element, width) => format!(
            "::bumbledb::__private::fixed_interval_{}(\
             {relation}, \
             ::bumbledb::schema::FieldId({idx}), {access}, {width}u64)?",
            element_rust(*element),
            relation = cx.relation,
        ),
        FieldTy::FixedBytes(_) => {
            format!("::bumbledb::__private::ValueRef::bytes(&{access})")
        }
        FieldTy::Str if insert => format!(
            "::bumbledb::__private::ValueRef::String(context.intern_str(self.{})?)",
            field.name
        ),
        FieldTy::Str => format!(
            "match context.lookup_str(self.{})? {{ Some(id) => ::bumbledb::__private::ValueRef::String(id), None => return Ok(::bumbledb::Probe::ProvablyAbsent) }}",
            field.name
        ),
    }
}

fn decode_arm(field: &Field, idx: usize) -> String {
    let wrap = |expr: &str| -> String {
        match &field.newtype {
            Some(newtype) => format!("{newtype}({expr})"),
            None => expr.to_owned(),
        }
    };
    let decode = |method: &str| {
        format!("context.{method}(<Self as ::bumbledb::Fact<'a>>::RELATION, fact, {idx})?")
    };
    let expr = match &field.ty {
        FieldTy::Bool => decode("decode_bool_field"),
        FieldTy::U64 => wrap(&decode("decode_u64_field")),
        FieldTy::I64 => wrap(&decode("decode_i64_field")),
        FieldTy::Interval(element) | FieldTy::FixedInterval(element, _) => wrap(&decode(&format!(
            "decode_interval_{}_field",
            element_rust(*element)
        ))),
        FieldTy::Str => decode("decode_str_field"),
        FieldTy::FixedBytes(len) => wrap(&format!(
            "{{ let raw = {}; let mut arr = [0u8; {len}]; arr.copy_from_slice(raw); arr }}",
            decode("decode_fixed_bytes_field")
        )),
    };
    format!("{}: {expr},", field.name)
}

fn emit_fact_struct(
    out: &mut String,
    schema_name: &str,
    index: usize,
    relation: &Relation,
    fresh_base: usize,
) {
    let name = &relation.name;

    let borrowed = relation.fields.iter().any(is_borrowed);
    let (struct_params, self_ty) = if borrowed {
        ("<'a>", format!("{name}<'a>"))
    } else {
        ("", name.clone())
    };
    let mut struct_fields = String::new();
    for field in &relation.fields {
        let _ = write!(
            struct_fields,
            "pub {}: {},",
            field.name,
            rust_field_ty(field)
        );
    }

    let mut insert_values = String::new();
    let mut probe_values = String::new();
    let mut decode_fields = String::new();
    let cx = EncodeCx {
        relation: "<Self as ::bumbledb::Fact<'a>>::RELATION",
    };
    for (idx, field) in relation.fields.iter().enumerate() {
        let _ = write!(insert_values, "{},", encode_value(field, idx, &cx, true));
        let _ = write!(probe_values, "{},", encode_value(field, idx, &cx, false));
        let _ = write!(decode_fields, "{}", decode_arm(field, idx));
    }

    let _ = write!(
        out,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
         pub struct {name}{struct_params} {{ {struct_fields} }}\n\
         impl<'a> ::bumbledb::Fact<'a> for {self_ty} {{\n\
             type Schema = {schema_name};\n\
             const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
             fn encode_insert<C>(&self, context: &mut C, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<()>\n\
             where\n\
                 C: ::bumbledb::CodecWrite<{schema_name}>,\n\
             {{\n\
                 let values = [{insert_values}];\n\
                 ::bumbledb::__private::encode_fact_for(context, <Self as ::bumbledb::Fact<'a>>::RELATION, &values, out);\n\
                 Ok(())\n\
             }}\n\
             fn encode_probe<C>(&self, context: &C, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<::bumbledb::Probe>\n\
             where\n\
                 C: ::bumbledb::CodecRead<{schema_name}>,\n\
             {{\n\
                 let values = [{probe_values}];\n\
                 ::bumbledb::__private::encode_fact_for(context, <Self as ::bumbledb::Fact<'a>>::RELATION, &values, out);\n\
                 Ok(::bumbledb::Probe::Encoded)\n\
             }}\n\
             fn decode<C>(context: &'a C, fact: &[u8]) -> ::bumbledb::Result<Self>\n\
             where\n\
                 C: ::bumbledb::CodecRead<{schema_name}>,\n\
             {{\n\
                 Ok(Self {{ {decode_fields} }})\n\
             }}\n\
         }}\n",
    );

    let mut auto_key_id = fresh_base;
    for (field_idx, field) in relation.fields.iter().enumerate() {
        let (true, Some(newtype)) = (field.fresh, &field.newtype) else {
            continue;
        };
        let _ = write!(
            out,
            "impl ::bumbledb::Fresh for {newtype} {{\n\
                 type Schema = {schema_name};\n\
                 const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
                 const FIELD: ::bumbledb::schema::FieldId = ::bumbledb::schema::FieldId({field_idx});\n\
                 fn from_fresh(raw: u64) -> Self {{ Self(raw) }}\n\
                 fn fresh(self) -> u64 {{ self.0 }}\n\
             }}\n\
             impl<'a> ::bumbledb::Key<'a> for {newtype} {{\n\
                 type Schema = {schema_name};\n\
                 type Fact = {self_ty};\n\
                 const STATEMENT: ::bumbledb::schema::StatementId = ::bumbledb::schema::StatementId({auto_key_id});\n\
                 fn encode_determinant<C>(&self, _context: &C, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<::bumbledb::Probe>\n\
                 where\n\
                     C: ::bumbledb::CodecRead<{schema_name}>,\n\
                 {{\n\
                     ::bumbledb::__private::append_field(::bumbledb::__private::ValueRef::U64(self.0), ::bumbledb::schema::ValueType::U64, out);\n\
                     Ok(::bumbledb::Probe::Encoded)\n\
                 }}\n\
             }}\n",
        );
        auto_key_id += 1;
    }
}

fn pascal(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for segment in name.split('_') {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn emit_key_structs(out: &mut String, schema: &SchemaAst) {
    let implied_total: usize = schema
        .relations
        .iter()
        .map(|relation| {
            relation.fields.iter().filter(|field| field.fresh).count()
                + usize::from(relation.closed.is_some())
        })
        .sum();
    let mut offset = 0usize;
    for statement in &schema.statements {
        let width = match statement {
            Statement::Containment {
                bidirectional: true,
                ..
            } => 2,
            _ => 1,
        };
        let id = implied_total + offset;
        offset += width;
        let Statement::Functionality {
            relation,
            projection,
            ..
        } = statement
        else {
            continue;
        };
        let (rel_idx, rel) = schema
            .relations
            .iter()
            .enumerate()
            .find(|(_, r)| r.name == *relation)
            .expect("the shared lowering resolved every statement relation");
        if rel.closed.is_some() {
            continue;
        }
        emit_key_struct(out, &schema.name, rel_idx, rel, projection, id);
    }
}

fn emit_key_struct(
    out: &mut String,
    schema_name: &str,
    rel_idx: usize,
    relation: &Relation,
    projection: &[(String, Span)],
    statement_id: usize,
) {
    let rel_name = &relation.name;
    let fields: Vec<(usize, &Field)> = projection
        .iter()
        .map(|(name, _)| {
            relation
                .fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == *name)
                .expect("the shared lowering resolved every projected field")
        })
        .collect();
    let key_name = format!(
        "{rel_name}By{}",
        projection
            .iter()
            .map(|(name, _)| pascal(name))
            .collect::<String>()
    );
    let borrowed = fields.iter().any(|(_, field)| is_borrowed(field));
    let fact_borrowed = relation.fields.iter().any(is_borrowed);
    let (struct_params, impl_params, impl_ty) = if borrowed {
        ("<'a>", "<'a, 'k>", format!("{key_name}<'k>"))
    } else {
        ("", "<'a>", key_name.clone())
    };
    let fact_ty = if fact_borrowed {
        format!("{rel_name}<'a>")
    } else {
        rel_name.clone()
    };
    let mut struct_fields = String::new();
    for (_, field) in &fields {
        let _ = write!(
            struct_fields,
            "pub {}: {},",
            field.name,
            rust_field_ty(field)
        );
    }

    let relation_expr = format!("::bumbledb::schema::RelationId({rel_idx})");
    let cx = EncodeCx {
        relation: &relation_expr,
    };
    let ctx_binding = if fields
        .iter()
        .any(|(_, field)| matches!(field.ty, FieldTy::Str))
    {
        "context"
    } else {
        "_context"
    };
    let mut body = String::new();
    for (idx, field) in &fields {
        let expr = encode_value(field, *idx, &cx, false);
        let ty = value_type_expr(field);
        let _ = write!(
            body,
            "::bumbledb::__private::append_field({expr}, {ty}, out);"
        );
    }
    let spelling = format!(
        "{rel_name}({}) -> {rel_name}",
        projection
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = write!(
        out,
        "/// The typed key of `{spelling}` — `snap.get(..)` / `tx.get(..)`\n\
         /// return `Option<{rel_name}>`.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
         pub struct {key_name}{struct_params} {{ {struct_fields} }}\n\
         impl{impl_params} ::bumbledb::Key<'a> for {impl_ty} {{\n\
             type Schema = {schema_name};\n\
             type Fact = {fact_ty};\n\
             const STATEMENT: ::bumbledb::schema::StatementId = ::bumbledb::schema::StatementId({statement_id});\n\
             fn encode_determinant<C>(&self, {ctx_binding}: &C, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<::bumbledb::Probe>\n\
             where\n\
                 C: ::bumbledb::CodecRead<{schema_name}>,\n\
             {{\n\
                 {body}\n\
                 Ok(::bumbledb::Probe::Encoded)\n\
             }}\n\
         }}\n",
    );
}
