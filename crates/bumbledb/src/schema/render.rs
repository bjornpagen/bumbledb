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
    FieldDescriptor, FieldId, LiteralSet, RelationId, Schema, SchemaDescriptor, Side,
    StatementDescriptor, StatementId, StatementKind, StatementView, Value, ValueType,
};
use crate::error::{Direction, Violation, Violations};

/// One rejected commit's citation rendered as plain data — everything a
/// bindings layer needs to show (or prompt with) the rejection: the
/// statement's fingerprint-pinned id, its form tag, its canonical
/// spelling (the renderer is a bijection on legal statements, so the
/// spelling pastes back), the direction/count payloads where the form
/// carries them, and the offending facts as named decoded values
/// (`docs/architecture/30-dependencies.md` § rendering the rejection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedViolation {
    pub statement: StatementId,
    pub kind: StatementKind,
    /// The statement in canonical spelling ([`render_declared`] — the
    /// single renderer, never a second stringifier).
    pub spelling: String,
    /// A containment citation's violated side; `None` for keys and
    /// windows.
    pub direction: Option<Direction>,
    /// A window citation's observed child-group count; `None` otherwise.
    pub count: Option<u64>,
    /// The offending facts — the violation's fact, then its incumbent
    /// where one exists — as named decoded values.
    pub facts: Vec<RenderedFact>,
}

/// One offending fact with its names resolved: the relation name and one
/// `(field name, value)` pair per sealed field, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFact {
    pub relation: Box<str>,
    pub fields: Vec<(Box<str>, Value)>,
}

/// Renders a rejection's complete violation set as plain data — the
/// bindings-consumable form of [`crate::error::Error::CommitRejected`]:
/// per citation, the statement id, kind tag, canonical spelling, and the
/// offending facts as `(relation name, [(field name, value)])` rows,
/// read off the decoded cited facts the commit boundary attached
/// ([`Violations::cited_facts`]). Pure over the descriptor: a foreign
/// host renders with its cached [`crate::Theory`] descriptor and no
/// database handle.
///
/// Total on plain data: unknown ids render as `relation#N` / `field#N`
/// placeholders, the declared renderer's own convention.
///
/// # Panics
///
/// When a cited fact's field ordinal exceeds the id space (`u16`) —
/// impossible for facts decoded from a schema the declaration boundary
/// admitted ([`crate::error::SchemaError::RelationTooManyColumns`] is
/// the typed rejection for such counts).
#[must_use]
pub fn render_rejection(
    descriptor: &SchemaDescriptor,
    violations: &Violations,
) -> Vec<RenderedViolation> {
    let names = DeclaredNames(descriptor);
    let materialized = descriptor.materialized_statements();
    violations
        .citations()
        .map(|(violation, cited)| {
            let statement = violation.statement();
            let (kind, direction, count) = match violation {
                Violation::Functionality { .. } => (StatementKind::Functionality, None, None),
                Violation::Containment { direction, .. } => {
                    (StatementKind::Containment, Some(*direction), None)
                }
                Violation::Cardinality { count, .. } => {
                    (StatementKind::Cardinality, None, Some(*count))
                }
            };
            RenderedViolation {
                statement,
                kind,
                // Guarded, not indexed: a statement id beyond this
                // descriptor's materialized roster (a foreign
                // descriptor) renders as a placeholder — the declared
                // renderer's own convention, kept total.
                spelling: if usize::from(statement.0) < materialized.len() {
                    render_declared(descriptor, statement)
                } else {
                    format!("statement#{}", statement.0)
                },
                direction,
                count,
                facts: cited
                    .iter()
                    .map(|fact| RenderedFact {
                        relation: names.relation_name(fact.relation).map_or_else(
                            || format!("relation#{}", fact.relation.0).into(),
                            Box::from,
                        ),
                        fields: fact
                            .values
                            .iter()
                            .enumerate()
                            .map(|(idx, value)| {
                                let field =
                                    FieldId(u16::try_from(idx).expect("field count fits u16"));
                                let name = names.field(fact.relation, field).map_or_else(
                                    || format!("field#{}", field.0).into(),
                                    |descriptor| descriptor.name.clone(),
                                );
                                (name, value.clone())
                            })
                            .collect(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Renders one sealed statement in the exact macro notation: an FD as
/// `SavingsTerms(account) -> SavingsTerms`, a containment as
/// `Account(holder) <= Holder(id)` with any selection after `|`
/// (`Account(id | kind == Savings)`), and a bidirectional pair — read off
/// the sealed [`super::ContainmentStatement::mirror`] link — as
/// `==` once, in the pair's written orientation (both ids render the same
/// string), and a cardinality window B-family, target-left, in its one
/// canonical spelling (`Parent(id) <={1..3} Task(parent)`; `lo = hi`
/// prints `{n}`). Selection literals render through one value formatter;
/// intervals render as `start..end`.
///
/// # Panics
///
/// On an out-of-range id — statement ids are validated, internal data.
#[must_use]
pub fn render(schema: &Schema, id: StatementId) -> String {
    let statement = match schema.statement(id) {
        StatementView::Key(_, statement) => RenderedStatement::Key {
            relation: statement.relation,
            projection: &statement.projection,
        },
        StatementView::Containment(_, statement) => RenderedStatement::Containment {
            source: &statement.source,
            target: &statement.target,
            mirror: statement.mirror,
        },
        StatementView::Cardinality(_, statement) => RenderedStatement::Cardinality {
            source: &statement.source,
            lo: statement.lo,
            hi: statement.hi,
            target: &statement.target,
        },
    };
    Rendered {
        names: &SealedNames(schema),
        statement,
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
    let statement = match &materialized[index] {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => RenderedStatement::Key {
            relation: *relation,
            projection,
        },
        StatementDescriptor::Containment { source, target } => {
            RenderedStatement::Containment {
                source,
                target,
                // A rejected declaration seals no `mirror` field to read,
                // so the pairing comes from sealing's one computation site.
                mirror: super::validate::mirror_of(&materialized, index),
            }
        }
        StatementDescriptor::Cardinality {
            source,
            lo,
            hi,
            target,
        } => RenderedStatement::Cardinality {
            source,
            lo: *lo,
            hi: *hi,
            target,
        },
    };
    Rendered {
        names: &DeclaredNames(descriptor),
        statement,
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
    /// `(relation, field)` as a closed-reference position: the closed
    /// relation whose row ids the field's words are — a walk over the
    /// declared containments whose target is a closed relation's id and
    /// whose source projection is that single field (`ir/render`'s own
    /// inference), plus each closed relation's id field mapping to
    /// itself. The macro admits bare handles by the field's *newtype*,
    /// which the engine never learns; the declared containment is the
    /// engine-visible fact this walk reads.
    fn closed_target(&self, relation: RelationId, field: FieldId) -> Option<RelationId>;
    /// Row `id` of closed relation `closed`, as its handle; `None` = out
    /// of range (the caller prints the visibly-wrong fallback).
    fn handle(&self, closed: RelationId, id: u64) -> Option<String>;
}

/// The shared containment walk behind both [`Names::closed_target`]
/// impls, over whichever statement list the schema form carries.
fn closed_target_of<'a>(
    statements: impl Iterator<Item = (&'a Side, &'a Side)>,
    is_closed: impl Fn(RelationId) -> bool,
    relation: RelationId,
    field: FieldId,
) -> Option<RelationId> {
    if field == FieldId(0) && is_closed(relation) {
        return Some(relation);
    }
    statements.into_iter().find_map(|(source, target)| {
        (source.relation == relation
            && source.projection.as_ref() == [field]
            && target.projection.as_ref() == [FieldId(0)]
            && is_closed(target.relation))
        .then_some(target.relation)
    })
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

    fn closed_target(&self, relation: RelationId, field: FieldId) -> Option<RelationId> {
        closed_target_of(
            self.0
                .containments()
                .iter()
                .map(|statement| (&statement.source, &statement.target)),
            |id| {
                self.0
                    .relation_checked(id)
                    .is_some_and(super::Relation::is_closed)
            },
            relation,
            field,
        )
    }

    fn handle(&self, closed: RelationId, id: u64) -> Option<String> {
        let rows = self.0.relation_checked(closed)?.extension()?;
        usize::try_from(id)
            .ok()
            .and_then(|row| rows.get(row))
            .map(|row| row.handle.to_string())
    }
}

struct DeclaredNames<'a>(&'a SchemaDescriptor);

/// The synthetic (`id`, U64) field a closed relation's sealed list opens
/// with — the declared-side renderer resolves the same ids the sealed
/// schema answers to.
static SYNTHETIC_ID: std::sync::LazyLock<FieldDescriptor> =
    std::sync::LazyLock::new(|| FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
        generation: super::Generation::None,
    });

impl Names for DeclaredNames<'_> {
    fn relation_name(&self, relation: RelationId) -> Option<&str> {
        self.0.relations.get(relation.0 as usize).map(|r| &*r.name)
    }

    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor> {
        let relation = self.0.relations.get(relation.0 as usize)?;
        // Statement field ids address the sealed numbering: on a closed
        // relation, 0 is the synthetic id and declared fields sit at +1.
        if relation.extension.is_some() {
            return match usize::from(field.0).checked_sub(1) {
                None => Some(&SYNTHETIC_ID),
                Some(idx) => relation.fields.get(idx),
            };
        }
        relation.fields.get(usize::from(field.0))
    }

    fn closed_target(&self, relation: RelationId, field: FieldId) -> Option<RelationId> {
        closed_target_of(
            self.0
                .statements
                .iter()
                .filter_map(|statement| match statement {
                    StatementDescriptor::Containment { source, target } => Some((source, target)),
                    StatementDescriptor::Functionality { .. }
                    | StatementDescriptor::Cardinality { .. } => None,
                }),
            |id| {
                self.0
                    .relations
                    .get(id.0 as usize)
                    .is_some_and(|r| r.extension.is_some())
            },
            relation,
            field,
        )
    }

    fn handle(&self, closed: RelationId, id: u64) -> Option<String> {
        let rows = self
            .0
            .relations
            .get(closed.0 as usize)?
            .extension
            .as_ref()?;
        usize::try_from(id)
            .ok()
            .and_then(|row| rows.get(row))
            .map(|row| row.handle.to_string())
    }
}

/// The lazy renderer behind both entry points.
struct Rendered<'a> {
    names: &'a dyn Names,
    statement: RenderedStatement<'a>,
    id: StatementId,
}

enum RenderedStatement<'a> {
    Key {
        relation: RelationId,
        projection: &'a [FieldId],
    },
    Containment {
        source: &'a Side,
        target: &'a Side,
        /// The `==` partner, if any — the sealed fact, never re-detected.
        mirror: Option<StatementId>,
    },
    Cardinality {
        source: &'a Side,
        lo: u64,
        hi: Option<u64>,
        target: &'a Side,
    },
}

impl fmt::Display for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.statement {
            RenderedStatement::Key {
                relation,
                projection,
            } => {
                side_parts(f, self.names, relation, projection, &[])?;
                write!(f, " -> ")?;
                relation_name(f, self.names, relation)
            }
            RenderedStatement::Containment {
                source,
                target,
                mirror,
            } => match mirror {
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
            // B-family, target-left, CANONICAL spellings only (the
            // canonical-utterance law, `docs/architecture/70-api.md`):
            // `lo = hi` is the `{n}` exact spelling (`{0}` the
            // exclusion), no ceiling is `{lo..*}`, else `{lo..hi}`.
            // Validation rejects the banned bound shapes (`{0..*}`,
            // `{1..*}`, inverted), so a sealed statement never renders
            // one; a rejected declaration renders its banned bounds as
            // written — the diagnostic must show the offense.
            RenderedStatement::Cardinality {
                source,
                lo,
                hi,
                target,
            } => {
                side(f, self.names, target)?;
                write!(f, " <={{")?;
                match hi {
                    Some(hi) if hi == lo => write!(f, "{lo}")?,
                    Some(hi) => write!(f, "{lo}..{hi}")?,
                    None => write!(f, "{lo}..*")?,
                }
                write!(f, "}} ")?;
                side(f, self.names, source)
            }
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

/// `Name(p1, p2 | s1 == lit1, s2 == {lit2, lit3})` — the one side shape;
/// the selection block only when σ is nonempty; a disjunctive binding
/// renders its literal set in braces.
fn side_parts(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
    projection: &[FieldId],
    selection: &[(FieldId, LiteralSet)],
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
        for (index, (field, literals)) in selection.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            field_name(f, names, relation, *field)?;
            write!(f, " == ")?;
            match literals {
                LiteralSet::One(value) => selection_literal(f, names, relation, *field, value)?,
                LiteralSet::Many(values) => {
                    write!(f, "{{")?;
                    for (value_index, value) in values.iter().enumerate() {
                        if value_index > 0 {
                            write!(f, ", ")?;
                        }
                        selection_literal(f, names, relation, *field, value)?;
                    }
                    write!(f, "}}")?;
                }
            }
        }
    }
    write!(f, ")")
}

/// One selection literal at its field position. A word at a
/// closed-reference position prints its handle (the macro's own
/// bare-handle spelling back out); an out-of-range word prints visibly
/// wrong as `Kind(7?)` — the `ir/render` convention, one fallback
/// everywhere.
fn selection_literal(
    f: &mut fmt::Formatter<'_>,
    names: &dyn Names,
    relation: RelationId,
    field: FieldId,
    value: &Value,
) -> fmt::Result {
    match (value, names.closed_target(relation, field)) {
        (Value::U64(word), Some(closed)) => {
            if let Some(handle) = names.handle(closed, *word) {
                write!(f, "{handle}")
            } else {
                relation_name(f, names, closed)?;
                write!(f, "({word}?)")
            }
        }
        _ => literal(f, value),
    }
}

/// The one selection-literal formatter: intervals render as their macro
/// form `start..end`, strings and bytes as escaped literals. Field-blind
/// — closed-reference words resolve to handles at the selection loop,
/// where the position is known.
fn literal(f: &mut fmt::Formatter<'_>, value: &Value) -> fmt::Result {
    match value {
        Value::Bool(v) => write!(f, "{v}"),
        Value::U64(v) => write!(f, "{v}"),
        Value::I64(v) => write!(f, "{v}"),
        // Unreachable through validated schemas (a mask is not a field
        // type, so no selection holds one); rendered anyway — Display
        // stays total on plain data.
        Value::AllenMask(mask) => write!(f, "allen({:#015b})", mask.bits()),
        Value::IntervalU64(interval) => write!(f, "{}..{}", interval.start(), interval.end()),
        Value::IntervalI64(interval) => write!(f, "{}..{}", interval.start(), interval.end()),
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
