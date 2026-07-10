//! The `schema!` proc-macro (docs/architecture/70-api.md): bumbledb's declarative schema
//! surface. A small, rigid grammar — this is Rust-side declaration, not a
//! query language — hand-parsed over the raw token stream (no `syn`, no
//! `quote`: the grammar is not Rust syntax and the dependency would buy
//! nothing).
//!
//! ```text
//! schema! {
//!     relation Holder  { id: u64 as HolderId, serial, name: str }
//!     relation Account {
//!         id:     u64 as AccountId, serial,
//!         holder: u64 as HolderId,
//!         kind:   enum Kind { Checking, Savings },
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
//! Types: `bool`, `u64`, `i64`, `str`, `bytes`, inline `enum Name { .. }`
//! (the name names the generated Rust enum only — engine identity is the
//! structural variant list), `interval<i64>`, `interval<u64>`. `as NewType`
//! generates the host-side nominal newtype (legal on u64, i64, and both
//! intervals). `serial` auto-materializes `R(field) -> R` at schema
//! resolution. **There are no field-level constraint modifiers** — everything
//! relational is a dependency statement between the relation blocks
//! (docs/architecture/30-dependencies.md): `R(X) -> R` (functionality),
//! `A(X | σ) <= B(Y | ψ)` (containment), `==` lowered here to the two
//! adjacent containments, `A <= B` first. Selection literals are typed
//! against the selected field in the macro (every enum's variant list is in
//! the same invocation, so variant names resolve to ordinals here); interval
//! literals are written `start..end`, half-open.
//!
//! The macro validates only its own grammar plus name-to-id resolution
//! (both are compile errors at the call site): expansion emits
//! `SchemaDescriptor` construction directly, ids resolved at expansion
//! time from declaration order. Everything semantic beyond names surfaces
//! at the first `schema()` call (memoized in a `OnceLock`) as a panic
//! carrying the typed `SchemaError`'s rendering.

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
    Bytes,
    Enum { name: String, variants: Vec<String> },
    Interval(IntervalElement),
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: FieldTy,
    newtype: Option<String>,
    serial: bool,
}

#[derive(Debug, Clone)]
struct Relation {
    name: String,
    fields: Vec<Field>,
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
    /// A bare ident: an enum variant name, resolved to its ordinal here.
    Variant(String),
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

/// One side of a dependency statement: `R(fields [ | field == literal, .. ])`.
#[derive(Debug, Clone)]
struct Side {
    relation: String,
    projection: Vec<String>,
    selection: Vec<(String, Literal)>,
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
}

/// The whole parsed invocation: relation blocks plus dependency statements,
/// each list in source order.
struct SchemaAst {
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
}

/// Parses a comma-separated identifier list.
fn ident_list(stream: TokenStream) -> Vec<String> {
    let mut names = Vec::new();
    let mut tokens = stream.into_iter().peekable();
    while tokens.peek().is_some() {
        names.push(expect_ident(&mut tokens, "a name"));
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    names
}

/// Parses one relation body: fields only — everything relational is a
/// statement outside the block.
fn parse_relation(name: String, body: TokenStream) -> Relation {
    let mut relation = Relation {
        name,
        fields: Vec::new(),
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

/// Parses a field's type, optional `as NewType`, and optional `, serial`.
fn parse_field(name: String, tokens: &mut Tokens) -> Field {
    let ty_name = expect_ident(tokens, "a type (bool/u64/i64/str/bytes/enum/interval)");
    reject_deleted_word(&ty_name);
    let ty = match ty_name.as_str() {
        "bool" => FieldTy::Bool,
        "u64" => FieldTy::U64,
        "i64" => FieldTy::I64,
        "str" => FieldTy::Str,
        "bytes" => FieldTy::Bytes,
        "enum" => {
            let enum_name = expect_ident(tokens, "an enum type name");
            let body = take_group(tokens, Delimiter::Brace, "an enum variant list");
            FieldTy::Enum {
                name: enum_name,
                variants: ident_list(body),
            }
        }
        "interval" => {
            expect_punct(tokens, '<');
            let element = match expect_ident(tokens, "an interval element (i64/u64)").as_str() {
                "u64" => IntervalElement::U64,
                "i64" => IntervalElement::I64,
                other => panic!("schema!: interval element must be i64 or u64, found `{other}`"),
            };
            expect_punct(tokens, '>');
            FieldTy::Interval(element)
        }
        other => panic!("schema!: unknown type `{other}`"),
    };
    let mut field = Field {
        name,
        ty,
        newtype: None,
        serial: false,
    };
    if peek_ident(tokens).as_deref() == Some("as") {
        tokens.next();
        assert!(
            matches!(field.ty, FieldTy::U64 | FieldTy::I64 | FieldTy::Interval(_)),
            "schema!: `as NewType` applies to u64/i64/interval fields only"
        );
        field.newtype = Some(expect_ident(tokens, "a newtype name"));
    }
    // Trailing modifier: `, serial` — distinguished from the next field
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
                    word, "serial",
                    "schema!: unknown field modifier `{word}` (the only modifier is `serial`)"
                );
                assert!(
                    field.newtype.is_some(),
                    "schema!: serial field `{}` needs `as NewType` — without it \
                     there is no typed alloc path (use the descriptor API for a \
                     raw-u64 serial)",
                    field.name
                );
                field.serial = true;
                tokens.next(); // the comma
                tokens.next(); // `serial`
            }
        }
    }
    field
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
/// enum-variant ident, a string/byte-string literal, or `start..end`.
fn parse_literal(tokens: &mut Tokens) -> Literal {
    match tokens.peek() {
        Some(TokenTree::Ident(_)) => {
            let word = expect_ident(tokens, "a literal");
            match word.as_str() {
                "true" => Literal::Bool(true),
                "false" => Literal::Bool(false),
                _ => Literal::Variant(word),
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

/// Parses `fields [ | field == literal, .. ]` out of one side's parens.
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
            selection.push((field, parse_literal(&mut tokens)));
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
        // `<=`: containment.
        Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
            expect_punct(tokens, '=');
            let right = parse_statement_side(tokens);
            statements.push(Statement::Containment {
                source: left,
                target: right,
            });
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
        other => panic!("schema!: expected `->`, `<=`, or `==`, found {other:?}"),
    }
    expect_punct(tokens, ';');
}

/// Parses the whole `schema!` body: relation blocks and dependency
/// statements, in any order.
fn parse_schema(input: TokenStream) -> SchemaAst {
    let mut schema = SchemaAst {
        relations: Vec::new(),
        statements: Vec::new(),
    };
    let mut tokens = input.into_iter().peekable();
    while tokens.peek().is_some() {
        let ident = expect_ident(&mut tokens, "`relation` or a statement");
        if ident == "relation" {
            let name = expect_ident(&mut tokens, "a relation name");
            let body = take_group(&mut tokens, Delimiter::Brace, "a relation body");
            schema.relations.push(parse_relation(name, body));
        } else {
            parse_statement(ident, &mut tokens, &mut schema.statements);
        }
    }
    schema
}

/// The declarative schema surface: expands to `fn schema()`, host-side
/// newtypes and enums, and one typed fact struct per relation with
/// `encode_write`/`encode_delete`/`encode_read`/`decode` boundaries. The
/// expansion constructs `SchemaDescriptor` directly — ids resolved here
/// from declaration order — and semantic validation runs at the first
/// `schema()` call.
///
/// # Panics
///
/// On malformed `schema!` grammar or an unresolvable relation/field/variant
/// name — a compile error at the macro call site, reported with the
/// offending token or name.
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let schema = parse_schema(input);
    let mut out = String::new();
    emit_schema_fn(&mut out, &schema);
    emit_newtypes(&mut out, &schema.relations);
    emit_enums(&mut out, &schema.relations);
    for (index, relation) in schema.relations.iter().enumerate() {
        emit_fact_struct(&mut out, index, relation);
    }
    out.parse().expect("schema!: generated code parses")
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
        FieldTy::Bytes => format!("{value_type}::Bytes"),
        FieldTy::Enum { variants, .. } => {
            let list = variants
                .iter()
                .map(|v| format!("::std::boxed::Box::from(\"{v}\")"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{value_type}::Enum {{ variants: ::std::boxed::Box::new([{list}]) }}")
        }
        FieldTy::Interval(element) => format!(
            "{value_type}::Interval {{ element: ::bumbledb::schema::IntervalElement::{} }}",
            element.suffix()
        ),
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

/// Renders one selection literal as a shared `Value` expression, typed
/// against the selected field's declaration (enum variants resolve to
/// ordinals here — every variant list is in the same invocation). Integer
/// and string/byte-string token text is spliced verbatim, so rustc polices
/// the value itself.
fn value_expr(declaration: &Relation, field: &str, literal: &Literal) -> String {
    let relation = &declaration.name;
    let ty = &declaration.fields[field_index(declaration, field)].ty;
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
        (FieldTy::Enum { variants, .. }, Literal::Variant(name)) => {
            let ordinal = variants.iter().position(|v| v == name).unwrap_or_else(|| {
                panic!("schema!: enum field `{relation}.{field}` has no variant `{name}`")
            });
            let ordinal = u8::try_from(ordinal).expect("variant count fits u8");
            format!("{value}::Enum({ordinal})")
        }
        (FieldTy::Str, Literal::Str(text)) => {
            format!("{value}::String(::std::boxed::Box::from({text}.as_bytes()))")
        }
        (FieldTy::Bytes, Literal::Bytes(text)) => {
            format!("{value}::Bytes(::std::boxed::Box::from(&{text}[..]))")
        }
        (
            FieldTy::Interval(IntervalElement::U64),
            Literal::Interval {
                start: (false, start),
                end: (false, end),
            },
        ) => format!("{value}::IntervalU64({start}, {end})"),
        (FieldTy::Interval(IntervalElement::I64), Literal::Interval { start, end }) => {
            format!("{value}::IntervalI64({}, {})", signed(start), signed(end))
        }
        _ => panic!(
            "schema!: selection literal for `{relation}.{field}` does not fit \
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

/// Renders one side as a `Side` expression, ids pre-resolved.
fn side_expr(relations: &[Relation], side: &Side) -> String {
    let relation = relation_index(relations, &side.relation);
    let declaration = &relations[relation];
    let mut selection = String::new();
    for (field, literal) in &side.selection {
        let _ = write!(
            selection,
            "(::bumbledb::schema::FieldId({}), {}),",
            field_index(declaration, field),
            value_expr(declaration, field, literal)
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

fn emit_schema_fn(out: &mut String, schema: &SchemaAst) {
    let mut relations = String::new();
    for relation in &schema.relations {
        let mut fields = String::new();
        for field in &relation.fields {
            let _ = write!(
                fields,
                "::bumbledb::schema::FieldDescriptor {{ \
                     name: ::std::boxed::Box::from(\"{}\"), \
                     value_type: {}, \
                     generation: ::bumbledb::schema::Generation::{} }},",
                field.name,
                value_type_expr(&field.ty),
                if field.serial { "Serial" } else { "None" },
            );
        }
        let _ = write!(
            relations,
            "::bumbledb::schema::RelationDescriptor {{ \
                 name: ::std::boxed::Box::from(\"{}\"), \
                 fields: ::std::vec![{fields}] }},",
            relation.name,
        );
    }
    let mut statements = String::new();
    for statement in &schema.statements {
        match statement {
            Statement::Functionality {
                relation,
                projection,
            } => {
                let index = relation_index(&schema.relations, relation);
                let _ = write!(
                    statements,
                    "::bumbledb::schema::StatementDescriptor::Functionality {{ \
                         relation: ::bumbledb::schema::RelationId({index}), \
                         projection: ::std::boxed::Box::new([{}]) }},",
                    field_id_list(&schema.relations[index], projection),
                );
            }
            Statement::Containment { source, target } => {
                let _ = write!(
                    statements,
                    "::bumbledb::schema::StatementDescriptor::Containment {{ source: {}, target: {} }},",
                    side_expr(&schema.relations, source),
                    side_expr(&schema.relations, target),
                );
            }
        }
    }
    let _ = write!(
        out,
        "/// The compiled schema (memoized; declaration errors surface as \
         the typed `SchemaError` at the first call).\n\
         pub fn schema() -> &'static ::bumbledb::schema::Schema {{\n\
             static SCHEMA: ::std::sync::OnceLock<::bumbledb::schema::Schema> = ::std::sync::OnceLock::new();\n\
             SCHEMA.get_or_init(|| {{\n\
                 ::bumbledb::schema::SchemaDescriptor {{\n\
                     relations: ::std::vec![{relations}],\n\
                     statements: ::std::vec![{statements}],\n\
                 }}\n\
                 .validate()\n\
                 .unwrap_or_else(|e| panic!(\"schema! declaration is invalid: {{e}}\"))\n\
             }})\n\
         }}\n",
    );
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
            let inner = match field.ty {
                FieldTy::U64 => ("u64".to_owned(), false),
                FieldTy::I64 => ("i64".to_owned(), false),
                FieldTy::Interval(element) => {
                    (format!("::bumbledb::Interval<{}>", element.rust()), true)
                }
                _ => unreachable!("parser restricts `as` to u64/i64/interval"),
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
    for (name, (inner, wraps_interval)) in newtypes {
        let order = if wraps_interval {
            ""
        } else {
            ", PartialOrd, Ord"
        };
        let _ = write!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash{order})]\n\
             pub struct {name}(pub {inner});\n",
        );
    }
}

fn emit_enums(out: &mut String, relations: &[Relation]) {
    let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for relation in relations {
        for field in &relation.fields {
            if let FieldTy::Enum { name, variants } = &field.ty {
                if let Some(existing) = seen.get(name) {
                    assert_eq!(
                        existing, variants,
                        "schema!: enum `{name}` declared twice with different variants"
                    );
                    continue;
                }
                seen.insert(name.clone(), variants.clone());
            }
        }
    }
    for (name, variants) in seen {
        let list = variants.join(", ");
        let mut from_arms = String::new();
        for (ordinal, variant) in variants.iter().enumerate() {
            let _ = write!(from_arms, "{ordinal} => Some(Self::{variant}),");
        }
        let _ = write!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
             pub enum {name} {{ {list} }}\n\
             impl {name} {{\n\
                 #[must_use] pub fn ordinal(self) -> u8 {{ self as u8 }}\n\
                 #[must_use] pub fn from_ordinal(ordinal: u8) -> Option<Self> {{\n\
                     match ordinal {{ {from_arms} _ => None }}\n\
                 }}\n\
             }}\n",
        );
    }
}

fn rust_field_ty(field: &Field) -> String {
    if let Some(newtype) = &field.newtype {
        return newtype.clone();
    }
    match &field.ty {
        FieldTy::Bool => "bool".to_owned(),
        FieldTy::U64 => "u64".to_owned(),
        FieldTy::I64 => "i64".to_owned(),
        FieldTy::Str => "String".to_owned(),
        FieldTy::Bytes => "Vec<u8>".to_owned(),
        FieldTy::Enum { name, .. } => name.clone(),
        FieldTy::Interval(element) => format!("::bumbledb::Interval<{}>", element.rust()),
    }
}

/// The per-field encode expressions for the three `Fact` boundaries:
/// write (mints through the delta), delete (resolves pending-then-
/// committed, never mints — a miss proves the fact absent), read
/// (committed dictionary only). Word-backed fields encode identically in
/// every context; only the interned kinds (str/bytes) split by boundary.
fn encode_exprs(field: &Field) -> (String, String, String) {
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
        FieldTy::Enum { .. } => same(format!(
            "::bumbledb::__private::ValueRef::Enum(self.{}.ordinal())",
            field.name
        )),
        FieldTy::Interval(element) => same(format!(
            "::bumbledb::__private::ValueRef::Interval{}({access}.start(), {access}.end())",
            element.suffix()
        )),
        FieldTy::Str => interned_exprs("str", "String", &field.name),
        FieldTy::Bytes => interned_exprs("bytes", "Bytes", &field.name),
    }
}

/// The three boundary expressions for one interned field: `family`
/// selects the plumbing functions (`intern_{family}_{write,delete,read}`)
/// and `variant` the `ValueRef` constructor. Delete and read share the
/// miss shape — `Ok(false)`, the fact provably absent — differing only
/// in context binding.
fn interned_exprs(family: &str, variant: &str, name: &str) -> (String, String, String) {
    let miss = |boundary: &str, ctx: &str| {
        format!(
            "match ::bumbledb::__private::intern_{family}_{boundary}({ctx}, &self.{name})? {{ Some(id) => ::bumbledb::__private::ValueRef::{variant}(id), None => return Ok(false) }}"
        )
    };
    (
        format!(
            "::bumbledb::__private::ValueRef::{variant}(::bumbledb::__private::intern_{family}_write(tx, &self.{name})?)"
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
        FieldTy::Enum {
            name: enum_name, ..
        } => arm(
            "Enum(o)".to_owned(),
            format!(
                "{enum_name}::from_ordinal(o).expect(\"decode_field range-checked the ordinal\")"
            ),
        ),
        FieldTy::Interval(element) => {
            let el = element.rust();
            arm(
                format!("Interval{}(start, end)", element.suffix()),
                wrap(&format!(
                    "::bumbledb::Interval::<{el}>::new(start, end).expect(\"stored intervals satisfy start < end\")"
                )),
            )
        }
        FieldTy::Str => arm(
            "String(id)".to_owned(),
            format!("::bumbledb::__private::resolve_string{suffix}({ctx}, id)?"),
        ),
        FieldTy::Bytes => arm(
            "Bytes(id)".to_owned(),
            format!("::bumbledb::__private::resolve_bytes{suffix}({ctx}, id)?"),
        ),
    }
}

fn emit_fact_struct(out: &mut String, index: usize, relation: &Relation) {
    let name = &relation.name;
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
        let (write_expr, delete_expr, read_expr) = encode_exprs(field);
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
         pub struct {name} {{ {struct_fields} }}\n\
         impl ::bumbledb::Fact for {name} {{\n\
             const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
             fn encode_write(&self, tx: &mut ::bumbledb::WriteTx<'_>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<()> {{\n\
                 let values = [{write_values}];\n\
                 ::bumbledb::__private::encode_write_fact(tx, Self::RELATION, &values, out);\n\
                 Ok(())\n\
             }}\n\
             fn encode_delete(&self, tx: &::bumbledb::WriteTx<'_>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<bool> {{\n\
                 let values = [{delete_values}];\n\
                 ::bumbledb::__private::encode_write_fact(tx, Self::RELATION, &values, out);\n\
                 Ok(true)\n\
             }}\n\
             fn encode_read(&self, snap: &::bumbledb::Snapshot<'_>, out: &mut ::std::vec::Vec<u8>) -> ::bumbledb::Result<bool> {{\n\
                 let values = [{read_values}];\n\
                 ::bumbledb::__private::encode_read_fact(snap, Self::RELATION, &values, out);\n\
                 Ok(true)\n\
             }}\n\
             fn decode(snap: &::bumbledb::Snapshot<'_>, fact: &[u8]) -> ::bumbledb::Result<Self> {{\n\
                 Ok(Self {{ {decode_fields} }})\n\
             }}\n\
             fn decode_write(tx: &::bumbledb::WriteTx<'_>, fact: &[u8]) -> ::bumbledb::Result<Self> {{\n\
                 Ok(Self {{ {decode_write_fields} }})\n\
             }}\n\
         }}\n",
    );

    // Serial-minting newtypes: `tx.alloc::<NewType>()` knows its field.
    for (field_idx, field) in relation.fields.iter().enumerate() {
        let (true, Some(newtype)) = (field.serial, &field.newtype) else {
            continue;
        };
        let _ = write!(
            out,
            "impl ::bumbledb::Serial for {newtype} {{\n\
                 const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
                 const FIELD: ::bumbledb::schema::FieldId = ::bumbledb::schema::FieldId({field_idx});\n\
                 fn from_serial(raw: u64) -> Self {{ Self(raw) }}\n\
                 fn serial(self) -> u64 {{ self.0 }}\n\
             }}\n",
        );
    }

    // Exactly one serial field: the typed point-read key
    // (`WriteTx::get::<Fact>(id)`). Relations with zero or several serial
    // fields have no single dominant key — those read through `get_dyn`
    // (the multi-key typed shape is an OPEN item, 70-api.md).
    let serials: Vec<&Field> = relation.fields.iter().filter(|f| f.serial).collect();
    if let [serial_field] = serials.as_slice() {
        let newtype = serial_field
            .newtype
            .as_ref()
            .expect("parser demands `as NewType` on serial fields");
        let _ = write!(
            out,
            "impl ::bumbledb::SerialKeyed for {name} {{\n\
                 type SerialKey = {newtype};\n\
             }}\n",
        );
    }
}
