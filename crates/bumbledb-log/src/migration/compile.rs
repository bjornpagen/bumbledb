//! Compile canonical plan data onto the CORE operators (C11/C05).
//!
//! Compilation is the complete native admission judgment over one plan:
//! schema binding (the supplied descriptors must fingerprint to the plan's
//! recorded schema ids), total source/target coverage over ordinary
//! relations, explicit destructive acknowledgements, exact field typing and
//! the terminal validate boundary. The output lowers every expression onto
//! [`bumbledb::ScalarExpr`] with `VarId(i)` bound to source field `i`; the
//! executor evaluates them through the core `ScalarEvaluator` — no second
//! evaluator, filter DSL or callback exists.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bumbledb::scalar::{ScalarError, ScalarExpr};
use bumbledb::schema::{
    FieldDescriptor, RelationId, Schema, SchemaDescriptor, ValidateDescriptor as _, ValueType,
    value_matches,
};
use bumbledb::{SchemaError, Value, VarId};

use crate::history::SchemaId;

use super::plan::{FieldMap, Loss, Operation, Plan, PlanExpr, StepLabel};

/// One lowered relation action, in plan operation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledAction {
    /// Read every source row, evaluate one expression per target field.
    Map {
        source: RelationId,
        target: RelationId,
        expressions: Vec<ScalarExpr>,
    },
    Empty {
        target: RelationId,
    },
    Drop {
        source: RelationId,
    },
    Seed {
        target: RelationId,
        rows: Vec<Box<[Value]>>,
    },
}

/// One checked, lowered plan with its validated core schemas.
#[derive(Debug)]
pub struct CompiledPlan {
    pub sequence: u64,
    pub label: StepLabel,
    pub from_id: SchemaId,
    pub to_id: SchemaId,
    pub from: Schema,
    pub to: Schema,
    pub actions: Vec<CompiledAction>,
}

/// Why a plan refused compilation. Every arm is actionable at generation
/// review time; none is a runtime guess.
#[derive(Debug)]
pub enum CompileError {
    /// The supplied descriptor does not fingerprint to the plan's schema id.
    SchemaIdMismatch {
        which: &'static str,
    },
    /// The core refused the descriptor itself.
    Schema(SchemaError),
    UnknownRelation {
        name: Box<str>,
    },
    /// Closed relations are sealed schema axioms; data operations refuse.
    ClosedRelation {
        name: Box<str>,
    },
    /// A source relation is consumed twice, or a target produced twice.
    DuplicateCoverage {
        name: Box<str>,
    },
    /// An ordinary source relation is neither mapped nor explicitly dropped.
    MissingSourceCoverage {
        name: Box<str>,
    },
    /// An ordinary target relation is neither mapped-to nor created empty.
    MissingTargetCoverage {
        name: Box<str>,
    },
    /// A seed names a target before the operation that produces it.
    SeedBeforeProduce {
        name: Box<str>,
    },
    UnknownField {
        relation: Box<str>,
        field: Box<str>,
    },
    /// Field maps must cover every target field exactly once, in target
    /// declaration order — the one canonical spelling.
    FieldCoverage {
        relation: Box<str>,
    },
    /// An expression's checked type does not equal the target field type.
    Type {
        relation: Box<str>,
        field: Box<str>,
        error: ScalarError,
    },
    /// A literal/seed value does not match the target field type.
    ValueShape {
        relation: Box<str>,
    },
    /// A seed row's arity does not match the target relation.
    SeedArity {
        relation: Box<str>,
    },
    /// Data loss without its explicit acknowledgement.
    MissingLossAck {
        relation: Box<str>,
        field: Option<Box<str>>,
    },
    /// An acknowledgement that acknowledges nothing (stale intent).
    StaleLossAck {
        relation: Box<str>,
        field: Option<Box<str>>,
    },
    /// The final operation must be `validate-schema` naming `toSchemaId`.
    MissingFinalValidate,
    /// `validate-schema` anywhere else in the plan.
    MisplacedValidate,
    WrongFinalValidate,
}

impl From<SchemaError> for CompileError {
    fn from(error: SchemaError) -> Self {
        Self::Schema(error)
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plan compilation: {self:?}")
    }
}

impl std::error::Error for CompileError {}

struct RelationRoster {
    /// name -> (id, fields, closed)
    by_name: BTreeMap<Box<str>, (RelationId, Vec<FieldDescriptor>, bool)>,
}

impl RelationRoster {
    fn new(descriptor: &SchemaDescriptor) -> Self {
        let mut by_name = BTreeMap::new();
        for (index, relation) in descriptor.relations.iter().enumerate() {
            by_name.insert(
                relation.name.clone(),
                (
                    RelationId(u32::try_from(index).expect("validated relation count")),
                    relation.fields.clone(),
                    relation.extension.is_some(),
                ),
            );
        }
        Self { by_name }
    }

    fn ordinary(&self, name: &str) -> Result<(RelationId, &[FieldDescriptor]), CompileError> {
        match self.by_name.get(name) {
            None => Err(CompileError::UnknownRelation { name: name.into() }),
            Some((_, _, true)) => Err(CompileError::ClosedRelation { name: name.into() }),
            Some((id, fields, false)) => Ok((*id, fields)),
        }
    }

    fn ordinary_names(&self) -> impl Iterator<Item = &str> {
        self.by_name
            .iter()
            .filter(|(_, (_, _, closed))| !closed)
            .map(|(name, _)| name.as_ref())
    }
}

/// Compile one plan against its actual schema descriptors.
/// # Errors
/// The complete typed refusal roster above; nothing is guessed or repaired.
#[expect(
    clippy::too_many_lines,
    reason = "one bounded compile pass over the closed operation roster"
)]
pub fn compile(
    plan: &Plan,
    from_descriptor: &SchemaDescriptor,
    to_descriptor: &SchemaDescriptor,
) -> Result<CompiledPlan, CompileError> {
    let from = from_descriptor.clone().validate()?;
    let from_id = SchemaId(bumbledb::schema::fingerprint::fingerprint(&from).0);
    if from_id != plan.from_schema {
        return Err(CompileError::SchemaIdMismatch { which: "from" });
    }
    let to = to_descriptor.clone().validate()?;
    let to_id = SchemaId(bumbledb::schema::fingerprint::fingerprint(&to).0);
    if to_id != plan.to_schema {
        return Err(CompileError::SchemaIdMismatch { which: "to" });
    }

    // The terminal validate boundary is mandatory and unique.
    match plan.operations.last() {
        Some(Operation::ValidateSchema { schema }) => {
            if *schema != plan.to_schema {
                return Err(CompileError::WrongFinalValidate);
            }
        }
        _ => return Err(CompileError::MissingFinalValidate),
    }

    let sources = RelationRoster::new(from_descriptor);
    let targets = RelationRoster::new(to_descriptor);

    let mut consumed: BTreeSet<Box<str>> = BTreeSet::new();
    let mut produced: BTreeSet<Box<str>> = BTreeSet::new();
    let mut losses: Vec<Loss> = Vec::new();
    let mut actions = Vec::new();

    let body = &plan.operations[..plan.operations.len() - 1];
    for operation in body {
        match operation {
            Operation::ValidateSchema { .. } => {
                return Err(CompileError::MisplacedValidate);
            }
            Operation::MapRelation {
                source,
                target,
                fields,
            } => {
                let (source_id, source_fields) = sources.ordinary(source)?;
                let (target_id, target_fields) = targets.ordinary(target)?;
                if !consumed.insert(source.clone()) {
                    return Err(CompileError::DuplicateCoverage {
                        name: source.clone(),
                    });
                }
                if !produced.insert(target.clone()) {
                    return Err(CompileError::DuplicateCoverage {
                        name: target.clone(),
                    });
                }
                let (expressions, referenced) =
                    compile_fields(source, source_fields, target, target_fields, fields)?;
                // Unreferenced source fields are data loss and need explicit
                // acknowledgement.
                for (index, field) in source_fields.iter().enumerate() {
                    if !referenced.contains(&index) {
                        losses.push(Loss {
                            relation: source.clone(),
                            field: Some(field.name.clone()),
                        });
                    }
                }
                actions.push(CompiledAction::Map {
                    source: source_id,
                    target: target_id,
                    expressions,
                });
            }
            Operation::EmptyRelation { target } => {
                let (target_id, _) = targets.ordinary(target)?;
                if !produced.insert(target.clone()) {
                    return Err(CompileError::DuplicateCoverage {
                        name: target.clone(),
                    });
                }
                actions.push(CompiledAction::Empty { target: target_id });
            }
            Operation::DropRelation { source } => {
                let (source_id, _) = sources.ordinary(source)?;
                if !consumed.insert(source.clone()) {
                    return Err(CompileError::DuplicateCoverage {
                        name: source.clone(),
                    });
                }
                losses.push(Loss {
                    relation: source.clone(),
                    field: None,
                });
                actions.push(CompiledAction::Drop { source: source_id });
            }
            Operation::Seed { target, rows } => {
                let (target_id, target_fields) = targets.ordinary(target)?;
                if !produced.contains(target) {
                    return Err(CompileError::SeedBeforeProduce {
                        name: target.clone(),
                    });
                }
                for row in rows {
                    if row.len() != target_fields.len() {
                        return Err(CompileError::SeedArity {
                            relation: target.clone(),
                        });
                    }
                    for (value, field) in row.iter().zip(target_fields) {
                        if value_matches(value, &field.value_type).is_err() {
                            return Err(CompileError::ValueShape {
                                relation: target.clone(),
                            });
                        }
                    }
                }
                actions.push(CompiledAction::Seed {
                    target: target_id,
                    rows: rows.clone(),
                });
            }
        }
    }

    // Total coverage over ordinary relations, both directions.
    for name in sources.ordinary_names() {
        if !consumed.contains(name) {
            return Err(CompileError::MissingSourceCoverage { name: name.into() });
        }
    }
    for name in targets.ordinary_names() {
        if !produced.contains(name) {
            return Err(CompileError::MissingTargetCoverage { name: name.into() });
        }
    }

    // Every actual loss needs exactly one acknowledgement; every
    // acknowledgement must acknowledge an actual loss.
    for loss in &losses {
        if !plan
            .destructive
            .iter()
            .any(|ack| ack.relation == loss.relation && ack.field == loss.field)
        {
            return Err(CompileError::MissingLossAck {
                relation: loss.relation.clone(),
                field: loss.field.clone(),
            });
        }
    }
    for ack in &plan.destructive {
        if !losses
            .iter()
            .any(|loss| loss.relation == ack.relation && loss.field == ack.field)
        {
            return Err(CompileError::StaleLossAck {
                relation: ack.relation.clone(),
                field: ack.field.clone(),
            });
        }
    }

    Ok(CompiledPlan {
        sequence: plan.sequence,
        label: plan.label.clone(),
        from_id,
        to_id,
        from,
        to,
        actions,
    })
}

/// Lower one map's field expressions, checking full coverage in target
/// declaration order and exact result typing. Returns the lowered
/// expressions and the set of referenced source field ordinals.
fn compile_fields(
    source_name: &str,
    source_fields: &[FieldDescriptor],
    target_name: &str,
    target_fields: &[FieldDescriptor],
    fields: &[FieldMap],
) -> Result<(Vec<ScalarExpr>, BTreeSet<usize>), CompileError> {
    if fields.len() != target_fields.len() {
        return Err(CompileError::FieldCoverage {
            relation: target_name.into(),
        });
    }
    let mut referenced = BTreeSet::new();
    let mut expressions = Vec::with_capacity(fields.len());
    for (field, descriptor) in fields.iter().zip(target_fields) {
        if field.target != descriptor.name {
            // One canonical spelling: target declaration order, no
            // permutation aliases for the same plan meaning.
            return Err(CompileError::FieldCoverage {
                relation: target_name.into(),
            });
        }
        let lowered = lower(
            &field.expression,
            source_name,
            source_fields,
            &mut referenced,
        )?;
        check_type(
            &lowered,
            &field.expression,
            source_fields,
            &descriptor.value_type,
            target_name,
            &descriptor.name,
        )?;
        expressions.push(lowered);
    }
    Ok((expressions, referenced))
}

fn lower(
    expr: &PlanExpr,
    source_name: &str,
    source_fields: &[FieldDescriptor],
    referenced: &mut BTreeSet<usize>,
) -> Result<ScalarExpr, CompileError> {
    fn boxed(
        inner: &PlanExpr,
        source_name: &str,
        source_fields: &[FieldDescriptor],
        referenced: &mut BTreeSet<usize>,
    ) -> Result<Box<ScalarExpr>, CompileError> {
        Ok(Box::new(lower(
            inner,
            source_name,
            source_fields,
            referenced,
        )?))
    }
    Ok(match expr {
        PlanExpr::Field(name) => {
            let index = source_fields
                .iter()
                .position(|field| field.name.as_ref() == name.as_ref())
                .ok_or_else(|| CompileError::UnknownField {
                    relation: source_name.into(),
                    field: name.clone(),
                })?;
            referenced.insert(index);
            ScalarExpr::Var(VarId(
                u16::try_from(index).expect("validated field count fits u16"),
            ))
        }
        PlanExpr::Literal(value) => ScalarExpr::Literal(value.clone()),
        PlanExpr::Negate(inner) => {
            ScalarExpr::Negate(boxed(inner, source_name, source_fields, referenced)?)
        }
        PlanExpr::Add(left, right) => ScalarExpr::Add(
            boxed(left, source_name, source_fields, referenced)?,
            boxed(right, source_name, source_fields, referenced)?,
        ),
        PlanExpr::Subtract(left, right) => ScalarExpr::Subtract(
            boxed(left, source_name, source_fields, referenced)?,
            boxed(right, source_name, source_fields, referenced)?,
        ),
        PlanExpr::Multiply(left, right) => ScalarExpr::Multiply(
            boxed(left, source_name, source_fields, referenced)?,
            boxed(right, source_name, source_fields, referenced)?,
        ),
        PlanExpr::Divide(left, right) => ScalarExpr::Divide(
            boxed(left, source_name, source_fields, referenced)?,
            boxed(right, source_name, source_fields, referenced)?,
        ),
        PlanExpr::Cast { kind, expr } => ScalarExpr::Cast {
            kind: *kind,
            expr: boxed(expr, source_name, source_fields, referenced)?,
        },
        PlanExpr::IsNaN(inner) => {
            ScalarExpr::IsNaN(boxed(inner, source_name, source_fields, referenced)?)
        }
        PlanExpr::IsFinite(inner) => {
            ScalarExpr::IsFinite(boxed(inner, source_name, source_fields, referenced)?)
        }
    })
}

/// Exact result typing. Direct copies and root literals may be ANY value
/// type (checked structurally); operator trees are typed by the core's own
/// `result_type`, so migration arithmetic can never mean something query
/// arithmetic would not.
fn check_type(
    lowered: &ScalarExpr,
    original: &PlanExpr,
    source_fields: &[FieldDescriptor],
    expected: &ValueType,
    target_name: &str,
    target_field: &str,
) -> Result<(), CompileError> {
    let type_error = |error: ScalarError| CompileError::Type {
        relation: target_name.into(),
        field: target_field.into(),
        error,
    };
    match original {
        PlanExpr::Field(_) => {
            let ScalarExpr::Var(var) = lowered else {
                unreachable!("a field lowers to a var");
            };
            let actual = &source_fields[usize::from(var.0)].value_type;
            if actual == expected {
                Ok(())
            } else {
                Err(type_error(ScalarError::TypeMismatch))
            }
        }
        PlanExpr::Literal(value) => {
            value_matches(value, expected).map_err(|_| type_error(ScalarError::TypeMismatch))
        }
        _ => {
            let actual = lowered
                .result_type(|var| {
                    source_fields
                        .get(usize::from(var.0))
                        .map(|field| field.value_type)
                })
                .map_err(type_error)?;
            if actual == *expected {
                Ok(())
            } else {
                Err(type_error(ScalarError::TypeMismatch))
            }
        }
    }
}
