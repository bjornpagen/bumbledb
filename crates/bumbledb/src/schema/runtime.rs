//! Runtime resolution for the `schema!` macro (PRD 27): the macro emits
//! name-based declaration data plus calls; every piece of real logic lives
//! here. The generated `schema()` resolves the declaration into id-based
//! descriptors and runs PRD 02's validated constructor — the macro output
//! is *exactly* sugar. (The intern/resolve helpers the generated `Fact`
//! impls call live in `api::db::plumbing`; both are re-exported through
//! `crate::__private`.)

use crate::error::SchemaError;
use crate::schema::{
    ConstraintDescriptor, ConstraintId, FieldDescriptor, FieldId, Generation, RelationDescriptor,
    RelationId, Schema, SchemaDescriptor, ValueType,
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
}

/// One declared field, name-based.
#[derive(Debug, Clone, Copy)]
pub struct FieldDecl {
    pub name: &'static str,
    pub ty: FieldTy,
    pub serial: bool,
    /// A single-field declared unique (ignored by the macro for serial
    /// fields — the auto-unique already covers them).
    pub unique: bool,
    /// `(target relation, target constraint name)` — a serial field's
    /// auto-unique shares its field's name, so `Rel.field` and
    /// `Rel.constraint` are one namespace.
    pub fk: Option<(&'static str, &'static str)>,
}

/// One declared relation, name-based.
#[derive(Debug, Clone, Copy)]
pub struct RelationDecl {
    pub name: &'static str,
    pub fields: &'static [FieldDecl],
    /// Compound uniques (auto-named by joining field names with `_`).
    pub uniques: &'static [&'static [&'static str]],
    /// Compound FKs: `(fields, target relation, target constraint name)`.
    pub fks: &'static [(&'static [&'static str], &'static str, &'static str)],
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

/// Constraint ids follow PRD 02's numbering: auto-uniques (serial fields in
/// declaration order) first, then declared constraints in order. The
/// declared order here is: per-field uniques, per-field fks, compound
/// uniques, compound fks.
fn constraint_id(decl: &RelationDecl, name: &str) -> ConstraintId {
    let mut names: Vec<String> = decl
        .fields
        .iter()
        .filter(|f| f.serial)
        .map(|f| f.name.to_owned())
        .collect();
    names.extend(
        decl.fields
            .iter()
            .filter(|f| f.unique && !f.serial)
            .map(|f| f.name.to_owned()),
    );
    names.extend(
        decl.fields
            .iter()
            .filter(|f| f.fk.is_some())
            .map(|f| format!("{}_fk", f.name)),
    );
    names.extend(decl.uniques.iter().map(|fields| fields.join("_")));
    names.extend(
        decl.fks
            .iter()
            .map(|(fields, _, _)| format!("{}_fk", fields.join("_"))),
    );
    let index = names
        .iter()
        .position(|n| n == name)
        .unwrap_or_else(|| panic!("schema!: unknown constraint `{}.{name}`", decl.name));
    ConstraintId(u16::try_from(index).expect("constraint count fits u16"))
}

/// One relation's declared constraints, ids resolved, in
/// [`constraint_id`]'s declared order.
fn declared_constraints(
    declarations: &[RelationDecl],
    decl: &RelationDecl,
) -> Vec<ConstraintDescriptor> {
    let target_decl = |target_relation: &str| -> &RelationDecl {
        &declarations
            [usize::try_from(relation_id(declarations, target_relation).0).expect("64-bit usize")]
    };
    let mut constraints: Vec<ConstraintDescriptor> = Vec::new();
    for f in decl.fields.iter().filter(|f| f.unique && !f.serial) {
        constraints.push(ConstraintDescriptor::Unique {
            name: f.name.into(),
            fields: Box::new([field_id(decl, f.name)]),
        });
    }
    for f in decl.fields {
        if let Some((target_relation, target)) = f.fk {
            constraints.push(ConstraintDescriptor::ForeignKey {
                name: format!("{}_fk", f.name).into(),
                fields: Box::new([field_id(decl, f.name)]),
                target_relation: relation_id(declarations, target_relation),
                target_constraint: constraint_id(target_decl(target_relation), target),
            });
        }
    }
    for unique_fields in decl.uniques {
        constraints.push(ConstraintDescriptor::Unique {
            name: unique_fields.join("_").into(),
            fields: unique_fields.iter().map(|f| field_id(decl, f)).collect(),
        });
    }
    for (fk_fields, target_relation, target) in decl.fks {
        constraints.push(ConstraintDescriptor::ForeignKey {
            name: format!("{}_fk", fk_fields.join("_")).into(),
            fields: fk_fields.iter().map(|f| field_id(decl, f)).collect(),
            target_relation: relation_id(declarations, target_relation),
            target_constraint: constraint_id(target_decl(target_relation), target),
        });
    }
    constraints
}

/// Resolves the name-based declaration to ids and runs the validated
/// constructor.
///
/// # Errors
///
/// PRD 02's typed [`SchemaError`]s, unchanged.
///
/// # Panics
///
/// On unresolvable *names* (unknown relation/field/constraint names in the
/// declaration) — those are programmer errors in the `schema!` source,
/// reported with the offending name.
pub fn build_schema(declarations: &[RelationDecl]) -> std::result::Result<Schema, SchemaError> {
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
            constraints: declared_constraints(declarations, decl),
        })
        .collect();
    SchemaDescriptor { relations }.validate()
}
