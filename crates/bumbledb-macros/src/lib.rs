//! The `schema!` proc-macro (docs/architecture/70-api.md): bumbledb's declarative schema
//! surface. A small, rigid grammar — this is Rust-side declaration, not a
//! query language — hand-parsed over the raw token stream (no `syn`, no
//! `quote`: the grammar is not Rust syntax and the dependency would buy
//! nothing).
//!
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
//!
//! The header `pub Ledger;` is the invocation's first item and names the
//! schema: it expands to `pub struct Ledger;` implementing
//! `bumbledb::Theory`, the value `Db::create(path, Ledger)` takes and
//! the typestate `Db<Ledger>` carries. Multiple schemas coexist in one
//! module — their headers disambiguate.
//!
//! Types: `bool`, `u64`, `i64`, `str`, `bytes<N>` (N ∈ 1..=64 — the
//! width is mandatory; bare `bytes` does not exist), `interval<i64>`,
//! `interval<u64>`, and the fixed-width interval family
//! `interval<u64, w>` / `interval<i64, w>` (w ≥ 1 an integer literal —
//! the width is the type, the encoding stores only the start; `w = 0`
//! and a trailing comma with no width are expansion errors naming the
//! field); a vocabulary is a closed relation, never a type. `as NewType` generates the host-side nominal newtype
//! (legal on u64, i64, `bytes<N>`, and both intervals). `fresh`
//! auto-materializes `R(field) -> R` at schema resolution. **There are no field-level constraint modifiers** — everything
//! relational is a dependency statement between the relation blocks
//! (docs/architecture/30-dependencies.md): `R(X) -> R` (functionality —
//! read as the functional dependency it spells: the key projection
//! DETERMINES the tuple, and the arrow closing over its own relation is
//! what makes a key a key; a right side naming any other relation is a
//! spanned teaching error, not a key statement),
//! `A(X | σ) <= B(Y | ψ)` (containment), `==` lowered here to the two
//! adjacent containments, `A <= B` first;
//! `B(Y | ψ) <={lo..hi} A(X | σ)` (the cardinality window — B-family,
//! target-left: per selected B fact, the count of selected A facts
//! sharing its projected tuple lies in the window). The window
//! vocabulary is closed under the canonical-utterance law
//! (docs/architecture/70-api.md): `{n}` is THE exact-count spelling
//! (`{0}` the exclusion), `{lo..hi}` with lo < hi, `{lo..*}` floors
//! (lo ≥ 2), `{0..hi}` ceilings — every other spelling (`{n..n}`,
//! `{0..0}`, `{1..*}`, `{0..*}`, inverted bounds, open shorthands) is
//! an expansion error naming the canonical form. Selection literals are typed
//! against the selected field in the macro (a bare handle resolves through
//! the selected field's newtype to its closed relation's row id); interval
//! literals are written `start..end`, half-open; a binding may carry a
//! literal SET — `field == {A, B}`, read disjunctively (a one-element
//! set is the bare literal — `{L}` and `{}` are expansion errors naming
//! the bare spelling).
//!
//! **Closed relations** declare their extension in the schema — rows are
//! ground axioms, handle = declaration-order row id
//! (`docs/architecture/70-api.md` § the `schema!` grammar):
//!
//! ```text
//! closed relation Status as StatusId = { Open, Frozen, Closed };
//! closed relation Kind as KindId {
//!     mastered: bool,
//! } = {
//!     DirectPass { mastered: true },
//!     Failed     { mastered: false },
//! };
//! ```
//!
//! `as NewType` is required (the handle needs a host type); the column
//! block is optional; the extension block is non-empty, each row carrying
//! every declared column exactly once (missing/extra/duplicate columns,
//! duplicate handles, and type-mismatched literals are expansion panics
//! naming the offender). The emission per closed relation: the **host
//! enum** (an emission, not a type — the engine's vocabulary is
//! relational; the macro projects it into a Rust enum so rustc's pattern
//! checking keeps working, welded to the row ids by const `id`/`from_id`
//! and pinned by an emitted weld test), the handle newtype through the
//! ordinary newtype machinery, and the descriptor's extension. **No fact
//! struct and no `Fact` impl** — closed relations are unwritable. A bare
//! handle in a statement selection (`| status == Frozen`) resolves through
//! the selected field's newtype to its owning closed relation's row id.
//!
//! Generated fact structs borrow their one variable-width field kind
//! (`str` → `&'a str`): a struct with any `str` field gains one lifetime.
//! `bytes<N>` fields are `[u8; N]` — owned, `Copy`, lifetime-free (the
//! fixed-width law) — so all-fixed-width structs stay lifetime-free.
//!
//! The macro judges its own grammar and literal typing (expansion
//! errors at the call site; the token→`Value` conversion is where a
//! literal that does not fit its field dies). Name→id resolution and the
//! canonical-utterance ban table are NOT the macro's: the parse builds a
//! [`bumbledb_theory::schema::spec::SchemaSpec`] plus a span table and
//! runs the ONE shared lowering (`SchemaSpec::descriptor`) at expansion —
//! the macro and the runtime spec path cannot drift — and every
//! `SpecIssue` lands as a `compile_error!` at the offending token. The
//! lowered `SchemaDescriptor` is then emitted as const construction.
//! Everything semantic beyond names surfaces as the typed `SchemaError`
//! from `Db::create`/`Db::open`, where the descriptor is validated.

use bumbledb_theory::schema::spec::{
    FieldSpec, LiteralAt, LiteralSetSpec, LiteralSpec, RelationSpec, RowSpec, SchemaSpec, SideSpec,
    SpecIssue, StatementSide, StatementSpec, WindowSpec,
};
use bumbledb_theory::schema::{
    Generation, IntervalElement, LiteralSet, SchemaDescriptor, Side as SideDescriptor,
    StatementDescriptor, ValueType,
};
use bumbledb_theory::{Interval, Value};
use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::iter::Peekable;

/// The Rust scalar type an interval element ranges over.
fn element_rust(element: IntervalElement) -> &'static str {
    match element {
        IntervalElement::U64 => "u64",
        IntervalElement::I64 => "i64",
    }
}

/// The engine-side variant-name suffix (`IntervalU64` / `IntervalI64`).
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
    /// `bytes<N>` — the width is part of the type (mandatory in the
    /// grammar; range-validated at `Db::create`/`open`).
    FixedBytes(u64),
    /// `interval<E>` (width `None`) or `interval<E, w>` (width
    /// `Some(w)`, `w ≥ 1` — the width is the type; the grammar rejects
    /// `w = 0` here, and `Db::create`/`open` re-validates the range).
    Interval(IntervalElement, Option<u64>),
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
    /// The sealed field list: for a closed relation the synthetic
    /// (`id`, `u64 as Handle`) field is materialized at index 0 at parse,
    /// so statement field ids, id constants, and the newtype emission all
    /// see the sealed shape (`FieldId(0)` = the handle's row id). The
    /// spec built for the lowering carries declared columns only —
    /// `validate()` prepends its own synthetic field.
    fields: Vec<Field>,
    /// The handle newtype token's span of a closed relation — where a
    /// `DuplicateHandleNewtype` issue points.
    newtype_span: Option<Span>,
    /// `Some` declares the relation **closed**: its extension is the row
    /// list, ground axioms in declaration order. The option is the kind,
    /// mirroring `RelationDescriptor.extension`.
    closed: Option<Closed>,
}

/// A closed relation's parsed extension.
#[derive(Debug, Clone)]
struct Closed {
    rows: Vec<ClosedRow>,
}

/// One ground axiom as written: the handle plus (column, literal) pairs
/// reordered to column-declaration order at parse (coverage checked there
/// — every declared column exactly once).
#[derive(Debug, Clone)]
struct ClosedRow {
    handle: String,
    values: Vec<(String, Literal)>,
}

/// A selection literal as written — classified by its own syntax, typed
/// against the selected field's declaration at the token→`Value` seam
/// ([`typed_literal`]), where a literal that does not fit dies at
/// expansion.
#[derive(Debug, Clone)]
enum Literal {
    Bool(bool),
    /// `[-] int`.
    Int {
        negative: bool,
        text: String,
    },
    /// A bare ident: a closed relation's handle, resolved to its
    /// declaration-order row id through the selected field's newtype by
    /// the shared lowering. The span is the ident's — where the
    /// handle-shaped issues point.
    Handle(String, Span),
    /// A string literal's raw token text, quotes included.
    Str(String),
    /// A byte-string literal's raw token text.
    Bytes(String),
    /// `start..end`, half-open — each bound `[-] int`.
    Interval {
        start: (bool, String),
        end: (bool, String),
    },
}

/// One σ binding's right side: a single literal (`f == L`, the equality
/// spelling) or a braced literal set (`f == {A, B}`, the disjunctive
/// spelling — `docs/architecture/30-dependencies.md`). The degenerate
/// sets (`{L}`, `{}`) parse and are banned by the shared lowering
/// (`DegenerateLiteralSet`), the error naming the canonical form.
#[derive(Debug, Clone)]
enum Literals {
    One(Literal),
    Many(Vec<Literal>),
}

/// One σ binding as written: the selected field, its ident's span, the
/// right side, and — for the braced set spelling — the brace group's
/// span (where a `DegenerateLiteralSet` issue points).
#[derive(Debug, Clone)]
struct Binding {
    field: String,
    field_span: Span,
    literals: Literals,
    set_span: Option<Span>,
}

/// One side of a dependency statement:
/// `R(fields [ | field == literal-or-set, .. ])`. Name spans ride along
/// for the lowering's span table.
#[derive(Debug, Clone)]
struct Side {
    relation: String,
    relation_span: Span,
    projection: Vec<(String, Span)>,
    selection: Vec<Binding>,
}

/// One parsed dependency statement, one per [`StatementSpec`] — `==` is
/// the `bidirectional` containment spelling, lowered to the two adjacent
/// descriptors (`A <= B` first) by the shared lowering, not here.
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
    /// `B(Y | ψ) <={window} A(X | σ);` — B-family, target-left: the
    /// LEFT side is the window's target (the per-group parent), the
    /// right side the counted source. The spelling survives as written —
    /// the shared lowering owns the ban table — and the brace group's
    /// span is where a banned spelling's error points.
    Cardinality {
        source: Side,
        window: WindowSpec,
        window_span: Span,
        target: Side,
    },
}

/// The whole parsed invocation: the header's schema name, relation blocks,
/// and dependency statements, each list in source order.
struct SchemaAst {
    /// The `pub Name;` header's name — the emitted `Theory` unit struct.
    name: String,
    relations: Vec<Relation>,
    statements: Vec<Statement>,
}

type Tokens = Peekable<proc_macro::token_stream::IntoIter>;

fn expect_ident(tokens: &mut Tokens, what: &str) -> String {
    spanned_ident(tokens, what).0
}

/// An expected ident plus its span — the span table's raw material.
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

fn take_group(tokens: &mut Tokens, delimiter: Delimiter, what: &str) -> TokenStream {
    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == delimiter => group.stream(),
        other => panic!("schema!: expected {what}, found {other:?}"),
    }
}

/// The deleted field-modifier vocabulary never parses, anywhere
/// (docs/architecture/30-dependencies.md — everything relational is a
/// statement).
fn reject_deleted_word(word: &str) {
    assert!(
        !matches!(word, "unique" | "fk"),
        "schema!: field-level constraints do not exist; write a statement — \
         see docs/architecture/30-dependencies.md"
    );
    assert!(
        word != "enum",
        "schema!: the enum type is deleted — a vocabulary is a closed relation \
         (`closed relation K as KId = {{ A, B }};` plus `Rel(k) <= K(id);` — \
         see docs/architecture/10-data-model.md)"
    );
}

/// Parses a comma-separated identifier list.
/// Parses one relation body: fields only — everything relational is a
/// statement outside the block.
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

/// Parses a field's type, optional `as NewType`, and optional `, fresh`.
fn parse_field(name: String, tokens: &mut Tokens) -> Field {
    let ty_name = expect_ident(tokens, "a type (bool/u64/i64/str/bytes<N>/interval)");
    reject_deleted_word(&ty_name);
    let ty = match ty_name.as_str() {
        "bool" => FieldTy::Bool,
        "u64" => FieldTy::U64,
        "i64" => FieldTy::I64,
        "str" => FieldTy::Str,
        // The width is mandatory: bare `bytes` is not a type — the
        // variable-width binary type is deleted (identity-shaped values
        // are bytes<N>; reuse-shaped text is str).
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
            let width: u64 = text
                .parse()
                .unwrap_or_else(|_| panic!("schema!: malformed bytes<N> width `{text}`"));
            FieldTy::FixedBytes(width)
        }
        "interval" => {
            expect_punct(tokens, '<');
            let element = match expect_ident(tokens, "an interval element (i64/u64)").as_str() {
                "u64" => IntervalElement::U64,
                "i64" => IntervalElement::I64,
                other => panic!("schema!: interval element must be i64 or u64, found `{other}`"),
            };
            let width = parse_interval_width(&name, tokens);
            expect_punct(tokens, '>');
            FieldTy::Interval(element, width)
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
                FieldTy::U64 | FieldTy::I64 | FieldTy::FixedBytes(_) | FieldTy::Interval(..)
            ),
            "schema!: `as NewType` applies to u64/i64/bytes<N>/interval fields only"
        );
        field.newtype = Some(expect_ident(tokens, "a newtype name"));
    }
    // Trailing modifier: `, fresh` — distinguished from the next field
    // (an ident followed by `:`) by lookahead.
    if peek_punct(tokens, ',') {
        let mut lookahead = tokens.clone();
        lookahead.next(); // the comma
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
                    field.newtype.is_some(),
                    "schema!: fresh field `{}` needs `as NewType` — without it \
                     there is no typed alloc path (use the descriptor API for a \
                     raw-u64 fresh field)",
                    field.name
                );
                field.fresh = true;
                tokens.next(); // the comma
                tokens.next(); // `fresh`
            }
        }
    }
    field
}

/// The optional interval width: `interval<u64, w>` — the fixed-width
/// family, w ≥ 1 an integer literal. A trailing comma with no width and
/// w = 0 are grammar errors naming the field.
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
    let width: u64 = text
        .parse()
        .unwrap_or_else(|_| panic!("schema!: field `{name}`: malformed interval width `{text}`"));
    assert!(
        width >= 1,
        "schema!: field `{name}`: interval<E, 0> denotes nothing — the width must be >= 1"
    );
    Some(width)
}

/// Whether the next token is a brace-delimited group.
fn peek_brace(tokens: &mut Tokens) -> bool {
    matches!(tokens.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
}

/// Parses one closed relation, `closed relation` already consumed:
/// `Name as Handle [ { columns } ] = { rows };`. The `as NewType` is
/// required (the handle needs a host type); the column block is optional;
/// the extension block is required and non-empty. The returned relation's
/// field list opens with the synthetic (`id`, `u64 as Handle`) field —
/// the sealed shape, materialized here so statement field ids, id
/// constants, and the newtype emission all address it uniformly.
fn parse_closed_relation(tokens: &mut Tokens) -> Relation {
    let name = expect_ident(tokens, "a relation name");
    assert_eq!(
        peek_ident(tokens).as_deref(),
        Some("as"),
        "schema!: closed relation `{name}` needs `as NewType` — the handle needs a host type \
         (docs/architecture/70-api.md)"
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

/// Parses the extension block: each row is `Handle` or
/// `Handle { column: literal, ... }` with every declared column present
/// exactly once — duplicate handles and missing/extra/duplicate columns
/// panic naming the offender. `declaration` still holds declared columns
/// only (the synthetic id lands after this returns). Row values are
/// reordered to column-declaration order.
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

/// The integer-literal token shape: digits first, no float dot. Shared
/// between [`parse_int`] and [`parse_literal`]'s bare-literal fallback.
fn is_int_text(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_digit()) && !text.contains('.')
}

/// Parses one `[-] int`, returning the sign and the raw token text —
/// range and radix are judged at the token→`Value` seam, against the
/// field's declared type.
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

/// An integer already begun: either a scalar or, on `..`, the start of a
/// half-open `start..end` interval literal.
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

/// Parses one selection literal: `int`, `-int`, `true`/`false`, a bare
/// handle ident, a string/byte-string literal, or `start..end`.
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

/// Parses one binding's right side: a braced literal set (`{A, B}`) or a
/// single literal. The degenerate sets (`{L}`, `{}`) parse here and are
/// banned by the shared lowering (the canonical-utterance law's
/// `DegenerateLiteralSet`, its error naming the canonical form) — the
/// returned span is the brace group's, where that error points.
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

/// Parses `fields [ | field == literal-or-set, .. ]` out of one side's
/// parens.
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

/// Parses `Rel(...)` — the right-hand side of `<=` / `==`.
fn parse_statement_side(tokens: &mut Tokens) -> Side {
    let (relation, relation_span) = spanned_ident(tokens, "a relation name");
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    parse_side(relation, relation_span, group)
}

/// A parse-stage teaching error: the offending token's span plus the
/// message. `schema()` lands it as a `compile_error!` at the token —
/// the same landing the lowering's `SpecIssue`s take — for the grammar
/// mistakes that carry MEANING worth teaching (today: the key arrow's
/// right side), where an expansion panic at the invocation would bury
/// the lesson.
struct ParseError {
    span: Span,
    message: String,
}

/// Parses one dependency statement, `relation` being its left relation
/// name (already consumed, its span in hand). `==` survives as the
/// `bidirectional` containment spelling — the shared lowering lowers it
/// to two adjacent `Containment`s, `A <= B` first
/// (docs/architecture/30-dependencies.md).
fn parse_statement(
    relation: String,
    relation_span: Span,
    tokens: &mut Tokens,
    statements: &mut Vec<Statement>,
) -> Result<(), ParseError> {
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    let left = parse_side(relation, relation_span, group);
    match tokens.next() {
        // `->`: functionality. The right side is the side's own relation
        // — the arrow closing over it is the dependency-theoretic reading
        // (OWNER RULING 2026-07-18: the arrow is canon, never respelled) —
        // and the FD form takes no selection: the engine descriptor
        // carries none by construction (the shape is unrepresentable, not
        // rejected downstream), so the grammar is the judge here.
        Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
            expect_punct(tokens, '>');
            let (right, right_span) = spanned_ident(tokens, "the FD's relation name");
            assert!(
                left.selection.is_empty(),
                "schema!: an FD takes no selection — the FD form is `R(X) -> R` \
                 (docs/architecture/30-dependencies.md)"
            );
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
                         `-> {right}` is not a key statement \
                         (docs/architecture/30-dependencies.md)",
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
        // `<=`: containment — or, with a brace group riding the operator
        // (`<={lo..hi}`), the cardinality window: B-family, target-left —
        // the LEFT side is the window's target (the per-group parent),
        // the right side the counted source.
        Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
            expect_punct(tokens, '=');
            if peek_brace(tokens) {
                let Some(TokenTree::Group(group)) = tokens.next() else {
                    unreachable!("peeked a brace group");
                };
                let window_span = group.span();
                let spelling = parse_window(group.stream());
                let right = parse_statement_side(tokens);
                statements.push(Statement::Cardinality {
                    source: right,
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
        // `==`: set equality — the bidirectional containment spelling.
        Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
            expect_punct(tokens, '=');
            let right = parse_statement_side(tokens);
            statements.push(Statement::Containment {
                source: left,
                target: right,
                bidirectional: true,
            });
        }
        // The deleted `in lo..hi per` window spelling never parses —
        // the window is B-family, target-left (the canonical-utterance
        // law: one meaning, one spelling).
        Some(TokenTree::Ident(ident)) if ident.to_string() == "in" => {
            panic!(
                "schema!: the `in lo..hi per` window form is deleted — a window is \
                 B-family, target-left: `Parent(key) <={{lo..hi}} Child(field)`, \
                 with `{{n}}` the exact-count spelling \
                 (docs/architecture/30-dependencies.md § the extension form)"
            );
        }
        other => panic!("schema!: expected `->`, `<=`, `<={{lo..hi}}`, or `==`, found {other:?}"),
    }
    expect_punct(tokens, ';');
    Ok(())
}

/// One window bound out of the brace group: a non-negative integer,
/// parsed here (not spliced) because the canonical-utterance law compares
/// bounds at expansion.
fn parse_window_bound(tokens: &mut Tokens, what: &str) -> u64 {
    let (negative, text) = parse_int(tokens, what);
    assert!(
        !negative,
        "schema!: a window bound is a count — non-negative"
    );
    text.replace('_', "")
        .parse()
        .unwrap_or_else(|_| panic!("schema!: malformed window bound `{text}`"))
}

/// Parses the `<={…}` brace group into the spelling as written — the
/// shared lowering's [`WindowSpec`], judged by its ban table, never
/// here. The open shorthands are not spellable in the spec vocabulary,
/// so the grammar itself refuses them: `{..hi}` and `{lo..}` are
/// expansion panics naming the explicit form.
fn parse_window(body: TokenStream) -> WindowSpec {
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
    let lo = parse_window_bound(&mut tokens, "the window's lower bound");
    if tokens.peek().is_none() {
        return WindowSpec::Exact(lo);
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
        WindowSpec::Floor(lo)
    } else {
        WindowSpec::Range {
            lo,
            hi: parse_window_bound(&mut tokens, "the window's upper bound"),
        }
    };
    assert!(
        tokens.peek().is_none(),
        "schema!: trailing tokens after the window bounds"
    );
    spelling
}

/// Parses the whole `schema!` body: the `pub Name;` header first, then
/// relation blocks and dependency statements in any order. `Err` is the
/// parse's one teaching error (the key arrow's foreign right side),
/// spanned at the offending token.
fn parse_schema(input: TokenStream) -> Result<SchemaAst, ParseError> {
    let mut tokens = input.into_iter().peekable();
    match tokens.next() {
        Some(TokenTree::Ident(ident)) if ident.to_string() == "pub" => {}
        other => panic!(
            "schema!: the first item names the schema — `pub Name;` — found {other:?} \
             (docs/architecture/70-api.md)"
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
            // The order-mark form left the vocabulary whole
            // (`docs/architecture/30-dependencies.md` § refused: order
            // marks) — the old spelling is unrepresentable, rejected by
            // the grammar itself, never the validator.
            panic!(
                "schema!: `order` statements no longer exist — order is a derivation, \
                 not a dependency: use fractional indexing over a keyed position, or \
                 the exact-partition interval recipe \
                 (docs/architecture/30-dependencies.md § refused: order marks)"
            );
        } else {
            parse_statement(ident, ident_span, &mut tokens, &mut schema.statements)?;
        }
    }
    Ok(schema)
}

/// The declarative schema surface: expands to the header's `Theory`
/// unit struct, host-side newtypes and host enums, and one typed fact struct
/// per relation with `encode_write`/`encode_delete`/`encode_read`/`decode`
/// boundaries. The expansion builds a `SchemaSpec` plus a span table, runs
/// the ONE shared lowering (`SchemaSpec::descriptor` — name→id resolution
/// and the canonical-utterance ban table, the same pass the runtime spec
/// path runs), and emits the lowered `SchemaDescriptor` as const
/// construction. Semantic validation beyond names runs where the
/// definition is consumed (`Db::create`/`Db::open`, as the typed
/// `SchemaError`).
///
/// # Panics
///
/// On malformed `schema!` grammar or a literal that does not fit its
/// field's declared type — a compile error at the macro call site.
/// Lowering issues (unresolvable names, banned spellings) are not panics:
/// each becomes a `compile_error!` at the offending token — as does the
/// parse's teaching error (a key arrow whose right side names a foreign
/// relation), spanned at the offending name.
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
    emit_closed(&mut out, &schema.relations);
    for (index, relation) in schema.relations.iter().enumerate() {
        // No fact struct and no `Fact`/`Fresh` impls for a closed relation:
        // its rows are ground axioms and the relation is unwritable — a
        // writable struct would be a lie the type system tells. Reads go
        // through queries and the dyn surface
        // (`docs/architecture/70-api.md`).
        if relation.closed.is_some() {
            continue;
        }
        emit_fact_struct(&mut out, &schema.name, index, relation);
    }
    out.parse().expect("schema!: generated code parses")
}

/// The parse's span table: every issue the shared lowering can return
/// maps through here to the offending token. Handle-shaped issues carry
/// their own key — the structural [`LiteralAt`] address — and the
/// duplicate-newtype issue carries relation indices; the name-keyed
/// multimaps serve the rest, and every span under a matched key is
/// offending (the key carries enough context that no innocent token
/// shares it), so all are marked.
#[derive(Default)]
struct SpanTable {
    /// (statement, relation name) → the relation ident's spans.
    relations: BTreeMap<(usize, String), Vec<Span>>,
    /// (statement, relation name, field name) → the field idents' spans,
    /// projection and selection occurrences alike.
    fields: BTreeMap<(usize, String, String), Vec<Span>>,
    /// statement → its window brace group's span.
    windows: BTreeMap<usize, Span>,
    /// (statement, field name, set len) → literal-set brace spans.
    sets: BTreeMap<(usize, String, usize), Vec<Span>>,
    /// A handle literal's structural address → its ident's span.
    literals: BTreeMap<LiteralAt, Span>,
    /// relation index → its handle newtype ident's span (closed only).
    newtypes: BTreeMap<usize, Span>,
}

/// The typing seam's lookup: the declared type of `relation.field`, if
/// both names resolve. Name→id RESOLUTION is the shared lowering's — this
/// lookup only types literal tokens, and an unknown name yields `None`
/// (the literal gets a placeholder and the lowering reports the name).
/// Closed relations are searched in their sealed shape — the synthetic
/// `id` field is in the AST's list, so `| id == 1` types as `u64`.
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

/// One selection literal into the spec: handles pass through by name
/// (the lowering resolves them), everything else runs the token→`Value`
/// seam against the field's declared type — or a placeholder when the
/// name itself is unresolvable, which the lowering reports (a nonempty
/// issue list fails the whole expansion, so placeholders never escape).
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

/// The token→`Value` seam — the macro's half of the two-boundary split:
/// literal TYPING stays an expansion error here (a literal that does not
/// fit its field's declared type never degrades to a `Db::create`
/// error), while name resolution and the ban table are the shared
/// lowering's. One machine for statement selections and closed rows —
/// same errors, both call sites.
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
        (FieldTy::Str, Literal::Str(text)) => Value::String(unescape_str(text).into()),
        // The width is the type: a `bytes<N>` literal of any other
        // length is a typing mismatch, judged here (the theory's
        // judgment, `bumbledb-theory/src/schema.rs: value_inhabits`).
        (FieldTy::FixedBytes(len), Literal::Bytes(text)) => {
            let bytes = unescape_bytes(text);
            if u64::try_from(bytes.len()) != Ok(*len) {
                literal_mismatch(relation, field);
            }
            Value::FixedBytes(bytes.into())
        }
        (
            FieldTy::Interval(IntervalElement::U64, width),
            Literal::Interval {
                start: (false, start),
                end: (false, end),
            },
        ) => {
            let start = u64_text(start).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = u64_text(end).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<u64>::new(start, end));
            // `interval<E, w>`: the spelled width must be exactly `w`
            // and never the unbounded ray — the theory's judgment.
            if let Some(w) = width
                && (interval.end() - interval.start() != *w || interval.is_ray())
            {
                literal_mismatch(relation, field);
            }
            Value::IntervalU64(interval)
        }
        (FieldTy::Interval(IntervalElement::I64, width), Literal::Interval { start, end }) => {
            let start =
                i64_text(start.0, &start.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let end = i64_text(end.0, &end.1).unwrap_or_else(|| literal_mismatch(relation, field));
            let interval = nonempty_interval(relation, field, Interval::<i64>::new(start, end));
            if let Some(w) = width
                && (interval.end().abs_diff(interval.start()) != *w || interval.is_ray())
            {
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

/// Interval literals are nonempty by the width law — judged here at the
/// seam (an empty literal is a typing error, not a `Db::create` one).
fn nonempty_interval<T>(relation: &str, field: &str, interval: Option<T>) -> T {
    interval.unwrap_or_else(|| {
        panic!(
            "schema!: the interval literal for `{relation}.{field}` is empty — \
             `start..end` is half-open, start < end"
        )
    })
}

/// An integer literal's magnitude out of its token text: underscores
/// dropped, the `0x`/`0o`/`0b` radix prefixes honored (the seam parses
/// what rustc would have; type suffixes are not part of the grammar).
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

/// A `u64` literal value, or `None` when the text is not one.
fn u64_text(text: &str) -> Option<u64> {
    u64::try_from(int_magnitude(text)?).ok()
}

/// An `i64` literal value from sign + magnitude, or `None`.
fn i64_text(negative: bool, text: &str) -> Option<i64> {
    let magnitude = i128::try_from(int_magnitude(text)?).ok()?;
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

/// Decodes a cooked string literal's token text (quotes included) to its
/// UTF-8 bytes.
fn unescape_str(text: &str) -> Vec<u8> {
    let body = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("rustc lexed the string literal");
    unescape(body, true)
}

/// Decodes a cooked byte-string literal's token text (`b"…"`) to its
/// bytes.
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
                     width's domain (1..=64 — docs/architecture/10-data-model.md)",
                    field.name
                )
            }),
        },
        FieldTy::Interval(element, width) => ValueType::Interval {
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
        let closed = relation.closed.is_some();
        if let Some(span) = relation.newtype_span {
            spans.newtypes.insert(rel_idx, span);
        }
        let declared = &relation.fields[usize::from(closed)..];
        let extension = relation.closed.as_ref().map(|extension| {
            extension
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
                .collect()
        });
        relations.push(RelationSpec {
            name: relation.name.as_str().into(),
            newtype: closed
                .then(|| relation.fields[0].newtype.as_deref())
                .flatten()
                .map(Into::into),
            fields: declared
                .iter()
                .map(|field| FieldSpec {
                    name: field.name.as_str().into(),
                    value_type: field_value_type(&relation.name, field),
                    newtype: field.newtype.as_deref().map(Into::into),
                    fresh: field.fresh,
                })
                .collect(),
            extension,
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
            Statement::Cardinality {
                source,
                window,
                window_span,
                target,
            } => {
                spans.windows.insert(index, *window_span);
                statements.push(StatementSpec::Cardinality {
                    target: lower_side(schema, index, StatementSide::Target, target, spans),
                    window: *window,
                    source: lower_side(schema, index, StatementSide::Source, source, spans),
                });
            }
        }
    }
    statements
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
        // `parse_extension` enforces exact column coverage, so an
        // over-wide row never reaches lowering from the macro — the
        // `SchemaSpec` bindings surface is this issue's only producer.
        SpecIssue::RowArityExcess { .. } => vec![Span::call_site()],
        SpecIssue::DuplicateHandleNewtype {
            second_relation, ..
        } => one(spans.newtypes.get(second_relation)),
        SpecIssue::WindowInverted { statement, .. }
        | SpecIssue::WindowExactRespelled { statement, .. }
        | SpecIssue::WindowExclusionRespelled { statement }
        | SpecIssue::WindowVacuous { statement }
        | SpecIssue::WindowContainmentRespelled { statement } => one(spans.windows.get(statement)),
        SpecIssue::DegenerateLiteralSet {
            statement,
            field,
            len,
        } => multi(spans.sets.get(&(*statement, field.to_string(), *len))),
    }
}

/// One issue's message — the macro's own dialect: the panic-era text,
/// verbatim, each naming the canonical form (the ban table's law). The
/// containment-respelled window composes the paste-back containment from
/// the spec's own statement.
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
        SpecIssue::DuplicateHandleNewtype {
            newtype,
            first,
            second,
            ..
        } => format!(
            "schema!: handle newtype `{newtype}` is declared by two closed relations \
             (`{first}` and `{second}`) — a handle newtype names exactly one closed relation"
        ),
        SpecIssue::WindowInverted { lo, hi, .. } => format!(
            "schema!: the window `{{{lo}..{hi}}}` is inverted — no count satisfies it; \
             bounds are `{{lo..hi}}` with lo < hi (an exact count is `{{n}}`)"
        ),
        SpecIssue::WindowExactRespelled { count, .. } => {
            format!("schema!: `{{{count}..{count}}}` — an exact count is written `{{{count}}}`")
        }
        SpecIssue::WindowExclusionRespelled { .. } => {
            "schema!: `{0..0}` — the exclusion is written `{0}`".to_owned()
        }
        SpecIssue::WindowVacuous { .. } => "schema!: the `{0..*}` window is vacuous — it \
             provably says nothing (`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`); \
             delete the statement"
            .to_owned(),
        SpecIssue::WindowContainmentRespelled { statement } => {
            let StatementSpec::Cardinality { target, source, .. } = &spec.statements[*statement]
            else {
                unreachable!("the containment-respelled window rides a cardinality statement");
            };
            format!(
                "schema!: `{{1..*}}` says only what the bare containment says — drop the \
                 annotation and write `{}(…) <= {}(…)`",
                target.relation, source.relation
            )
        }
        SpecIssue::DegenerateLiteralSet { field, len: 0, .. } => format!(
            "schema!: the literal set for `{field}` is empty — an empty set selects \
             nothing; write no binding"
        ),
        SpecIssue::DegenerateLiteralSet { field, .. } => format!(
            "schema!: the literal set for `{field}` has one element — a one-element \
             set is the bare literal: write `{field} == L`, no braces"
        ),
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
        ValueType::Interval { element, width } => {
            let width = match width {
                None => "::std::option::Option::None".to_owned(),
                Some(w) => format!("::std::option::Option::Some({w}u64)"),
            };
            format!(
                "{path}::Interval {{ element: ::bumbledb::schema::IntervalElement::{}, \
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
        Value::String(bytes) => {
            let text = std::str::from_utf8(bytes).expect("schema! string literals are UTF-8");
            format!(
                "{path}::String(::std::boxed::Box::from(\"{}\".as_bytes()))",
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
        Value::AllenMask(_) => unreachable!("schema! literals never carry an Allen mask"),
    }
}

/// Renders one binding's lowered literal set as a `LiteralSet`
/// expression.
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

/// Renders one lowered side as a `Side` expression.
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

/// Renders one lowered statement as its `StatementDescriptor` expression.
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
        StatementDescriptor::Cardinality {
            source,
            lo,
            hi,
            target,
        } => {
            let hi = match hi {
                None => "::std::option::Option::None".to_owned(),
                Some(hi) => format!("::std::option::Option::Some({hi}u64)"),
            };
            format!(
                "::bumbledb::schema::StatementDescriptor::Cardinality {{ \
                     source: {}, lo: {lo}u64, hi: {hi}, target: {} }},",
                side_tokens(source),
                side_tokens(target),
            )
        }
    }
}

/// Renders the LOWERED descriptor as const construction — ids already
/// minted by the shared lowering, nothing resolved here.
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

/// A declaration name as a `SCREAMING_SNAKE` constant name:
/// `SavingsTerms` → `SAVINGS_TERMS`, `rate_bps` → `RATE_BPS` — an
/// underscore lands before an uppercase letter that starts a new word
/// (after a lowercase/digit, or heading a lowercase run after an
/// uppercase run).
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

/// The declaration-order id constants on the theory
/// (docs/architecture/70-api.md § id constants — named data, not
/// ergonomics): per relation `Theory::BUSY: RelationId`, per field
/// `Theory::BUSY_PERSON: FieldId`), so the Rust host never
/// writes a magic number into an `ir::Query` — and the downstream
/// `query!` macro resolves names through ordinary rustc name resolution
/// (proc macros cannot see each other's output; paths to these constants
/// are how a typo'd relation becomes a compile error).
fn emit_id_constants(out: &mut String, schema: &SchemaAst) {
    // name → what it names; a collision (`Busy.person` vs a relation
    // `BusyPerson`, say) is diagnosed here with both claimants named,
    // not left to rustc's duplicate-item error.
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
    // name -> (inner Rust type, wraps an Interval). Intervals deliberately
    // carry no order (an encoding accident, not semantics — the `Interval`
    // doc), so their newtypes derive none either.
    let mut newtypes: BTreeMap<String, (String, bool)> = BTreeMap::new();
    for relation in relations {
        for field in &relation.fields {
            let Some(name) = &field.newtype else {
                continue;
            };
            // The bool marks order-free newtypes: intervals carry no
            // order (an encoding accident, not semantics) and bytes<N>
            // deliberately none either — a digest's lexicographic order
            // is an encoding artifact (the order-on-bytes refusal).
            let inner = match field.ty {
                FieldTy::U64 => ("u64".to_owned(), false),
                FieldTy::I64 => ("i64".to_owned(), false),
                FieldTy::FixedBytes(len) => (format!("[u8; {len}]"), true),
                FieldTy::Interval(element, _) => (
                    format!("::bumbledb::Interval<{}>", element_rust(element)),
                    true,
                ),
                _ => unreachable!("parser restricts `as` to u64/i64/bytes<N>/interval"),
            };
            if let Some(existing) = newtypes.get(name) {
                assert_eq!(
                    existing, &inner,
                    "schema!: newtype `{name}` declared twice with different inner types"
                );
                continue;
            }
            newtypes.insert(name.clone(), inner);
        }
    }
    for (name, (inner, order_free)) in newtypes {
        let order = if order_free { "" } else { ", PartialOrd, Ord" };
        let _ = write!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash{order})]\n\
             pub struct {name}(pub {inner});\n",
        );
    }
}

/// The per-closed-relation emission (`docs/architecture/70-api.md`): the
/// **host enum** — an emission, not a type. The engine's vocabulary is
/// relational; the macro projects it into a Rust enum so rustc's pattern
/// checking keeps working — one vocabulary, two checkers, zero drift: the
/// ids are the same declaration-order numbers on both sides, welded by
/// const `id`/`from_id` (explicit matches, no `as` casts — the mapping is
/// the declaration order stated, not a repr accident) and pinned by an
/// EMITTED weld test per closed relation, so the weld cannot be forgotten
/// for a new theory. The host enum is the constant namespace: no separate
/// per-handle constants exist.
fn emit_closed(out: &mut String, relations: &[Relation]) {
    for relation in relations {
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

/// A declaration name as a `snake_case` module name (`SavingsTerms` →
/// `savings_terms`) — the emitted weld-test module's.
fn snake(name: &str) -> String {
    screaming_snake(name).to_ascii_lowercase()
}

/// Whether the field is variable-width — borrowed in the generated struct
/// (`&'a str`), the reason its struct gains a lifetime. `bytes<N>` is
/// fixed-width and owned (`[u8; N]`, `Copy`) — no borrow surface.
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
        FieldTy::Interval(element, _) => {
            format!("::bumbledb::Interval<{}>", element_rust(*element))
        }
    }
}

/// The per-field encode expressions for the three `Fact` boundaries:
/// write (mints through the delta), delete (resolves pending-then-
/// committed, never mints — a miss proves the fact absent), read
/// (committed dictionary only). Word-backed fields encode identically in
/// every context; only the interned kinds (str/bytes) split by boundary.
fn encode_exprs(field: &Field, idx: usize) -> (String, String, String) {
    let access = if field.newtype.is_some() {
        format!("self.{}.0", field.name)
    } else {
        format!("self.{}", field.name)
    };
    let same = |expr: String| (expr.clone(), expr.clone(), expr);
    match &field.ty {
        FieldTy::Bool => same(format!("::bumbledb::__private::ValueRef::Bool({access})")),
        FieldTy::U64 => same(format!("::bumbledb::__private::ValueRef::U64({access})")),
        FieldTy::I64 => same(format!("::bumbledb::__private::ValueRef::I64({access})")),
        FieldTy::Interval(element, None) => same(format!(
            "::bumbledb::__private::ValueRef::Interval{}({access})",
            element_suffix(*element)
        )),
        // The fixed-width family: the host hands the same checked
        // `Interval<T>`; the boundary checks the declared width (a wide
        // or narrow value is a typed error — the width is the type) and
        // marks the one-word encoding.
        FieldTy::Interval(element, Some(width)) => same(format!(
            "::bumbledb::__private::fixed_interval_{}(\
             <Self as ::bumbledb::Fact<'a>>::RELATION, \
             ::bumbledb::schema::FieldId({idx}), {access}, {width}u64)?",
            element_rust(*element)
        )),
        // Inline in every context: bytes<N> never touches the dictionary,
        // so write/delete/read share one self-encoding expression.
        FieldTy::FixedBytes(_) => same(format!(
            "::bumbledb::__private::ValueRef::fixed_bytes(&{access})"
        )),
        FieldTy::Str => interned_exprs("str", "String", &field.name),
    }
}

/// The three boundary expressions for one interned field: `family`
/// selects the plumbing functions (`intern_{family}_{write,delete,read}`)
/// and `variant` the `ValueRef` constructor. Delete and read share the
/// miss shape — `Ok(false)`, the fact provably absent — differing only
/// in context binding. The field is already a borrow (`&'a str`), so it
/// passes straight through.
fn interned_exprs(family: &str, variant: &str, name: &str) -> (String, String, String) {
    let miss = |boundary: &str, ctx: &str| {
        format!(
            "match ::bumbledb::__private::intern_{family}_{boundary}({ctx}, self.{name})? {{ Some(id) => ::bumbledb::__private::ValueRef::{variant}(id), None => return Ok(false) }}"
        )
    };
    (
        format!(
            "::bumbledb::__private::ValueRef::{variant}(::bumbledb::__private::intern_{family}_write(tx, self.{name})?)"
        ),
        miss("delete", "tx"),
        miss("read", "snap"),
    )
}

/// The struct-literal arm decoding one field out of canonical fact bytes.
/// `ctx` is the decode context's binding (`snap`/`tx`) and `suffix` selects
/// the plumbing family (`""` = snapshot decode, `"_write"` = the write
/// transaction's pending-aware point-read decode).
fn decode_arm(field: &Field, idx: usize, ctx: &str, suffix: &str) -> String {
    let wrap = |expr: &str| -> String {
        match &field.newtype {
            Some(newtype) => format!("{newtype}({expr})"),
            None => expr.to_owned(),
        }
    };
    let decode =
        format!("::bumbledb::__private::decode{suffix}({ctx}, Self::RELATION, fact, {idx})?");
    // Every field decodes through one shape: destructure the
    // schema-typed `ValueRef` variant, convert; any other variant is a
    // programmer-invariant violation.
    let arm = |pattern: String, expr: String| {
        format!(
            "{}: match {decode} {{ ::bumbledb::__private::ValueRef::{pattern} => {expr}, _ => unreachable!(\"schema-typed\") }},",
            field.name
        )
    };
    match &field.ty {
        FieldTy::Bool => arm("Bool(v)".to_owned(), "v".to_owned()),
        FieldTy::U64 => arm("U64(v)".to_owned(), wrap("v")),
        FieldTy::I64 => arm("I64(v)".to_owned(), wrap("v")),
        FieldTy::Interval(element, None) => arm(
            format!("Interval{}(interval)", element_suffix(*element)),
            wrap("interval"),
        ),
        // A fixed-width field decodes through its own ValueRef variant —
        // the end was re-derived from the type's width at decode.
        FieldTy::Interval(element, Some(_)) => arm(
            format!("FixedInterval{}(interval)", element_suffix(*element)),
            wrap("interval"),
        ),
        FieldTy::Str => arm(
            "String(id)".to_owned(),
            format!("::bumbledb::__private::resolve_string{suffix}({ctx}, id)?"),
        ),
        FieldTy::FixedBytes(len) => arm(
            "FixedBytes(value)".to_owned(),
            wrap(&format!(
                "<[u8; {len}]>::try_from(value.as_bytes()).expect(\"schema-typed width\")"
            )),
        ),
    }
}

fn emit_fact_struct(out: &mut String, schema_name: &str, index: usize, relation: &Relation) {
    let name = &relation.name;
    // A struct with any variable-width field gains one lifetime: those
    // fields are borrowed (`&'a str` / `&'a [u8]`) — from the host at
    // insert, from the resolver at decode. All-fixed-width structs stay
    // lifetime-free and implement `Fact<'a>` for every `'a`.
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

    let mut write_values = String::new();
    let mut delete_values = String::new();
    let mut read_values = String::new();
    let mut decode_fields = String::new();
    let mut decode_write_fields = String::new();
    for (idx, field) in relation.fields.iter().enumerate() {
        let (write_expr, delete_expr, read_expr) = encode_exprs(field, idx);
        let _ = write!(write_values, "{write_expr},");
        let _ = write!(delete_values, "{delete_expr},");
        let _ = write!(read_values, "{read_expr},");
        let _ = write!(decode_fields, "{}", decode_arm(field, idx, "snap", ""));
        let _ = write!(
            decode_write_fields,
            "{}",
            decode_arm(field, idx, "tx", "_write")
        );
    }

    let _ = write!(
        out,
        "#[derive(Debug, Clone, PartialEq)]\n\
         pub struct {name}{struct_params} {{ {struct_fields} }}\n\
         impl<'a> ::bumbledb::Fact<'a> for {self_ty} {{\n\
             type Schema = {schema_name};\n\
             const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
             fn encode_write(&self, tx: &mut ::bumbledb::WriteTx<'_, {schema_name}>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<()> {{\n\
                 let values = [{write_values}];\n\
                 ::bumbledb::__private::encode_write_fact(tx, <Self as ::bumbledb::Fact<'a>>::RELATION, &values, out);\n\
                 Ok(())\n\
             }}\n\
             fn encode_delete(&self, tx: &::bumbledb::WriteTx<'_, {schema_name}>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<bool> {{\n\
                 let values = [{delete_values}];\n\
                 ::bumbledb::__private::encode_write_fact(tx, <Self as ::bumbledb::Fact<'a>>::RELATION, &values, out);\n\
                 Ok(true)\n\
             }}\n\
             fn encode_read(&self, snap: &::bumbledb::Snapshot<'_, {schema_name}>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<bool> {{\n\
                 let values = [{read_values}];\n\
                 ::bumbledb::__private::encode_read_fact(snap, <Self as ::bumbledb::Fact<'a>>::RELATION, &values, out);\n\
                 Ok(true)\n\
             }}\n\
             fn decode(snap: &'a ::bumbledb::Snapshot<'_, {schema_name}>, fact: &[u8]) -> ::bumbledb::Result<Self> {{\n\
                 Ok(Self {{ {decode_fields} }})\n\
             }}\n\
             fn decode_write(tx: &'a ::bumbledb::WriteTx<'_, {schema_name}>, fact: &[u8]) -> ::bumbledb::Result<Self> {{\n\
                 Ok(Self {{ {decode_write_fields} }})\n\
             }}\n\
         }}\n",
    );

    // Fresh-minting newtypes: `tx.alloc::<NewType>()` knows its field.
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
             }}\n",
        );
    }

    // Exactly one fresh field: the typed point-read key
    // (`WriteTx::get::<Fact>(id)`). Relations with zero or several fresh
    // fields have no single dominant key — those read through `get_dyn`
    // (the multi-key typed shape is an OPEN item, 70-api.md).
    let fresh_fields: Vec<&Field> = relation.fields.iter().filter(|f| f.fresh).collect();
    if let [fresh_field] = fresh_fields.as_slice() {
        let newtype = fresh_field
            .newtype
            .as_ref()
            .expect("parser demands `as NewType` on fresh fields");
        let _ = write!(
            out,
            "impl<'a> ::bumbledb::FreshKeyed<'a> for {self_ty} {{\n\
                 type FreshKey = {newtype};\n\
             }}\n",
        );
    }
}
