//! The theory's schema check at expansion: a declaration `bumbledb` would
//! refuse is a compile error at the tokens the refusal names.
use bumbledb_theory::schema::{
    FieldId, RelationId, RowIndex, SchemaDescriptor, SchemaError, StatementErrorKind, StatementId,
};
use proc_macro2::Span;

use super::lower::Spans;
use super::{Relation, Schema, Statement};
use crate::lex::{Error, Result};

pub(super) fn check(schema: &Schema, descriptor: &SchemaDescriptor, spans: &Spans) -> Result<()> {
    let Err(error) = bumbledb_theory::schema::check(descriptor) else {
        return Ok(());
    };
    let message = format!("schema!: {}", error.named(descriptor));
    let mut marked = error_spans(&error, schema, spans);
    if marked.is_empty() {
        marked.push(Span::call_site());
    }
    Err(Error::many(
        marked
            .into_iter()
            .map(|span| (span, message.clone()))
            .collect(),
    ))
}

/// Where a materialized statement was written: a closed relation's
/// implicit handle key, or a declared statement (`==` materializes two).
enum Origin {
    ClosedKey(usize),
    Declared(usize),
}

fn origin(schema: &Schema, statement: StatementId) -> Option<Origin> {
    let mut index = usize::from(statement.0);
    let closed: Vec<usize> = schema
        .relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.closed.is_some())
        .map(|(position, _)| position)
        .collect();
    if let Some(relation) = closed.get(index) {
        return Some(Origin::ClosedKey(*relation));
    }
    index -= closed.len();
    for (position, statement) in schema.statements.iter().enumerate() {
        let width = match statement {
            Statement::Containment {
                bidirectional: true,
                ..
            } => 2,
            _ => 1,
        };
        if index < width {
            return Some(Origin::Declared(position));
        }
        index -= width;
    }
    None
}

fn relation(schema: &Schema, id: RelationId) -> Option<&Relation> {
    schema.relations.get(id.0 as usize)
}

/// A field's declared name; a closed relation's field 0 is its handle `id`.
fn field_name(schema: &Schema, id: RelationId, field: FieldId) -> Option<String> {
    let relation = relation(schema, id)?;
    let index = usize::from(field.0);
    match (&relation.closed, index) {
        (Some(_), 0) => Some("id".to_owned()),
        (Some(_), index) => relation.fields.get(index - 1).map(|f| f.name.to_string()),
        (None, index) => relation.fields.get(index).map(|f| f.name.to_string()),
    }
}

/// A field's declaring token; a closed relation's handle `id` is declared by
/// its newtype.
fn field_span(schema: &Schema, id: RelationId, field: FieldId) -> Option<Span> {
    let relation = relation(schema, id)?;
    let index = usize::from(field.0);
    match (&relation.closed, index) {
        (Some(closed), 0) => Some(closed.newtype.span()),
        (Some(_), index) => relation.fields.get(index - 1).map(|f| f.name.span()),
        (None, index) => relation.fields.get(index).map(|f| f.name.span()),
    }
}

fn row_span(schema: &Schema, id: RelationId, row: RowIndex) -> Option<Span> {
    let closed = relation(schema, id)?.closed.as_ref()?;
    closed.rows.get(row.0).map(|row| row.handle.span())
}

fn error_spans(error: &SchemaError, schema: &Schema, spans: &Spans) -> Vec<Span> {
    let relation_span = |id: RelationId| relation(schema, id).map(|r| r.name.span());
    match error {
        SchemaError::TooManyRelations { .. } | SchemaError::TooManyStatements { .. } => Vec::new(),
        SchemaError::DuplicateRelationName { name } => schema
            .relations
            .iter()
            .filter(|relation| relation.name == **name)
            .skip(1)
            .map(|relation| relation.name.span())
            .collect(),
        SchemaError::DuplicateFieldName { relation: id, name } => {
            let Some(declared) = relation(schema, *id) else {
                return Vec::new();
            };
            let repeats = usize::from(declared.closed.is_none());
            declared
                .fields
                .iter()
                .filter(|field| field.name == **name)
                .skip(repeats)
                .map(|field| field.name.span())
                .collect()
        }
        SchemaError::FixedBytesWidthOutOfRange {
            relation: id,
            field,
            ..
        }
        | SchemaError::IntervalWidthOutOfRange {
            relation: id,
            field,
            ..
        }
        | SchemaError::StrOnClosedRelation {
            relation: id,
            field,
        } => field_span(schema, *id, *field).into_iter().collect(),
        SchemaError::RelationTooManyColumns { relation: id, .. }
        | SchemaError::EmptyExtension { relation: id }
        | SchemaError::ExtensionTooManyRows { relation: id, .. } => {
            relation_span(*id).into_iter().collect()
        }
        SchemaError::DuplicateExtensionHandle {
            relation: id,
            handle,
        } => relation(schema, *id)
            .and_then(|relation| relation.closed.as_ref())
            .map_or_else(Vec::new, |closed| {
                closed
                    .rows
                    .iter()
                    .filter(|row| row.handle == **handle)
                    .skip(1)
                    .map(|row| row.handle.span())
                    .collect()
            }),
        SchemaError::ExtensionArityMismatch {
            relation: id, row, ..
        }
        | SchemaError::ExtensionValueTypeMismatch {
            relation: id, row, ..
        }
        | SchemaError::ExtensionIntervalRay {
            relation: id, row, ..
        } => row_span(schema, *id, *row).into_iter().collect(),
        SchemaError::Statement { statement, kind } => match origin(schema, *statement) {
            None => Vec::new(),
            Some(Origin::ClosedKey(index)) => vec![schema.relations[index].name.span()],
            Some(Origin::Declared(index)) => statement_spans(schema, spans, index, kind),
        },
    }
}

/// The tokens of declared statement `index` that `kind` is about.
fn statement_spans(
    schema: &Schema,
    spans: &Spans,
    index: usize,
    kind: &StatementErrorKind,
) -> Vec<Span> {
    use StatementErrorKind as K;
    let field = |id: RelationId, field: FieldId| -> Vec<Span> {
        let (Some(relation), Some(name)) = (relation(schema, id), field_name(schema, id, field))
        else {
            return Vec::new();
        };
        spans.fields_of(index, &relation.name.to_string(), &name)
    };
    let relation_tokens = |id: RelationId| -> Vec<Span> {
        relation(schema, id).map_or_else(Vec::new, |relation| {
            spans.relations_of(index, &relation.name.to_string())
        })
    };
    let marked = match kind {
        K::DuplicateProjectionField {
            relation: id,
            field: f,
        }
        | K::DuplicateSelectionField {
            relation: id,
            field: f,
        }
        | K::DuplicateSelectionLiteral {
            relation: id,
            field: f,
        }
        | K::CapacityIntervalPosition {
            relation: id,
            field: f,
        }
        | K::CapacityWeightNotU64 {
            relation: id,
            field: f,
        }
        | K::CapacityWeightNotDuration {
            relation: id,
            field: f,
        }
        | K::CapacityBoundNotU64 {
            relation: id,
            field: f,
        }
        | K::CapacityBoundNotDuration {
            relation: id,
            field: f,
        }
        | K::CapacityDimensionMixing {
            relation: id,
            field: f,
        }
        | K::FunctionalityMultipleIntervals {
            relation: id,
            field: f,
        }
        | K::FunctionalityIntervalNotLast {
            relation: id,
            field: f,
        }
        | K::SelectedFieldProjected {
            relation: id,
            field: f,
        }
        | K::SelectionLiteralTypeMismatch {
            relation: id,
            field: f,
        } => field(*id, *f),
        K::DegenerateSelectionSet {
            relation: id,
            field: f,
            len,
        } => match field_name(schema, *id, *f) {
            Some(name) => spans.set_of(index, &name, *len),
            None => Vec::new(),
        },
        K::CapacityInvertedWindow { .. } => spans.window_of(index).into_iter().collect(),
        K::UnknownRelation { relation: id }
        | K::UnknownField { relation: id, .. }
        | K::EmptyProjection { relation: id }
        | K::ClosedContainmentInterval { relation: id }
        | K::ClosedTargetNotHandle { target: id, .. }
        | K::NoMatchingTargetKey { target: id, .. }
        | K::NoPointwiseTargetKey { target: id, .. } => relation_tokens(*id),
        K::ContainmentTypeMismatch { position } => match &schema.statements[index] {
            Statement::Containment { source, target, .. }
            | Statement::Capacity { source, target, .. } => [source, target]
                .into_iter()
                .filter_map(|side| side.projection.get(*position).map(proc_macro2::Ident::span))
                .collect(),
            Statement::Key { .. } => Vec::new(),
        },
        K::ContainmentArityMismatch { .. }
        | K::ClosedStatementRefuted { .. }
        | K::DuplicateFunctionality { .. }
        | K::DuplicateStatement { .. } => Vec::new(),
    };
    if marked.is_empty() {
        spans.statement_relations(index)
    } else {
        marked
    }
}
