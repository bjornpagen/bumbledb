//! The parse as a `SchemaSpec`, lowered by the theory's one resolver.
//! Literals are typed against their declared field here; every resolver
//! issue maps back to the tokens that spelled it.
use std::collections::BTreeMap;

use bumbledb_theory::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FieldSpec, LiteralAt, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SchemaSpec, SideSpec, SpecIssue, StatementSide, StatementSpec,
    WeightSpec,
};
use bumbledb_theory::schema::{FixedIntervalElement, IntervalElement, SchemaDescriptor, ValueType};
use bumbledb_theory::{F64, Interval, Uuid, Value};
use proc_macro2::{Ident, Span};

use super::{Bound, FieldTy, Literal, Literals, Relation, Schema, Side, Statement, Weight, Window};
use crate::lex::{Error, Int, LitKind, Result, fail};

/// The descriptor, with the spans that map the resolver's and the
/// checker's ids and names back to tokens.
pub(super) fn descriptor(schema: &Schema) -> Result<(SchemaDescriptor, Spans)> {
    let mut spans = Spans::default();
    let spec = SchemaSpec {
        relations: lower_relations(schema, &mut spans)?,
        statements: lower_statements(schema, &mut spans)?,
    };
    let descriptor = spec
        .descriptor()
        .map_err(|error| issue_errors(error.issues(), &spans))?;
    Ok((descriptor, spans))
}

/// Source spans keyed the way the resolver's issues address names.
#[derive(Default)]
pub(super) struct Spans {
    relation_names: BTreeMap<usize, Span>,
    relations: BTreeMap<(usize, String), Vec<Span>>,
    fields: BTreeMap<(usize, String, String), Vec<Span>>,
    windows: BTreeMap<usize, Span>,
    weights: BTreeMap<usize, Span>,
    sets: BTreeMap<(usize, String, usize), Vec<Span>>,
    literals: BTreeMap<LiteralAt, Span>,
    newtypes: BTreeMap<usize, Span>,
}

impl Spans {
    fn relation(&mut self, statement: usize, relation: &Ident) {
        self.relations
            .entry((statement, relation.to_string()))
            .or_default()
            .push(relation.span());
    }

    fn field(&mut self, statement: usize, relation: &Ident, field: &Ident) {
        self.fields
            .entry((statement, relation.to_string(), field.to_string()))
            .or_default()
            .push(field.span());
    }

    pub(super) fn fields_of(&self, statement: usize, relation: &str, field: &str) -> Vec<Span> {
        self.fields
            .get(&(statement, relation.to_owned(), field.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn relations_of(&self, statement: usize, relation: &str) -> Vec<Span> {
        self.relations
            .get(&(statement, relation.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    /// Every relation token of one statement.
    pub(super) fn statement_relations(&self, statement: usize) -> Vec<Span> {
        self.relations
            .range((statement, String::new())..)
            .take_while(|((at, _), _)| *at == statement)
            .flat_map(|(_, spans)| spans.iter().copied())
            .collect()
    }

    pub(super) fn set_of(&self, statement: usize, field: &str, len: usize) -> Vec<Span> {
        self.sets
            .get(&(statement, field.to_owned(), len))
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn window_of(&self, statement: usize) -> Option<Span> {
        self.windows.get(&statement).copied()
    }
}

fn name(ident: &Ident) -> Box<str> {
    ident.to_string().into()
}

fn value_type(ty: FieldTy) -> ValueType {
    match ty {
        FieldTy::Bool => ValueType::Bool,
        FieldTy::U64 => ValueType::U64,
        FieldTy::I64 => ValueType::I64,
        FieldTy::F64 => ValueType::F64,
        FieldTy::Uuid => ValueType::Uuid,
        FieldTy::Str => ValueType::String,
        FieldTy::FixedBytes(len) => ValueType::FixedBytes { len },
        FieldTy::Interval(element) => ValueType::Interval { element },
        FieldTy::FixedInterval(element, width) => ValueType::FixedInterval { element, width },
    }
}

fn lower_relations(schema: &Schema, spans: &mut Spans) -> Result<Vec<RelationSpec>> {
    let mut relations = Vec::with_capacity(schema.relations.len());
    for (rel_idx, relation) in schema.relations.iter().enumerate() {
        spans.relation_names.insert(rel_idx, relation.name.span());
        let closed = match &relation.closed {
            None => None,
            Some(closed) => {
                spans.newtypes.insert(rel_idx, closed.newtype.span());
                let mut rows = Vec::with_capacity(closed.rows.len());
                for (row_idx, row) in closed.rows.iter().enumerate() {
                    let mut values = Vec::with_capacity(row.values.len());
                    for (column, (field, literal)) in
                        relation.fields.iter().zip(&row.values).enumerate()
                    {
                        if let Literal::Handle(handle) = literal {
                            let at = LiteralAt::Row {
                                relation: rel_idx,
                                row: row_idx,
                                column,
                            };
                            spans.literals.insert(at, handle.span());
                        }
                        values.push(typed(relation, &field.name, field.ty, literal)?);
                    }
                    rows.push(RowSpec {
                        handle: name(&row.handle),
                        values,
                    });
                }
                Some(ClosedSpec {
                    newtype: name(&closed.newtype),
                    rows,
                })
            }
        };
        relations.push(RelationSpec {
            name: name(&relation.name),
            fields: relation
                .fields
                .iter()
                .map(|field| FieldSpec {
                    name: name(&field.name),
                    value_type: value_type(field.ty),
                    newtype: field.newtype.as_ref().map(name),
                })
                .collect(),
            closed,
        });
    }
    Ok(relations)
}

fn lower_statements(schema: &Schema, spans: &mut Spans) -> Result<Vec<StatementSpec>> {
    let mut statements = Vec::with_capacity(schema.statements.len());
    for (index, statement) in schema.statements.iter().enumerate() {
        statements.push(match statement {
            Statement::Key {
                relation,
                projection,
            } => {
                spans.relation(index, relation);
                for field in projection {
                    spans.field(index, relation, field);
                }
                StatementSpec::Fd {
                    relation: name(relation),
                    projection: projection.iter().map(name).collect(),
                }
            }
            Statement::Containment {
                source,
                target,
                bidirectional,
            } => StatementSpec::Containment {
                source: lower_side(schema, index, StatementSide::Source, source, spans)?,
                target: lower_side(schema, index, StatementSide::Target, target, spans)?,
                bidirectional: *bidirectional,
            },
            Statement::Capacity {
                target,
                weight,
                window,
                window_span,
                source,
            } => {
                spans.windows.insert(index, *window_span);
                let weight = match weight {
                    Weight::Unit => WeightSpec::Unit,
                    Weight::Field(field) => {
                        spans.weights.insert(index, field.span());
                        spans.field(index, &source.relation, field);
                        WeightSpec::Field(name(field))
                    }
                    Weight::Duration(field) => {
                        spans.weights.insert(index, field.span());
                        spans.field(index, &source.relation, field);
                        WeightSpec::Duration(name(field))
                    }
                };
                let mut bound = |bound: &Bound| match bound {
                    Bound::Lit(value) => BoundSpec::Lit(*value),
                    Bound::Field(field) => {
                        spans.field(index, &target.relation, field);
                        BoundSpec::Field(name(field))
                    }
                    Bound::Duration(field) => {
                        spans.field(index, &target.relation, field);
                        BoundSpec::Duration(name(field))
                    }
                };
                let window = match window {
                    Window::Exact(lo) => CapacityWindowSpec::Exact(bound(lo)),
                    Window::Floor(lo) => CapacityWindowSpec::Floor(bound(lo)),
                    Window::Range(lo, hi) => CapacityWindowSpec::Range {
                        lo: bound(lo),
                        hi: bound(hi),
                    },
                };
                StatementSpec::Capacity {
                    target: lower_side(schema, index, StatementSide::Target, target, spans)?,
                    weight,
                    window,
                    source: lower_side(schema, index, StatementSide::Source, source, spans)?,
                }
            }
        });
    }
    Ok(statements)
}

fn lower_side(
    schema: &Schema,
    statement: usize,
    which: StatementSide,
    side: &Side,
    spans: &mut Spans,
) -> Result<SideSpec> {
    spans.relation(statement, &side.relation);
    for field in &side.projection {
        spans.field(statement, &side.relation, field);
    }
    let mut selection = Vec::with_capacity(side.selection.len());
    for (binding_idx, binding) in side.selection.iter().enumerate() {
        spans.field(statement, &side.relation, &binding.field);
        let ty = schema.field_ty(&side.relation, &binding.field);
        let mut lower = |literal_idx: usize, literal: &Literal| {
            if let Literal::Handle(handle) = literal {
                let at = LiteralAt::Selection {
                    statement,
                    side: which,
                    binding: binding_idx,
                    literal: literal_idx,
                };
                spans.literals.insert(at, handle.span());
            }
            match (ty, literal) {
                (Some(ty), _) => typed_on(&side.relation, &binding.field, ty, literal),
                (None, Literal::Handle(handle)) => Ok(LiteralSpec::Handle(name(handle))),
                (None, Literal::Lit(_)) => Ok(LiteralSpec::Value(Value::U64(0))),
            }
        };
        let literals = match &binding.literals {
            Literals::One(literal) => LiteralSetSpec::One(lower(0, literal)?),
            Literals::Many(many, span) => {
                spans
                    .sets
                    .entry((statement, binding.field.to_string(), many.len()))
                    .or_default()
                    .push(*span);
                LiteralSetSpec::Many(
                    many.iter()
                        .enumerate()
                        .map(|(literal_idx, literal)| lower(literal_idx, literal))
                        .collect::<Result<_>>()?,
                )
            }
        };
        selection.push((name(&binding.field), literals));
    }
    Ok(SideSpec {
        relation: name(&side.relation),
        projection: side.projection.iter().map(name).collect(),
        selection,
    })
}

fn typed(
    relation: &Relation,
    field: &Ident,
    ty: FieldTy,
    literal: &Literal,
) -> Result<LiteralSpec> {
    typed_on(&relation.name, field, ty, literal)
}

/// A literal as a value of `relation.field`'s declared type.
fn typed_on(
    relation: &Ident,
    field: &Ident,
    ty: FieldTy,
    literal: &Literal,
) -> Result<LiteralSpec> {
    let lit = match literal {
        Literal::Handle(handle) => return Ok(LiteralSpec::Handle(name(handle))),
        Literal::Lit(lit) => lit,
    };
    let mismatch = || {
        fail(
            lit.span,
            format!("schema!: the literal does not fit `{relation}.{field}`'s declared type"),
        )
    };
    let empty = || {
        fail(
            lit.span,
            format!(
                "schema!: the interval literal for `{relation}.{field}` is empty — \
                 `start..end` is half-open, start < end"
            ),
        )
    };
    let u64_pair = |start: &Int, end: &Int| Some((start.to_u64()?, end.to_u64()?));
    let i64_pair = |start: &Int, end: &Int| Some((start.to_i64()?, end.to_i64()?));
    let value = match (ty, &lit.kind) {
        (FieldTy::Bool, LitKind::Bool(value)) => Value::Bool(*value),
        (FieldTy::U64, LitKind::Int(int)) => match int.to_u64() {
            Some(value) => Value::U64(value),
            None => return mismatch(),
        },
        (FieldTy::I64, LitKind::Int(int)) => match int.to_i64() {
            Some(value) => Value::I64(value),
            None => return mismatch(),
        },
        (FieldTy::F64, LitKind::Float(value)) => Value::F64(*value),
        (FieldTy::Uuid, LitKind::Bytes(bytes)) => match <[u8; 16]>::try_from(bytes.as_slice()) {
            Ok(bytes) => Value::Uuid(Uuid::from_bytes(bytes)),
            Err(_) => return mismatch(),
        },
        (FieldTy::Uuid, LitKind::Str(text)) => match Uuid::parse_str(text) {
            Ok(id) => Value::Uuid(id),
            Err(_) => return mismatch(),
        },
        (FieldTy::Str, LitKind::Str(text)) => Value::String(text.as_str().into()),
        (FieldTy::FixedBytes(len), LitKind::Bytes(bytes)) => {
            if bytes.len() != usize::from(len) {
                return mismatch();
            }
            Value::FixedBytes(bytes.as_slice().into())
        }
        (FieldTy::Interval(IntervalElement::U64), LitKind::IntInterval(start, end)) => {
            let Some((start, end)) = u64_pair(start, end) else {
                return mismatch();
            };
            match Interval::<u64>::new(start, end) {
                Some(interval) => Value::IntervalU64(interval),
                None => return empty(),
            }
        }
        (FieldTy::Interval(IntervalElement::I64), LitKind::IntInterval(start, end)) => {
            let Some((start, end)) = i64_pair(start, end) else {
                return mismatch();
            };
            match Interval::<i64>::new(start, end) {
                Some(interval) => Value::IntervalI64(interval),
                None => return empty(),
            }
        }
        (FieldTy::Interval(IntervalElement::F64), LitKind::FloatInterval(start, end)) => {
            match Interval::<F64>::new(*start, *end) {
                Some(interval) => Value::IntervalF64(interval),
                None => return empty(),
            }
        }
        (
            FieldTy::FixedInterval(FixedIntervalElement::U64, width),
            LitKind::IntInterval(start, end),
        ) => {
            let Some((start, end)) = u64_pair(start, end) else {
                return mismatch();
            };
            let Some(interval) = Interval::<u64>::new(start, end) else {
                return empty();
            };
            if interval.end() - interval.start() != width || interval.is_ray() {
                return mismatch();
            }
            Value::IntervalU64(interval)
        }
        (
            FieldTy::FixedInterval(FixedIntervalElement::I64, width),
            LitKind::IntInterval(start, end),
        ) => {
            let Some((start, end)) = i64_pair(start, end) else {
                return mismatch();
            };
            let Some(interval) = Interval::<i64>::new(start, end) else {
                return empty();
            };
            if interval.end().abs_diff(interval.start()) != width || interval.is_ray() {
                return mismatch();
            }
            Value::IntervalI64(interval)
        }
        _ => return mismatch(),
    };
    Ok(LiteralSpec::Value(value))
}

/// Each distinct issue once, as a `compile_error!` at every token it names.
fn issue_errors(issues: &[SpecIssue], spans: &Spans) -> Error {
    let mut diagnostics = Vec::new();
    let mut seen: Vec<&SpecIssue> = Vec::new();
    for issue in issues {
        if seen.contains(&issue) {
            continue;
        }
        seen.push(issue);
        let message = format!("schema!: {issue}");
        for span in issue_spans(issue, spans) {
            diagnostics.push((span, message.clone()));
        }
    }
    Error::many(diagnostics)
}

fn issue_spans(issue: &SpecIssue, spans: &Spans) -> Vec<Span> {
    let many =
        |found: Option<&Vec<Span>>| found.cloned().unwrap_or_else(|| vec![Span::call_site()]);
    let one = |found: Option<&Span>| vec![found.copied().unwrap_or_else(Span::call_site)];
    let field = |statement: usize, relation: &str, field: &str| {
        spans
            .fields
            .get(&(statement, relation.to_owned(), field.to_owned()))
            .cloned()
            .unwrap_or_default()
    };
    match issue {
        SpecIssue::UnknownRelation {
            statement,
            relation,
        } => many(spans.relations.get(&(*statement, relation.to_string()))),
        SpecIssue::UnknownField {
            statement,
            relation,
            field: name,
        } => {
            let marked = field(*statement, relation, name);
            if marked.is_empty() {
                vec![Span::call_site()]
            } else {
                marked
            }
        }
        SpecIssue::NotAHandleField { at, .. } | SpecIssue::UnknownHandle { at, .. } => {
            one(spans.literals.get(at))
        }
        SpecIssue::RowArityExcess { relation, .. }
        | SpecIssue::RelationTooManyFields { relation, .. } => {
            one(spans.relation_names.get(relation))
        }
        SpecIssue::DuplicateHandleNewtype {
            second_relation, ..
        } => one(spans.newtypes.get(second_relation)),
        SpecIssue::CapacityInverted { statement, .. }
        | SpecIssue::CapacityDependentFloor { statement }
        | SpecIssue::BoundPathRefused { statement, .. } => one(spans.windows.get(statement)),
        SpecIssue::WeightPathRefused { statement, .. } => one(spans.weights.get(statement)),
        SpecIssue::DegenerateLiteralSet {
            statement,
            field,
            len,
        } => many(spans.sets.get(&(*statement, field.to_string(), *len))),
        SpecIssue::StatementNewtypeMismatch {
            statement,
            source,
            target,
            ..
        } => {
            let mut marked = field(*statement, &source.relation, &source.field);
            marked.extend(field(*statement, &target.relation, &target.field));
            if marked.is_empty() {
                vec![Span::call_site()]
            } else {
                marked
            }
        }
    }
}
