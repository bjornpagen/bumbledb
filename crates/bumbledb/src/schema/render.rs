//! Statement rendering back to the `schema!` algebra notation
//! . Statements are anonymous —
//! their identity is their materialized-order id — and errors cite the
//! .
//! Rendering allocates; it runs only in `Display`/diagnostic contexts
//! (`crate::error`), never on a write or query path.
use std::collections::BTreeMap;
use std::fmt;

use super::{
    Bound, FieldDescriptor, FieldId, LiteralSet, RelationId, Schema, SchemaDescriptor, Side,
    StatementDescriptor, StatementId, StatementKind, StatementView, ValidateDescriptor, Value,
    ValueType, Weight,
};
use crate::error::{Direction, Violation, Violations};

/// One rejected commit's citation rendered as plain data — everything a
/// bindings layer needs to show (or prompt with) the rejection: the
/// statement's fingerprint-pinned id, its form tag, its canonical
/// spelling (the renderer is a bijection on legal statements, so the
/// spelling pastes back), the direction/count payloads where the form
/// carries them, and the offending facts as named decoded values
/// .
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedViolation {
    Functionality {
        statement: StatementId,
        spelling: String,
        facts: Vec<RenderedFact>,
    },
    Containment {
        statement: StatementId,
        spelling: String,
        direction: Direction,
        facts: Vec<RenderedFact>,
    },
    Capacity {
        statement: StatementId,
        spelling: String,
        measure: u128,
        facts: Vec<RenderedFact>,
    },
}

impl RenderedViolation {
    #[must_use]
    pub fn statement(&self) -> StatementId {
        match *self {
            Self::Functionality { statement, .. }
            | Self::Containment { statement, .. }
            | Self::Capacity { statement, .. } => statement,
        }
    }

    #[must_use]
    pub fn kind(&self) -> StatementKind {
        match self {
            Self::Functionality { .. } => StatementKind::Functionality,
            Self::Containment { .. } => StatementKind::Containment,
            Self::Capacity { .. } => StatementKind::Capacity,
        }
    }

    #[must_use]
    pub fn spelling(&self) -> &str {
        match self {
            Self::Functionality { spelling, .. }
            | Self::Containment { spelling, .. }
            | Self::Capacity { spelling, .. } => spelling,
        }
    }

    #[must_use]
    pub fn facts(&self) -> &[RenderedFact] {
        match self {
            Self::Functionality { facts, .. }
            | Self::Containment { facts, .. }
            | Self::Capacity { facts, .. } => facts,
        }
    }

    #[must_use]
    pub fn direction(&self) -> Option<Direction> {
        match self {
            Self::Containment { direction, .. } => Some(*direction),
            Self::Functionality { .. } | Self::Capacity { .. } => None,
        }
    }

    #[must_use]
    pub fn measure(&self) -> Option<u128> {
        match self {
            Self::Capacity { measure, .. } => Some(*measure),
            Self::Functionality { .. } | Self::Containment { .. } => None,
        }
    }
}

/// One offending fact with its names resolved: the relation name and one
/// `(field name, value)` pair per sealed field, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFact {
    pub relation: Box<str>,
    pub fields: Vec<(Box<str>, Value)>,
}

/// Renders a rejection's complete violation set as plain data — the
/// bindings-consumable form of a rejected [`crate::error::Admission`]:
/// per citation, the statement id, kind tag, canonical spelling, and the
/// offending facts as `(relation name, [(field name, value)])` rows,
/// ([`Violations::cited_facts`]). Pure over the descriptor: a foreign
/// host renders with its cached [`crate::Theory`] descriptor and no
/// database handle.
/// Total on plain data: unknown ids render as `relation#N` / `field#N`
/// # Panics
/// When a cited fact's field ordinal exceeds the id space (`u16`) —
/// admitted ([`crate::error::SchemaError::RelationTooManyColumns`] is
/// the typed rejection for such counts).
#[must_use]
pub fn render_rejection(
    descriptor: &SchemaDescriptor,
    violations: &Violations,
) -> Vec<RenderedViolation> {
    let names = DeclaredNames(descriptor);

    let schema = descriptor
        .clone()
        .validate()
        .expect("render_rejection is for an admitted theory");
    let materialized = descriptor.materialized_statements();
    let mirrors = super::validate::mirror_links(&materialized);
    violations
        .citations()
        .map(|(violation, cited)| {
            let statement = violation.statement_id(&schema);
            let spelling = if usize::from(statement.0) < materialized.len() {
                render_materialized(descriptor, &materialized, &mirrors, statement)
            } else {
                format!("statement#{}", statement.0)
            };
            let facts = cited
                .iter()
                .map(|fact| RenderedFact {
                    relation: names.relation_name(fact.relation()).map_or_else(
                        || format!("relation#{}", fact.relation().0).into(),
                        Box::from,
                    ),
                    fields: fact
                        .values()
                        .iter()
                        .enumerate()
                        .map(|(idx, value)| {
                            let field = FieldId(u16::try_from(idx).expect("field count fits u16"));
                            let name = names.field(fact.relation(), field).map_or_else(
                                || format!("field#{}", field.0).into(),
                                |descriptor| descriptor.name.clone(),
                            );
                            (name, value.clone())
                        })
                        .collect(),
                })
                .collect();
            match violation {
                Violation::Functionality { .. } => RenderedViolation::Functionality {
                    statement,
                    spelling,
                    facts,
                },
                Violation::Containment { direction, .. } => RenderedViolation::Containment {
                    statement,
                    spelling,
                    direction: *direction,
                    facts,
                },
                Violation::Capacity { measure, .. } => RenderedViolation::Capacity {
                    statement,
                    spelling,
                    measure: *measure,
                    facts,
                },
            }
        })
        .collect()
}

/// Renders one sealed statement in the exact macro notation: an FD as
/// `SavingsTerms(account) -> SavingsTerms`, a containment as
/// `Account(holder) <= Holder(id)` with any selection after `|`
/// (`Account(id | kind == Savings)`), and a bidirectional pair — read off
/// the sealed [`super::ContainmentStatement::pairing`] link — as
/// `==` once, in the pair's written orientation (both ids render the same
/// string), and a capacity statement B-family, target-left, in its one
/// canonical spelling (`Parent(id) <={1..3} Task(parent)`;
/// # Panics
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
            mirror: statement.mirror_id(schema),
        },
        StatementView::Capacity(_, statement) => RenderedStatement::Capacity {
            target: &statement.target,
            weight: statement.weight.to_weight(),
            lo: statement.lo,
            hi: statement.hi.to_bound(),
            source: &statement.source,
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
/// `relation#N`/`field#N` placeholders. The one-statement convenience:
/// materializes and pairs, then delegates to [`render_materialized`] —
/// # Panics
/// On an out-of-range id — schema errors carry ids produced by validating
/// this same descriptor.
#[must_use]
pub fn render_declared(descriptor: &SchemaDescriptor, id: StatementId) -> String {
    let materialized = descriptor.materialized_statements();
    let mirrors = super::validate::mirror_links(&materialized);
    render_materialized(descriptor, &materialized, &mirrors, id)
}

/// # Panics
pub(super) fn render_materialized(
    descriptor: &SchemaDescriptor,
    materialized: &[StatementDescriptor],
    mirrors: &BTreeMap<StatementId, StatementId>,
    id: StatementId,
) -> String {
    let index = usize::from(id.0);
    let statement = match &materialized[index] {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => RenderedStatement::Key {
            relation: *relation,
            projection,
        },
        StatementDescriptor::Containment { source, target } => RenderedStatement::Containment {
            source,
            target,

            mirror: mirrors.get(&id).copied(),
        },
        StatementDescriptor::Capacity {
            target,
            weight,
            lo,
            hi,
            source,
        } => RenderedStatement::Capacity {
            target,
            weight: *weight,
            lo: *lo,
            hi: *hi,
            source,
        },
    };
    Rendered {
        names: &DeclaredNames(descriptor),
        statement,
        id,
    }
    .to_string()
}

trait Names {
    fn relation_name(&self, relation: RelationId) -> Option<&str>;
    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor>;

    fn closed_target(&self, relation: RelationId, field: FieldId) -> Option<RelationId>;

    fn handle(&self, closed: RelationId, id: u64) -> Option<String>;
}

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
                    .is_some_and(|rel| rel.body().closed_rows().is_some())
            },
            relation,
            field,
        )
    }

    fn handle(&self, closed: RelationId, id: u64) -> Option<String> {
        let rows = self.0.relation_checked(closed)?.body().closed_rows()?;
        usize::try_from(id)
            .ok()
            .and_then(|row| rows.get(row))
            .map(|row| row.handle.to_string())
    }
}

struct DeclaredNames<'a>(&'a SchemaDescriptor);

static SYNTHETIC_ID: std::sync::LazyLock<FieldDescriptor> =
    std::sync::LazyLock::new(|| FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
    });

impl Names for DeclaredNames<'_> {
    fn relation_name(&self, relation: RelationId) -> Option<&str> {
        self.0.relations.get(relation.0 as usize).map(|r| &*r.name)
    }

    fn field(&self, relation: RelationId, field: FieldId) -> Option<&FieldDescriptor> {
        let relation = self.0.relations.get(relation.0 as usize)?;

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
                    | StatementDescriptor::Capacity { .. } => None,
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

struct Rendered<'a, N: Names + ?Sized> {
    names: &'a N,
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

        mirror: Option<StatementId>,
    },
    Capacity {
        target: &'a Side,
        weight: Weight,
        lo: u64,
        hi: Option<Bound>,
        source: &'a Side,
    },
}

impl<N: Names + ?Sized> fmt::Display for Rendered<'_, N> {
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

            RenderedStatement::Capacity {
                target,
                weight,
                lo,
                hi,
                source,
            } => {
                side(f, self.names, target)?;
                write!(f, " <=")?;
                match weight {
                    Weight::Unit => {}
                    Weight::Field(field) => {
                        write!(f, "[")?;
                        field_name(f, self.names, source.relation, field)?;
                        write!(f, "]")?;
                    }
                    Weight::DurationOf(field) => {
                        write!(f, "[Duration(")?;
                        field_name(f, self.names, source.relation, field)?;
                        write!(f, ")]")?;
                    }
                }
                write!(f, "{{")?;
                match hi {
                    Some(Bound::Lit(hi)) if hi == lo => write!(f, "{lo}")?,
                    Some(hi) => {
                        write!(f, "{lo}..")?;
                        bound(f, self.names, target.relation, &hi)?;
                    }
                    None => write!(f, "{lo}..*")?,
                }
                write!(f, "}} ")?;
                side(f, self.names, source)
            }
        }
    }
}

fn bound<N: Names + ?Sized>(
    f: &mut fmt::Formatter<'_>,
    names: &N,
    target: RelationId,
    bound: &Bound,
) -> fmt::Result {
    match bound {
        Bound::Lit(n) => write!(f, "{n}"),
        Bound::TargetField(field) => field_name(f, names, target, *field),
        Bound::TargetDuration(field) => {
            write!(f, "Duration(")?;
            field_name(f, names, target, *field)?;
            write!(f, ")")
        }
    }
}

fn relation_name<N: Names + ?Sized>(
    f: &mut fmt::Formatter<'_>,
    names: &N,
    relation: RelationId,
) -> fmt::Result {
    match names.relation_name(relation) {
        Some(name) => write!(f, "{name}"),
        None => write!(f, "relation#{}", relation.0),
    }
}

fn field_name<N: Names + ?Sized>(
    f: &mut fmt::Formatter<'_>,
    names: &N,
    relation: RelationId,
    field: FieldId,
) -> fmt::Result {
    match names.field(relation, field) {
        Some(descriptor) => write!(f, "{}", descriptor.name),
        None => write!(f, "field#{}", field.0),
    }
}

fn side<N: Names + ?Sized>(f: &mut fmt::Formatter<'_>, names: &N, side: &Side) -> fmt::Result {
    side_parts(f, names, side.relation, &side.projection, &side.selection)
}

fn side_parts<N: Names + ?Sized>(
    f: &mut fmt::Formatter<'_>,
    names: &N,
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

fn selection_literal<N: Names + ?Sized>(
    f: &mut fmt::Formatter<'_>,
    names: &N,
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

fn literal(f: &mut fmt::Formatter<'_>, value: &Value) -> fmt::Result {
    match value {
        Value::Bool(v) => write!(f, "{v}"),
        Value::U64(v) => write!(f, "{v}"),
        Value::I64(v) => write!(f, "{v}"),
        Value::F64(v) => write!(f, "f64:0x{:016x}", v.to_bits()),
        Value::Id128(id) => write!(f, "id128:{id}"),
        Value::IntervalU64(interval) => write!(f, "{}..{}", interval.start(), interval.end()),
        Value::IntervalI64(interval) => write!(f, "{}..{}", interval.start(), interval.end()),
        Value::IntervalF64(interval) => write!(
            f,
            "f64:0x{:016x}..f64:0x{:016x}",
            interval.start().to_bits(),
            interval.end().to_bits()
        ),
        Value::String(text) => {
            write!(f, "\"")?;
            for c in text.chars() {
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
