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
//! (docs/architecture/30-dependencies.md): `R(X) -> R` (functionality),
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
//! set is the bare literal — `{L}` and `{}` do not parse).
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
//! The macro validates only its own grammar plus name-to-id resolution
//! (both are compile errors at the call site): expansion emits
//! `SchemaDescriptor` construction directly, ids resolved at expansion
//! time from declaration order. Everything semantic beyond names surfaces
//! as the typed `SchemaError` from `Db::create`/`Db::open`, where the
//! descriptor is validated.

use proc_macro::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::iter::Peekable;

/// The element domain of an `interval<..>` field: closed to the two
/// orderable scalars, mirroring the engine's `IntervalElement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntervalElement {
    U64,
    I64,
}

impl IntervalElement {
    /// The Rust scalar type the interval ranges over.
    fn rust(self) -> &'static str {
        match self {
            Self::U64 => "u64",
            Self::I64 => "i64",
        }
    }

    /// The engine-side variant-name suffix (`IntervalU64` / `IntervalI64`).
    fn suffix(self) -> &'static str {
        match self {
            Self::U64 => "U64",
            Self::I64 => "I64",
        }
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
    /// descriptor emission skips it — `validate()` prepends its own.
    fields: Vec<Field>,
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
/// against the selected field's declaration at emission. Integer and
/// string/byte-string token text is spliced verbatim into the generated
/// `Value`, so rustc polices the value itself.
#[derive(Debug, Clone)]
enum Literal {
    Bool(bool),
    /// `[-] int`.
    Int {
        negative: bool,
        text: String,
    },
    /// A bare ident: a closed relation's handle, resolved to its
    /// declaration-order row id through the selected field's newtype.
    Handle(String),
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
/// spelling — `docs/architecture/30-dependencies.md`). By the
/// canonical-utterance law both degenerate sets are expansion panics
/// naming the canonical form: `{L}` (a one-element set is the bare
/// literal) and `{}` (an empty set selects nothing; write no binding).
#[derive(Debug, Clone)]
enum Literals {
    One(Literal),
    Many(Vec<Literal>),
}

/// One side of a dependency statement:
/// `R(fields [ | field == literal-or-set, .. ])`.
#[derive(Debug, Clone)]
struct Side {
    relation: String,
    projection: Vec<String>,
    selection: Vec<(String, Literals)>,
}

/// One parsed dependency statement. `==` never reaches here: it is lowered
/// at parse into two adjacent `Containment`s, `A <= B` first.
#[derive(Debug, Clone)]
enum Statement {
    Functionality {
        relation: String,
        projection: Vec<String>,
    },
    Containment {
        source: Side,
        target: Side,
    },
    /// `B(Y | ψ) <={lo..hi} A(X | σ);` — B-family, target-left: the
    /// LEFT side is the window's target (the per-group parent), the
    /// right side the counted source. `hi` is `None` for the `{lo..*}`
    /// floor spelling; the `{n}` exact spelling lands as `lo = hi = n`.
    Cardinality {
        source: Side,
        lo: u64,
        hi: Option<u64>,
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
    match tokens.next() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
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
    let newtype = expect_ident(tokens, "the handle newtype's name");
    let mut relation = if peek_brace(tokens) {
        let body = take_group(tokens, Delimiter::Brace, "a relation body");
        parse_relation(name, body)
    } else {
        Relation {
            name,
            fields: Vec::new(),
            closed: None,
        }
    };
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

/// Parses one `[-] int`, returning the sign and the raw token text (spliced
/// verbatim into the generated code — rustc polices range and form).
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
            let word = expect_ident(tokens, "a literal");
            match word.as_str() {
                "true" => Literal::Bool(true),
                "false" => Literal::Bool(false),
                _ => Literal::Handle(word),
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
/// single literal. The degenerate sets are expansion panics naming the
/// canonical form (the canonical-utterance law,
/// `docs/architecture/70-api.md`): `{L}` — a one-element set is the bare
/// literal `field == L` — and `{}` (an empty set selects nothing; write
/// no binding).
fn parse_literals(field: &str, tokens: &mut Tokens) -> Literals {
    if !peek_brace(tokens) {
        return Literals::One(parse_literal(tokens));
    }
    let body = take_group(tokens, Delimiter::Brace, "a literal set");
    let mut set_tokens = body.into_iter().peekable();
    let mut literals = Vec::new();
    while set_tokens.peek().is_some() {
        literals.push(parse_literal(&mut set_tokens));
        if peek_punct(&mut set_tokens, ',') {
            set_tokens.next();
        }
    }
    match literals.len() {
        0 => panic!(
            "schema!: the literal set for `{field}` is empty — an empty set selects \
             nothing; write no binding"
        ),
        1 => panic!(
            "schema!: the literal set for `{field}` has one element — a one-element \
             set is the bare literal: write `{field} == L`, no braces"
        ),
        _ => Literals::Many(literals),
    }
}

/// Parses `fields [ | field == literal-or-set, .. ]` out of one side's
/// parens.
fn parse_side(relation: String, group: TokenStream) -> Side {
    let mut tokens = group.into_iter().peekable();
    let mut projection = Vec::new();
    while tokens.peek().is_some() && !peek_punct(&mut tokens, '|') {
        projection.push(expect_ident(&mut tokens, "a field name"));
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    let mut selection = Vec::new();
    if peek_punct(&mut tokens, '|') {
        tokens.next();
        while tokens.peek().is_some() {
            let field = expect_ident(&mut tokens, "a selected field name");
            expect_punct(&mut tokens, '=');
            expect_punct(&mut tokens, '=');
            let literals = parse_literals(&field, &mut tokens);
            selection.push((field, literals));
            if peek_punct(&mut tokens, ',') {
                tokens.next();
            }
        }
    }
    Side {
        relation,
        projection,
        selection,
    }
}

/// Parses `Rel(...)` — the right-hand side of `<=` / `==`.
fn parse_statement_side(tokens: &mut Tokens) -> Side {
    let relation = expect_ident(tokens, "a relation name");
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    parse_side(relation, group)
}

/// Parses one dependency statement, `relation` being its left relation name
/// (already consumed). `==` lowers here to two adjacent `Containment`s,
/// `A <= B` first (docs/architecture/30-dependencies.md).
fn parse_statement(relation: String, tokens: &mut Tokens, statements: &mut Vec<Statement>) {
    let group = take_group(tokens, Delimiter::Parenthesis, "a projection list");
    let left = parse_side(relation, group);
    match tokens.next() {
        // `->`: functionality. The right side is the side's own relation,
        // and the FD form takes no selection — the engine descriptor
        // carries none by construction (the shape is unrepresentable, not
        // rejected downstream), so the grammar is the judge here.
        Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
            expect_punct(tokens, '>');
            let right = expect_ident(tokens, "the FD's relation name");
            assert!(
                left.selection.is_empty(),
                "schema!: an FD takes no selection — the FD form is `R(X) -> R` \
                 (docs/architecture/30-dependencies.md)"
            );
            assert_eq!(
                right, left.relation,
                "schema!: an FD's right side is its own relation: R(X) -> R"
            );
            statements.push(Statement::Functionality {
                relation: left.relation,
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
                let body = take_group(tokens, Delimiter::Brace, "the window bounds");
                let spelling = parse_window(body);
                let right = parse_statement_side(tokens);
                let (lo, hi) = admit_window(spelling, &left.relation, &right.relation);
                statements.push(Statement::Cardinality {
                    source: right,
                    lo,
                    hi,
                    target: left,
                });
            } else {
                let right = parse_statement_side(tokens);
                statements.push(Statement::Containment {
                    source: left,
                    target: right,
                });
            }
        }
        // `==`: set equality, lowered to the two containments.
        Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
            expect_punct(tokens, '=');
            let right = parse_statement_side(tokens);
            statements.push(Statement::Containment {
                source: left.clone(),
                target: right.clone(),
            });
            statements.push(Statement::Containment {
                source: right,
                target: left,
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
}

/// A window's bounds exactly as written — the spelling is judged by
/// [`admit_window`] before it lowers to `(lo, hi)`.
#[derive(Clone, Copy)]
enum WindowSpelling {
    /// `{n}` — THE exact-count spelling (`{0}` is the exclusion).
    Exact(u64),
    /// `{lo..hi}` — two explicit bounds.
    Range(u64, u64),
    /// `{lo..*}` — a floor, no ceiling.
    Floor(u64),
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

/// Parses the `<={…}` brace group into the spelling as written. The open
/// shorthands are never admitted — bounds are always explicit:
/// `{..hi}` and `{lo..}` are expansion panics naming the explicit form.
fn parse_window(body: TokenStream) -> WindowSpelling {
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
        return WindowSpelling::Exact(lo);
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
        WindowSpelling::Floor(lo)
    } else {
        WindowSpelling::Range(
            lo,
            parse_window_bound(&mut tokens, "the window's upper bound"),
        )
    };
    assert!(
        tokens.peek().is_none(),
        "schema!: trailing tokens after the window bounds"
    );
    spelling
}

/// The canonical-utterance law over the window vocabulary
/// (`docs/architecture/70-api.md`): every banned spelling is an expansion
/// panic NAMING the canonical form. Survivors — each otherwise
/// unrepresentable — lower to the descriptor's `(lo, hi)`: `{n}` exact
/// (`{0}` the exclusion), `{lo..hi}` with lo < hi, `{lo..*}` floors
/// (lo ≥ 2), `{0..hi}` ceilings.
fn admit_window(spelling: WindowSpelling, target: &str, source: &str) -> (u64, Option<u64>) {
    match spelling {
        WindowSpelling::Exact(n) => (n, Some(n)),
        WindowSpelling::Range(lo, hi) if hi < lo => panic!(
            "schema!: the window `{{{lo}..{hi}}}` is inverted — no count satisfies it; \
             bounds are `{{lo..hi}}` with lo < hi (an exact count is `{{n}}`)"
        ),
        WindowSpelling::Range(0, 0) => {
            panic!("schema!: `{{0..0}}` — the exclusion is written `{{0}}`")
        }
        WindowSpelling::Range(lo, hi) if lo == hi => {
            panic!("schema!: `{{{lo}..{hi}}}` — an exact count is written `{{{lo}}}`")
        }
        WindowSpelling::Range(lo, hi) => (lo, Some(hi)),
        WindowSpelling::Floor(0) => panic!(
            "schema!: the `{{0..*}}` window is vacuous — it provably says nothing \
             (`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`); delete the statement"
        ),
        WindowSpelling::Floor(1) => panic!(
            "schema!: `{{1..*}}` says only what the bare containment says — drop the \
             annotation and write `{target}(…) <= {source}(…)`"
        ),
        WindowSpelling::Floor(lo) => (lo, None),
    }
}

/// Parses the whole `schema!` body: the `pub Name;` header first, then
/// relation blocks and dependency statements in any order.
fn parse_schema(input: TokenStream) -> SchemaAst {
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
        let ident = expect_ident(&mut tokens, "`relation`, `closed relation`, or a statement");
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
            parse_statement(ident, &mut tokens, &mut schema.statements);
        }
    }
    schema
}

/// The declarative schema surface: expands to the header's `Theory`
/// unit struct, host-side newtypes and host enums, and one typed fact struct
/// per relation with `encode_write`/`encode_delete`/`encode_read`/`decode`
/// boundaries. The expansion constructs `SchemaDescriptor` directly — ids
/// resolved here from declaration order — and semantic validation runs
/// where the definition is consumed (`Db::create`/`Db::open`, as the
/// typed `SchemaError`).
///
/// # Panics
///
/// On malformed `schema!` grammar or an unresolvable relation/field/handle
/// name — a compile error at the macro call site, reported with the
/// offending token or name.
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let schema = parse_schema(input);
    let closed = closed_map(&schema.relations);
    let mut out = String::new();
    emit_schema_def(&mut out, &schema, &closed);
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

/// The handle namespace: newtype name → its owning closed relation. A
/// handle literal in a selection or row resolves through the referenced
/// field's newtype, so each handle newtype must name exactly one closed
/// relation — two claimants panic with both named.
fn closed_map(relations: &[Relation]) -> BTreeMap<&str, &Relation> {
    let mut map: BTreeMap<&str, &Relation> = BTreeMap::new();
    for relation in relations {
        if relation.closed.is_none() {
            continue;
        }
        let newtype = relation.fields[0]
            .newtype
            .as_deref()
            .expect("closed relations carry the handle newtype");
        if let Some(existing) = map.insert(newtype, relation) {
            panic!(
                "schema!: handle newtype `{newtype}` is declared by two closed relations \
                 (`{}` and `{}`) — a handle newtype names exactly one closed relation",
                existing.name, relation.name
            );
        }
    }
    map
}

/// Resolves a statement-named relation to its declaration index — the
/// `RelationId`, by the declaration-order rule.
fn relation_index(relations: &[Relation], name: &str) -> usize {
    relations
        .iter()
        .position(|r| r.name == name)
        .unwrap_or_else(|| panic!("schema!: relation `{name}` is not declared in this invocation"))
}

/// Resolves a statement-named field to its declaration index within its
/// relation — the `FieldId`.
fn field_index(declaration: &Relation, field: &str) -> usize {
    declaration
        .fields
        .iter()
        .position(|f| f.name == field)
        .unwrap_or_else(|| {
            panic!(
                "schema!: relation `{}` has no field `{field}`",
                declaration.name
            )
        })
}

/// Renders one field's structural type as a `ValueType` expression.
fn value_type_expr(ty: &FieldTy) -> String {
    let value_type = "::bumbledb::schema::ValueType";
    match ty {
        FieldTy::Bool => format!("{value_type}::Bool"),
        FieldTy::U64 => format!("{value_type}::U64"),
        FieldTy::I64 => format!("{value_type}::I64"),
        FieldTy::Str => format!("{value_type}::String"),
        FieldTy::FixedBytes(len) => format!("{value_type}::FixedBytes {{ len: {len} }}"),
        FieldTy::Interval(element, width) => {
            let width = match width {
                None => "::std::option::Option::None".to_owned(),
                Some(w) => format!("::std::option::Option::Some({w}u64)"),
            };
            format!(
                "{value_type}::Interval {{ element: ::bumbledb::schema::IntervalElement::{},                  width: {width} }}",
                element.suffix()
            )
        }
    }
}

/// Renders one signed integer bound.
fn signed(bound: &(bool, String)) -> String {
    let (negative, text) = bound;
    if *negative {
        format!("-{text}")
    } else {
        text.clone()
    }
}

/// Renders one literal — a statement selection's or a closed row's — as a
/// shared `Value` expression, typed against the field's declaration: one
/// machine, same errors, both call sites. A bare handle resolves through
/// the field's newtype to its owning closed relation's declaration-order
/// row id. Integer and string/byte-string
/// token text is spliced verbatim, so rustc polices the value itself.
fn value_expr(
    closed: &BTreeMap<&str, &Relation>,
    declaration: &Relation,
    field: &str,
    literal: &Literal,
) -> String {
    let relation = &declaration.name;
    let field_decl = &declaration.fields[field_index(declaration, field)];
    let ty = &field_decl.ty;
    let value = "::bumbledb::Value";
    match (ty, literal) {
        (FieldTy::Bool, Literal::Bool(v)) => format!("{value}::Bool({v})"),
        (
            FieldTy::U64,
            Literal::Int {
                negative: false,
                text,
            },
        ) => format!("{value}::U64({text})"),
        (FieldTy::I64, Literal::Int { negative, text }) => {
            format!("{value}::I64({})", signed(&(*negative, text.clone())))
        }
        // A bare handle: legal on a field whose newtype is a closed
        // relation's handle newtype — the handle namespace is
        // per-closed-relation, resolved through the reference. It compiles
        // to the row's declaration-order id.
        (_, Literal::Handle(name)) => {
            let owner = field_decl
                .newtype
                .as_deref()
                .and_then(|newtype| closed.get(newtype));
            let Some(owner) = owner else {
                panic!(
                    "schema!: `{relation}.{field}` is not a closed-relation reference — \
                     the handle literal `{name}` is legal only on a field whose newtype \
                     is a closed relation's handle newtype"
                );
            };
            let rows = &owner
                .closed
                .as_ref()
                .expect("closed-map entries are closed")
                .rows;
            let id = rows
                .iter()
                .position(|row| row.handle == *name)
                .unwrap_or_else(|| {
                    panic!(
                        "schema!: closed relation `{}` has no handle `{name}`",
                        owner.name
                    )
                });
            format!("{value}::U64({id})")
        }
        (FieldTy::Str, Literal::Str(text)) => {
            format!("{value}::String(::std::boxed::Box::from({text}.as_bytes()))")
        }
        (FieldTy::FixedBytes(_), Literal::Bytes(text)) => {
            format!("{value}::FixedBytes(::std::boxed::Box::from(&{text}[..]))")
        }
        (
            FieldTy::Interval(IntervalElement::U64, _),
            Literal::Interval {
                start: (false, start),
                end: (false, end),
            },
        ) => format!(
            "{value}::IntervalU64(::bumbledb::Interval::<u64>::new({start}, {end}).expect(\"schema! interval literals are nonempty\"))"
        ),
        (FieldTy::Interval(IntervalElement::I64, _), Literal::Interval { start, end }) => {
            format!(
                "{value}::IntervalI64(::bumbledb::Interval::<i64>::new({}, {}).expect(\"schema! interval literals are nonempty\"))",
                signed(start),
                signed(end)
            )
        }
        _ => panic!(
            "schema!: the literal for `{relation}.{field}` does not fit \
             the field's declared type"
        ),
    }
}

/// Renders `FieldId(i), ..` for a statement-named field list.
fn field_id_list(declaration: &Relation, fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| {
            format!(
                "::bumbledb::schema::FieldId({})",
                field_index(declaration, f)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Renders one binding's literal set as a `LiteralSet` expression — the
/// singleton spelling stays the `One` arm by construction.
fn literals_expr(
    closed: &BTreeMap<&str, &Relation>,
    declaration: &Relation,
    field: &str,
    literals: &Literals,
) -> String {
    match literals {
        Literals::One(literal) => format!(
            "::bumbledb::schema::LiteralSet::One({})",
            value_expr(closed, declaration, field, literal)
        ),
        Literals::Many(values) => {
            let mut rendered = String::new();
            for literal in values {
                let _ = write!(
                    rendered,
                    "{},",
                    value_expr(closed, declaration, field, literal)
                );
            }
            format!("::bumbledb::schema::LiteralSet::Many(::std::boxed::Box::new([{rendered}]))")
        }
    }
}

/// Renders one side as a `Side` expression, ids pre-resolved.
fn side_expr(relations: &[Relation], closed: &BTreeMap<&str, &Relation>, side: &Side) -> String {
    let relation = relation_index(relations, &side.relation);
    let declaration = &relations[relation];
    let mut selection = String::new();
    for (field, literals) in &side.selection {
        let _ = write!(
            selection,
            "(::bumbledb::schema::FieldId({}), {}),",
            field_index(declaration, field),
            literals_expr(closed, declaration, field, literals)
        );
    }
    format!(
        "::bumbledb::schema::Side {{ \
             relation: ::bumbledb::schema::RelationId({relation}), \
             projection: ::std::boxed::Box::new([{}]), \
             selection: ::std::boxed::Box::new([{selection}]) }}",
        field_id_list(declaration, &side.projection),
    )
}

/// Renders one parsed statement as its `StatementDescriptor` expression,
/// ids resolved from declaration order.
fn statement_expr(
    schema: &SchemaAst,
    closed: &BTreeMap<&str, &Relation>,
    statement: &Statement,
) -> String {
    match statement {
        Statement::Functionality {
            relation,
            projection,
        } => {
            let index = relation_index(&schema.relations, relation);
            format!(
                "::bumbledb::schema::StatementDescriptor::Functionality {{ \
                     relation: ::bumbledb::schema::RelationId({index}), \
                     projection: ::std::boxed::Box::new([{}]) }},",
                field_id_list(&schema.relations[index], projection),
            )
        }
        Statement::Containment { source, target } => format!(
            "::bumbledb::schema::StatementDescriptor::Containment {{ source: {}, target: {} }},",
            side_expr(&schema.relations, closed, source),
            side_expr(&schema.relations, closed, target),
        ),
        Statement::Cardinality {
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
                side_expr(&schema.relations, closed, source),
                side_expr(&schema.relations, closed, target),
            )
        }
    }
}

fn emit_schema_def(out: &mut String, schema: &SchemaAst, closed: &BTreeMap<&str, &Relation>) {
    let mut relations = String::new();
    for relation in &schema.relations {
        let mut fields = String::new();
        // The descriptor carries declared columns only: the AST's
        // synthetic (`id`, U64) field at index 0 of a closed relation is
        // `validate()`'s to prepend — emitting it too would collide.
        let declared = &relation.fields[usize::from(relation.closed.is_some())..];
        for field in declared {
            let _ = write!(
                fields,
                "::bumbledb::schema::FieldDescriptor {{ \
                     name: ::std::boxed::Box::from(\"{}\"), \
                     value_type: {}, \
                     generation: ::bumbledb::schema::Generation::{} }},",
                field.name,
                value_type_expr(&field.ty),
                if field.fresh { "Fresh" } else { "None" },
            );
        }
        // The extension: ground axioms as `Row` values in declaration
        // order, literals through the same `value_expr` machine as
        // statement selections.
        let extension = match &relation.closed {
            None => "::std::option::Option::None".to_owned(),
            Some(extension) => {
                let mut rows = String::new();
                for row in &extension.rows {
                    let mut values = String::new();
                    for (field, literal) in &row.values {
                        let _ = write!(values, "{},", value_expr(closed, relation, field, literal));
                    }
                    let _ = write!(
                        rows,
                        "::bumbledb::schema::Row {{ \
                             handle: ::std::boxed::Box::from(\"{}\"), \
                             values: ::std::boxed::Box::new([{values}]) }},",
                        row.handle,
                    );
                }
                format!("::std::option::Option::Some(::std::boxed::Box::new([{rows}]))")
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
    for statement in &schema.statements {
        let _ = write!(statements, "{}", statement_expr(schema, closed, statement));
    }
    let name = &schema.name;
    let _ = write!(
        out,
        "/// The `{name}` schema definition: the value `Db::create`/`Db::open` \
         take and the typestate `Db<{name}>` carries. Validation runs at \
         open, surfacing declaration errors as the typed `SchemaError`.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub struct {name};\n\
         impl ::bumbledb::Theory for {name} {{\n\
             fn descriptor(self) -> ::bumbledb::schema::SchemaDescriptor {{\n\
                 ::bumbledb::schema::SchemaDescriptor {{\n\
                     relations: ::std::vec![{relations}],\n\
                     statements: ::std::vec![{statements}],\n\
                 }}\n\
             }}\n\
         }}\n",
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
                FieldTy::Interval(element, _) => {
                    (format!("::bumbledb::Interval<{}>", element.rust()), true)
                }
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
        FieldTy::Interval(element, _) => format!("::bumbledb::Interval<{}>", element.rust()),
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
            element.suffix()
        )),
        // The fixed-width family: the host hands the same checked
        // `Interval<T>`; the boundary checks the declared width (a wide
        // or narrow value is a typed error — the width is the type) and
        // marks the one-word encoding.
        FieldTy::Interval(element, Some(width)) => same(format!(
            "::bumbledb::__private::fixed_interval_{}(\
             <Self as ::bumbledb::Fact<'a>>::RELATION, \
             ::bumbledb::schema::FieldId({idx}), {access}, {width}u64)?",
            element.rust()
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
            format!("Interval{}(interval)", element.suffix()),
            wrap("interval"),
        ),
        // A fixed-width field decodes through its own ValueRef variant —
        // the end was re-derived from the type's width at decode.
        FieldTy::Interval(element, Some(_)) => arm(
            format!("FixedInterval{}(interval)", element.suffix()),
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
