//! The `schema!` proc-macro (PRD 27): bumbledb's declarative schema
//! surface. A small, rigid grammar — this is Rust-side declaration, not a
//! query language — hand-parsed over the raw token stream (no `syn`, no
//! `quote`: the grammar is not Rust syntax and the dependency would buy
//! nothing).
//!
//! ```text
//! schema! {
//!     relation Account {
//!         id:     u64 as AccountId, serial,
//!         holder: u64 as HolderId,  fk(Holder.id),
//!         status: enum Status { Active, Closed },
//!         unique(holder, status),
//!     }
//!     relation Holder { id: u64 as HolderId, serial, name: str }
//! }
//! ```
//!
//! Types: `bool`, `u64`, `i64`, `str`, `bytes`, inline `enum Name { .. }`
//! (the name names the generated Rust enum only — engine identity is the
//! structural variant list). `as NewType` generates the host-side nominal
//! newtype. `serial` implies the auto-unique (writing `unique` too is
//! tolerated and ignored). Relation-level `unique(f, ..)` and
//! `fk(f, .. -> Rel.target)` declare compound constraints; the target
//! names a unique constraint (a serial field's auto-unique shares its
//! field's name).
//!
//! The macro does **no** validation of its own: expansion emits data plus
//! calls into `bumbledb::schema::runtime`, and every schema error surfaces
//! as PRD 02's typed errors at the first `schema()` call (memoized in a
//! `OnceLock`).

use proc_macro::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::iter::Peekable;

#[derive(Debug, Clone)]
enum FieldTy {
    Bool,
    U64,
    I64,
    Str,
    Bytes,
    Enum { name: String, variants: Vec<String> },
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: FieldTy,
    newtype: Option<String>,
    serial: bool,
    unique: bool,
    /// `(target relation, target constraint/field name)`.
    fk: Option<(String, String)>,
}

#[derive(Debug, Clone)]
struct Relation {
    name: String,
    fields: Vec<Field>,
    /// Compound uniques: field-name lists.
    uniques: Vec<Vec<String>>,
    /// Compound FKs: `(field names, target relation, target name)`.
    fks: Vec<(Vec<String>, String, String)>,
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

/// Parses a comma-separated identifier list.
fn ident_list(stream: TokenStream) -> Vec<String> {
    let mut names = Vec::new();
    let mut tokens = stream.into_iter().peekable();
    while tokens.peek().is_some() {
        names.push(expect_ident(&mut tokens, "a field name"));
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    names
}

/// Parses `Rel.target` out of an fk group's tail.
fn fk_target(tokens: &mut Tokens) -> (String, String) {
    let relation = expect_ident(tokens, "a target relation name");
    expect_punct(tokens, '.');
    let target = expect_ident(tokens, "a target constraint or field name");
    (relation, target)
}

/// Parses one relation body.
fn parse_relation(name: String, body: TokenStream) -> Relation {
    let mut relation = Relation {
        name,
        fields: Vec::new(),
        uniques: Vec::new(),
        fks: Vec::new(),
    };
    let mut tokens = body.into_iter().peekable();
    while tokens.peek().is_some() {
        let ident = expect_ident(&mut tokens, "a field name or clause");
        match (ident.as_str(), tokens.peek()) {
            // Relation-level compound clauses: `unique(..)` / `fk(.. -> R.t)`.
            ("unique", Some(TokenTree::Group(g))) if g.delimiter() == Delimiter::Parenthesis => {
                let group = take_group(&mut tokens, Delimiter::Parenthesis, "a field list");
                relation.uniques.push(ident_list(group));
            }
            ("fk", Some(TokenTree::Group(g))) if g.delimiter() == Delimiter::Parenthesis => {
                let group = take_group(&mut tokens, Delimiter::Parenthesis, "an fk clause");
                let mut inner = group.into_iter().peekable();
                let mut fields = Vec::new();
                loop {
                    fields.push(expect_ident(&mut inner, "a field name"));
                    if peek_punct(&mut inner, ',') {
                        inner.next();
                        continue;
                    }
                    break;
                }
                expect_punct(&mut inner, '-');
                expect_punct(&mut inner, '>');
                let (target_relation, target) = fk_target(&mut inner);
                relation.fks.push((fields, target_relation, target));
            }
            // A field entry.
            _ => {
                expect_punct(&mut tokens, ':');
                let field = parse_field(ident, &mut tokens);
                relation.fields.push(field);
            }
        }
        if peek_punct(&mut tokens, ',') {
            tokens.next();
        }
    }
    relation
}

/// Parses a field's type, optional newtype, and trailing modifiers.
fn parse_field(name: String, tokens: &mut Tokens) -> Field {
    let ty_name = expect_ident(tokens, "a type (bool/u64/i64/str/bytes/enum)");
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
        other => panic!("schema!: unknown type `{other}`"),
    };
    let mut field = Field {
        name,
        ty,
        newtype: None,
        serial: false,
        unique: false,
        fk: None,
    };
    if peek_ident(tokens).as_deref() == Some("as") {
        tokens.next();
        assert!(
            matches!(field.ty, FieldTy::U64 | FieldTy::I64),
            "schema!: `as NewType` applies to u64/i64 fields only"
        );
        field.newtype = Some(expect_ident(tokens, "a newtype name"));
    }
    // Modifiers: `, serial` `, unique` `, fk(Rel.target)` until the next
    // field (an ident followed by `:`), a relation-level clause, or the end.
    loop {
        if !peek_punct(tokens, ',') {
            break;
        }
        let mut lookahead = tokens.clone();
        lookahead.next(); // the comma
        match lookahead.peek() {
            Some(TokenTree::Ident(ident)) => {
                let word = ident.to_string();
                lookahead.next();
                let starts_clause = matches!(
                    (word.as_str(), lookahead.peek()),
                    ("unique" | "fk", Some(TokenTree::Group(g)))
                        if g.delimiter() == Delimiter::Parenthesis
                );
                let is_modifier = !starts_clause
                    && matches!(word.as_str(), "serial" | "unique")
                    && !matches!(lookahead.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ':');
                let is_field_fk = word == "fk" && starts_clause && {
                    // A field-level fk has exactly `Rel.target` inside;
                    // compound (relation-level) fks carry a `->`. Peek the
                    // group to distinguish.
                    if let Some(TokenTree::Group(g)) = lookahead.peek() {
                        !g.stream().to_string().contains("->")
                    } else {
                        false
                    }
                };
                if is_modifier {
                    tokens.next(); // comma
                    tokens.next(); // the modifier word
                    match word.as_str() {
                        "serial" => field.serial = true,
                        "unique" => field.unique = true,
                        _ => unreachable!(),
                    }
                } else if is_field_fk {
                    tokens.next(); // comma
                    tokens.next(); // `fk`
                    let group = take_group(tokens, Delimiter::Parenthesis, "an fk target");
                    let mut inner = group.into_iter().peekable();
                    field.fk = Some(fk_target(&mut inner));
                } else {
                    break; // next field or relation-level clause
                }
            }
            _ => break,
        }
    }
    field
}

/// Parses the whole `schema!` body into relations.
fn parse_schema(input: TokenStream) -> Vec<Relation> {
    let mut relations = Vec::new();
    let mut tokens = input.into_iter().peekable();
    while tokens.peek().is_some() {
        let keyword = expect_ident(&mut tokens, "`relation`");
        assert_eq!(keyword, "relation", "schema!: expected `relation`");
        let name = expect_ident(&mut tokens, "a relation name");
        let body = take_group(&mut tokens, Delimiter::Brace, "a relation body");
        relations.push(parse_relation(name, body));
    }
    relations
}

/// The declarative schema surface: expands to `fn schema()`, host-side
/// newtypes and enums, and one typed fact struct per relation with
/// `encode_write`/`encode_read`/`decode` boundaries. All real logic lives
/// in `bumbledb::schema::runtime`; the expansion emits data and calls.
///
/// # Panics
///
/// On malformed `schema!` grammar — a compile error at the macro call
/// site, reported with the offending token.
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let relations = parse_schema(input);
    let mut out = String::new();
    emit_schema_fn(&mut out, &relations);
    emit_newtypes(&mut out, &relations);
    emit_enums(&mut out, &relations);
    for (index, relation) in relations.iter().enumerate() {
        emit_fact_struct(&mut out, index, relation);
    }
    out.parse().expect("schema!: generated code parses")
}

fn ty_decl(ty: &FieldTy) -> String {
    match ty {
        FieldTy::Bool => "::bumbledb::schema::runtime::FieldTy::Bool".to_owned(),
        FieldTy::U64 => "::bumbledb::schema::runtime::FieldTy::U64".to_owned(),
        FieldTy::I64 => "::bumbledb::schema::runtime::FieldTy::I64".to_owned(),
        FieldTy::Str => "::bumbledb::schema::runtime::FieldTy::Str".to_owned(),
        FieldTy::Bytes => "::bumbledb::schema::runtime::FieldTy::Bytes".to_owned(),
        FieldTy::Enum { variants, .. } => {
            let list = variants
                .iter()
                .map(|v| format!("\"{v}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("::bumbledb::schema::runtime::FieldTy::Enum(&[{list}])")
        }
    }
}

fn emit_schema_fn(out: &mut String, relations: &[Relation]) {
    let mut decls = String::new();
    for relation in relations {
        let mut fields = String::new();
        for field in &relation.fields {
            let fk = match &field.fk {
                Some((rel, target)) => format!("Some((\"{rel}\", \"{target}\"))"),
                None => "None".to_owned(),
            };
            // `serial` implies the auto-unique: a redundant `unique` is
            // tolerated and dropped (it would collide with the auto name).
            let unique = field.unique && !field.serial;
            let _ = write!(
                fields,
                "::bumbledb::schema::runtime::FieldDecl {{ name: \"{}\", ty: {}, serial: {}, unique: {}, fk: {} }},",
                field.name,
                ty_decl(&field.ty),
                field.serial,
                unique,
                fk,
            );
        }
        let mut uniques = String::new();
        for names in &relation.uniques {
            let list = names
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = write!(uniques, "&[{list}],");
        }
        let mut fks = String::new();
        for (names, rel, target) in &relation.fks {
            let list = names
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = write!(fks, "(&[{list}], \"{rel}\", \"{target}\"),");
        }
        let _ = write!(
            decls,
            "::bumbledb::schema::runtime::RelationDecl {{ name: \"{}\", fields: &[{fields}], uniques: &[{uniques}], fks: &[{fks}] }},",
            relation.name,
        );
    }
    let _ = write!(
        out,
        "/// The compiled schema (memoized; declaration errors surface as \
         the typed `SchemaError` at the first call).\n\
         pub fn schema() -> &'static ::bumbledb::schema::Schema {{\n\
             static SCHEMA: ::std::sync::OnceLock<::bumbledb::schema::Schema> = ::std::sync::OnceLock::new();\n\
             SCHEMA.get_or_init(|| {{\n\
                 ::bumbledb::schema::runtime::build_schema(&[{decls}])\n\
                     .expect(\"schema! declaration is invalid\")\n\
             }})\n\
         }}\n",
    );
}

fn emit_newtypes(out: &mut String, relations: &[Relation]) {
    let mut newtypes: BTreeMap<String, &'static str> = BTreeMap::new();
    for relation in relations {
        for field in &relation.fields {
            if let Some(name) = &field.newtype {
                let inner = match field.ty {
                    FieldTy::U64 => "u64",
                    FieldTy::I64 => "i64",
                    _ => unreachable!("parser restricts `as` to u64/i64"),
                };
                newtypes.insert(name.clone(), inner);
            }
        }
    }
    for (name, inner) in newtypes {
        let _ = write!(
            out,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n\
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
    }
}

/// The `ValueRef` expressions for one field in the write and read encode
/// contexts (write interns novel values; read bails `Ok(false)` on a miss).
fn encode_exprs(field: &Field) -> (String, String) {
    let access = if field.newtype.is_some() {
        format!("self.{}.0", field.name)
    } else {
        format!("self.{}", field.name)
    };
    match &field.ty {
        FieldTy::Bool => {
            let expr = format!("::bumbledb::encoding::ValueRef::Bool({access})");
            (expr.clone(), expr)
        }
        FieldTy::U64 => {
            let expr = format!("::bumbledb::encoding::ValueRef::U64({access})");
            (expr.clone(), expr)
        }
        FieldTy::I64 => {
            let expr = format!("::bumbledb::encoding::ValueRef::I64({access})");
            (expr.clone(), expr)
        }
        FieldTy::Enum { .. } => {
            let expr = format!(
                "::bumbledb::encoding::ValueRef::Enum(self.{}.ordinal())",
                field.name
            );
            (expr.clone(), expr)
        }
        FieldTy::Str => (
            format!(
                "::bumbledb::encoding::ValueRef::String(::bumbledb::schema::runtime::intern_str_write(view, delta, &self.{})?)",
                field.name
            ),
            format!(
                "match ::bumbledb::schema::runtime::intern_str_read(txn, &self.{})? {{ Some(id) => ::bumbledb::encoding::ValueRef::String(id), None => return Ok(false) }}",
                field.name
            ),
        ),
        FieldTy::Bytes => (
            format!(
                "::bumbledb::encoding::ValueRef::Bytes(::bumbledb::schema::runtime::intern_bytes_write(view, delta, &self.{})?)",
                field.name
            ),
            format!(
                "match ::bumbledb::schema::runtime::intern_bytes_read(txn, &self.{})? {{ Some(id) => ::bumbledb::encoding::ValueRef::Bytes(id), None => return Ok(false) }}",
                field.name
            ),
        ),
    }
}

/// The struct-literal arm decoding one field out of canonical fact bytes.
fn decode_arm(field: &Field, idx: usize) -> String {
    let wrap = |expr: &str| -> String {
        match &field.newtype {
            Some(newtype) => format!("{newtype}({expr})"),
            None => expr.to_owned(),
        }
    };
    match &field.ty {
        FieldTy::Bool => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::Bool(v) => v, _ => unreachable!(\"schema-typed\") }},",
            field.name
        ),
        FieldTy::U64 => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::U64(v) => {}, _ => unreachable!(\"schema-typed\") }},",
            field.name,
            wrap("v")
        ),
        FieldTy::I64 => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::I64(v) => {}, _ => unreachable!(\"schema-typed\") }},",
            field.name,
            wrap("v")
        ),
        FieldTy::Enum { name: enum_name, .. } => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::Enum(o) => {enum_name}::from_ordinal(o).expect(\"decode_field range-checked the ordinal\"), _ => unreachable!(\"schema-typed\") }},",
            field.name
        ),
        FieldTy::Str => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::String(id) => ::bumbledb::schema::runtime::resolve_string(txn, id)?, _ => unreachable!(\"schema-typed\") }},",
            field.name
        ),
        FieldTy::Bytes => format!(
            "{}: match ::bumbledb::encoding::decode_field(fact, layout, {idx})? {{ ::bumbledb::encoding::ValueRef::Bytes(id) => ::bumbledb::schema::runtime::resolve_bytes(txn, id)?, _ => unreachable!(\"schema-typed\") }},",
            field.name
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
    let mut read_values = String::new();
    let mut decode_fields = String::new();
    for (idx, field) in relation.fields.iter().enumerate() {
        let (write_expr, read_expr) = encode_exprs(field);
        let _ = write!(write_values, "{write_expr},");
        let _ = write!(read_values, "{read_expr},");
        let _ = write!(decode_fields, "{}", decode_arm(field, idx));
    }

    let _ = write!(
        out,
        "#[derive(Debug, Clone, PartialEq)]\n\
         pub struct {name} {{ {struct_fields} }}\n\
         impl {name} {{\n\
             pub const RELATION: ::bumbledb::schema::RelationId = ::bumbledb::schema::RelationId({index});\n\
             /// Encodes against a write context, interning novel strings.\n\
             /// # Errors\n\
             /// Storage errors from the intern path.\n\
             pub fn encode_write(&self, schema: &::bumbledb::schema::Schema, view: &::bumbledb::storage::env::ReadTxn<'_>, delta: &mut ::bumbledb::storage::delta::WriteDelta<'_>, out: &mut Vec<u8>) -> ::bumbledb::error::Result<()> {{\n\
                 let layout = schema.relation(Self::RELATION).layout();\n\
                 ::bumbledb::encoding::encode_fact(&[{write_values}], layout, out);\n\
                 Ok(())\n\
             }}\n\
             /// Encodes against a read context; `Ok(false)` when a string or\n\
             /// bytes value was never interned (the fact cannot exist).\n\
             /// # Errors\n\
             /// Storage errors from the lookup path.\n\
             pub fn encode_read(&self, schema: &::bumbledb::schema::Schema, txn: &::bumbledb::storage::env::ReadTxn<'_>, out: &mut Vec<u8>) -> ::bumbledb::error::Result<bool> {{\n\
                 let layout = schema.relation(Self::RELATION).layout();\n\
                 ::bumbledb::encoding::encode_fact(&[{read_values}], layout, out);\n\
                 Ok(true)\n\
             }}\n\
             /// Decodes canonical fact bytes back into the typed struct.\n\
             /// # Errors\n\
             /// `Corruption` on undecodable bytes or dangling intern ids.\n\
             /// # Panics\n\
             /// Only on programmer-invariant violations (schema-typed\n\
             /// variants).\n\
             pub fn decode(schema: &::bumbledb::schema::Schema, txn: &::bumbledb::storage::env::ReadTxn<'_>, fact: &[u8]) -> ::bumbledb::error::Result<Self> {{\n\
                 let layout = schema.relation(Self::RELATION).layout();\n\
                 Ok(Self {{ {decode_fields} }})\n\
             }}\n\
         }}\n",
    );
}
