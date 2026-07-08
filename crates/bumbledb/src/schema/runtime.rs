//! Runtime resolution for the `schema!` macro
//! (docs/architecture/70-api.md): the macro emits name-based declaration
//! data plus calls; every piece of real logic lives here. The generated
//! `schema()` resolves the declaration into id-based descriptors and runs
//! the validated constructor — the macro output is *exactly* sugar. (The
//! intern/resolve helpers the generated `Fact` impls call live in
//! `api::db::plumbing`; both are re-exported through `crate::__private`.)

use crate::error::SchemaError;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, LiteralValue, RelationDescriptor,
    RelationId, Schema, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

/// A field's declared type, name-based (macro-facing).
#[derive(Debug, Clone, Copy)]
pub enum FieldTy {
    Bool,
    U64,
    I64,
    Str,
    Bytes,
    /// The ordered variant-name list — the enum's structural identity.
    Enum(&'static [&'static str]),
    /// `interval<u64>` / `interval<i64>`.
    Interval(IntervalElement),
}

/// One declared field, name-based.
#[derive(Debug, Clone, Copy)]
pub struct FieldDecl {
    pub name: &'static str,
    pub ty: FieldTy,
    pub serial: bool,
}

/// One declared relation, name-based. Everything relational is a statement
/// (`docs/architecture/30-dependencies.md`); a field carries only its type
/// and generation.
#[derive(Debug, Clone, Copy)]
pub struct RelationDecl {
    pub name: &'static str,
    pub fields: &'static [FieldDecl],
}

/// A selection literal, borrowed for `static` declaration tables. Enum
/// ordinals arrive pre-resolved: the macro sees the variant list in the
/// same invocation (PRD 05 grammar, `docs/architecture/70-api.md`).
#[derive(Debug, Clone, Copy)]
pub enum LiteralDecl {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    IntervalU64(u64, u64),
    IntervalI64(i64, i64),
    Str(&'static str),
    Bytes(&'static [u8]),
}

/// One side of a declared containment, name-based.
#[derive(Debug, Clone, Copy)]
pub struct SideDecl {
    pub relation: &'static str,
    pub projection: &'static [&'static str],
    pub selection: &'static [(&'static str, LiteralDecl)],
}

/// One declared statement, name-based. `==` never reaches here: the macro
/// lowers it to two adjacent `Containment` declarations with the sides
/// swapped (`docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, Copy)]
pub enum StatementDecl {
    Functionality {
        relation: &'static str,
        projection: &'static [&'static str],
    },
    Containment {
        source: SideDecl,
        target: SideDecl,
    },
}

fn value_type(ty: FieldTy) -> ValueType {
    match ty {
        FieldTy::Bool => ValueType::Bool,
        FieldTy::U64 => ValueType::U64,
        FieldTy::I64 => ValueType::I64,
        FieldTy::Str => ValueType::String,
        FieldTy::Bytes => ValueType::Bytes,
        FieldTy::Enum(variants) => ValueType::Enum {
            variants: variants.iter().map(|v| Box::from(*v)).collect(),
        },
        FieldTy::Interval(element) => ValueType::Interval { element },
    }
}

fn literal_value(literal: LiteralDecl) -> LiteralValue {
    match literal {
        LiteralDecl::Bool(v) => LiteralValue::Bool(v),
        LiteralDecl::U64(v) => LiteralValue::U64(v),
        LiteralDecl::I64(v) => LiteralValue::I64(v),
        LiteralDecl::Enum(ordinal) => LiteralValue::Enum(ordinal),
        LiteralDecl::IntervalU64(start, end) => LiteralValue::IntervalU64(start, end),
        LiteralDecl::IntervalI64(start, end) => LiteralValue::IntervalI64(start, end),
        LiteralDecl::Str(s) => LiteralValue::String(s.as_bytes().into()),
        LiteralDecl::Bytes(b) => LiteralValue::Bytes(b.into()),
    }
}

fn relation_id(declarations: &[RelationDecl], name: &str) -> RelationId {
    let index = declarations
        .iter()
        .position(|r| r.name == name)
        .unwrap_or_else(|| panic!("schema!: unknown relation `{name}`"));
    RelationId(u32::try_from(index).expect("relation count fits u32"))
}

fn field_id(decl: &RelationDecl, name: &str) -> FieldId {
    let index = decl
        .fields
        .iter()
        .position(|f| f.name == name)
        .unwrap_or_else(|| panic!("schema!: unknown field `{}.{name}`", decl.name));
    FieldId(u16::try_from(index).expect("field count fits u16"))
}

/// Resolves one side's names to ids.
fn side(declarations: &[RelationDecl], decl: &SideDecl) -> Side {
    let relation = relation_id(declarations, decl.relation);
    let relation_decl = &declarations[relation.0 as usize];
    Side {
        relation,
        projection: decl
            .projection
            .iter()
            .map(|f| field_id(relation_decl, f))
            .collect(),
        selection: decl
            .selection
            .iter()
            .map(|(f, literal)| (field_id(relation_decl, f), literal_value(*literal)))
            .collect(),
    }
}

/// Resolves one declared statement's names to ids.
fn statement(declarations: &[RelationDecl], decl: &StatementDecl) -> StatementDescriptor {
    match decl {
        StatementDecl::Functionality {
            relation,
            projection,
        } => {
            let id = relation_id(declarations, relation);
            let relation_decl = &declarations[id.0 as usize];
            StatementDescriptor::Functionality {
                relation: id,
                projection: projection
                    .iter()
                    .map(|f| field_id(relation_decl, f))
                    .collect(),
            }
        }
        StatementDecl::Containment { source, target } => StatementDescriptor::Containment {
            source: side(declarations, source),
            target: side(declarations, target),
        },
    }
}

/// Resolves the name-based declaration to ids and runs the validated
/// constructor.
///
/// # Errors
///
/// The declaration validator's typed [`SchemaError`]s, unchanged.
///
/// # Panics
///
/// On unresolvable *names* (unknown relation/field names in the
/// declaration) — those are programmer errors in the `schema!` source,
/// reported with the offending name.
pub fn build_schema(
    declarations: &[RelationDecl],
    statements: &[StatementDecl],
) -> std::result::Result<Schema, SchemaError> {
    let relations = declarations
        .iter()
        .map(|decl| RelationDescriptor {
            name: decl.name.into(),
            fields: decl
                .fields
                .iter()
                .map(|f| FieldDescriptor {
                    name: f.name.into(),
                    value_type: value_type(f.ty),
                    generation: if f.serial {
                        Generation::Serial
                    } else {
                        Generation::None
                    },
                })
                .collect(),
        })
        .collect();
    SchemaDescriptor {
        relations,
        statements: statements
            .iter()
            .map(|s| statement(declarations, s))
            .collect(),
    }
    .validate()
}
