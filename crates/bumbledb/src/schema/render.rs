//! Statement rendering back to the `schema!` algebra notation
//! (`docs/architecture/70-api.md` § grammar). Statements are anonymous —
//! their identity is their materialized-order id — and errors cite the
//! statement rendered back in this notation
//! (`docs/architecture/30-dependencies.md`).
//!
//! Rendering allocates; it runs only in `Display`/diagnostic contexts
//! (`crate::error`), never on a write or query path.

use std::fmt;

use super::{
    FieldDescriptor, FieldId, RelationId, Schema, SchemaDescriptor, Side, StatementDescriptor,
    StatementId, Value, ValueType,
};

/// Renders one sealed statement in the exact macro notation: an FD as
/// `SavingsTerms(account) -> SavingsTerms`, a containment as
/// `Account(holder) <= Holder(id)` with any selection after `|`
/// (`Account(id | kind == Savings)`), and a bidirectional pair — read off
/// the sealed [`Statement::mirror`](super::Statement::mirror) link — as
/// `==` once, in the pair's written orientation (both ids render the same
/// string). Selection literals render through one value formatter: enum
/// ordinals resolve to variant names, intervals as `start..end`.
///
/// # Panics
///
/// On an out-of-range id — statement ids are validated, internal data.
#[must_use]
pub fn render(schema: &Schema, id: StatementId) -> String {
    let statement = schema.statement(id);
    Rendered {
        names: &SealedNames(schema),
        descriptor: &statement.descriptor,
        mirror: statement.mirror,
        id,
    }
    .to_string()
}

/// [`render`]'s declaration-side sibling, for schema-error diagnostics: a
/// rejected declaration never seals a [`Schema`], so the statement renders
/// from the descriptor. `id` indexes
/// [`SchemaDescriptor::materialized_statements`] — exactly what
/// [`crate::error::SchemaError`] payloads carry. Names a rejected
/// statement may fail to resolve (that can be the error) render as
/// `relation#N`/`field#N` placeholders.
///
/// # Panics
///
/// On an out-of-range id — schema errors carry ids produced by validating
/// this same descriptor.
#[must_use]
pub fn render_declared(descriptor: &SchemaDescriptor, id: StatementId) -> String {
    let materialized = descriptor.materialized_statements();
    let index = usize::from(id.0);
    Rendered {
        names: &DeclaredNames(descriptor),
        descriptor: &materialized[index],
        // A rejected declaration seals no `mirror` field to read, so the
        // pairing comes from sealing's one computation site.
        mirror: super::validate::mirror_of(&materialized, index),
        id,
    }
    .to_string()
}

/// Name resolution over whichever schema form the caller holds. `None`
/// falls back to an id placeholder — the declared path renders statements
/// whose ids may be the very thing validation rejected.
trait Names {
    fn relation_name(&self, relation: RelationId) -> Option<&str>;
    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor>;
}

struct SealedNames<'a>(&'a Schema);

impl Names for SealedNames<'_> {
    fn relation_name(&self, relation: RelationId) -> Option<&str> {
        self.0.relation_checked(relation).map(super::Relation::name)
    }

    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor> {
        self.0
            .relation_checked(relation)?
            .fields()
            .get(usize::from(field.0))
    }
}

struct DeclaredNames<'a>(&'a SchemaDescriptor);

impl Names for DeclaredNames<'_> {
    fn relation_name(&self, relation: RelationId) -> Option<&str> {
        self.0.relations.get(relation.0 as usize).map(|r| &*r.name)
    }

    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor> {
        self.0
            .relations
            .get(relation.0 as usize)?
            .fields
            .get(usize::from(field.0))
    }
}

/// The lazy renderer behind both entry points.
struct Rendered<'a> {
    names: &'a dyn Names,
    descriptor: &'a StatementDescriptor,
    /// The `==` partner, if any — the sealed fact, never re-detected here.
    mirror: Option<StatementId>,
    id: StatementId,
}

impl fmt::Display for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.descriptor {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                side_parts(f, self.names, *relation, projection, &[])?;
                write!(f, " -> ")?;
                relation_name(f, self.names, *relation)
            }
            StatementDescriptor::Containment { source, target } => match self.mirror {
                // A mirrored pair renders `==` once, canonically in the
                // lower id's written orientation — both partners produce
                // the same string, so the higher id flips its sides
                // (which *are* the partner's sides, swapped).
                Some(partner) if partner < self.id => {
                    side(f, self.names, target)?;
                    write!(f, " == ")?;
                    side(f, self.names, source)
                }
                Some(_) => {
                    side(f, self.names, source)?;
                    write!(f, " == ")?;
                    side(f, self.names, target)
                }
                None => {
                    side(f, self.names, source)?;
                    write!(f, " <= ")?;
                    side(f, self.names, target)
                }
            },
        }
    }
}

fn relation_name(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
) -> fmt::Result {
    match names.relation_name(relation) {
        Some(name) => write!(f, "{name}"),
        None => write!(f, "relation#{}", relation.0),
    }
}

fn field_name(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
    field: FieldId,
) -> fmt::Result {
    match names.field(relation, field) {
        Some(descriptor) => write!(f, "{}", descriptor.name),
        None => write!(f, "field#{}", field.0),
    }
}

fn side(f: &mut fmt::Formatter<'_>, names: &dyn Names, side: &Side) -> fmt::Result {
    side_parts(f, names, side.relation, &side.projection, &side.selection)
}

/// `Name(p1, p2 | s1 == lit1, s2 == lit2)` — the one side shape; the
/// selection block only when σ is nonempty.
fn side_parts(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
    projection: &[FieldId],
    selection: &[(FieldId, Value)],
) -> fmt::Result {
    relation_name(f, names, relation)?;
    write!(f, "(")?;
    for (index, field) in projection.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        field_name(f, names, relation, *field)?;
    }
    if !selection.is_empty() {
        write!(f, " | ")?;
        for (index, (field, value)) in selection.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            field_name(f, names, relation, *field)?;
            write!(f, " == ")?;
            literal(f, names, relation, *field, value)?;
        }
    }
    write!(f, ")")
}

/// The one selection-literal formatter: enum ordinals resolve to variant
/// names through the schema (out-of-range — a diagnosable rejection —
/// falls back to the bare ordinal), intervals render as their macro form
/// `start..end`, strings and bytes as escaped literals.
fn literal(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
    field: FieldId,
    value: &Value,
) -> fmt::Result {
    match value {
        Value::Bool(v) => write!(f, "{v}"),
        Value::U64(v) => write!(f, "{v}"),
        Value::I64(v) => write!(f, "{v}"),
        // Unreachable through validated schemas (a mask is not a field
        // type, so no selection holds one); rendered anyway — Display
        // stays total on plain data.
        Value::AllenMask(mask) => write!(f, "allen({:#015b})", mask.bits()),
        Value::Enum(ordinal) => {
            let variant =
                names
                    .field(relation, field)
                    .and_then(|descriptor| match &descriptor.value_type {
                        ValueType::Enum { variants } => variants.get(usize::from(*ordinal)),
                        _ => None,
                    });
            match variant {
                Some(name) => write!(f, "{name}"),
                None => write!(f, "{ordinal}"),
            }
        }
        Value::IntervalU64(start, end) => write!(f, "{start}..{end}"),
        Value::IntervalI64(start, end) => write!(f, "{start}..{end}"),
        Value::String(bytes) => {
            write!(f, "\"")?;
            for c in String::from_utf8_lossy(bytes).chars() {
                write!(f, "{}", c.escape_debug())?;
            }
            write!(f, "\"")
        }
        Value::FixedBytes(bytes) => {
            write!(f, "b\"")?;
            for byte in bytes.as_ref() {
                write!(f, "{}", byte.escape_ascii())?;
            }
            write!(f, "\"")
        }
    }
}

#[cfg(test)]
mod tests;
