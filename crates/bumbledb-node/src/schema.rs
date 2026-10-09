//! Synchronous schema compilation: one spec JSON in, one validated schema
//! handle (or a structured diagnostic) out. The engine is the only judge.
use std::fmt::Debug;
use std::sync::Arc;

use bumbledb::schema::{
    Bound, FieldDescriptor, IntervalElement, SealedField, Side, SpecIssue, StatementDescriptor,
    ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{SchemaDescriptor, SchemaError, SchemaSpec, StatementErrorKind, StatementId};
use napi::bindgen_prelude::External;
use napi_derive::napi;

use crate::input::schema::SchemaSpecIn;
use crate::input::{Malformed, decode};
use crate::marshal::{NamedValueOut, ValueOut};

/// A sealed field roster: the relation name and its sealed field slots
/// (a closed relation's synthetic `id` first).
pub(crate) struct SealedRoster {
    pub(crate) name: Box<str>,
    pub(crate) fields: Vec<FieldDescriptor>,
}

/// A validated schema plus everything the bridge derives from it once:
/// materialized statements, sealed rosters and the spec's newtype names.
pub struct SchemaHandle {
    pub(crate) descriptor: SchemaDescriptor,
    pub(crate) schema: Arc<bumbledb::schema::Schema>,
    pub(crate) statements: Vec<StatementDescriptor>,
    pub(crate) rosters: Vec<SealedRoster>,
    newtypes: Vec<Vec<Option<Box<str>>>>,
    pub(crate) fingerprint: String,
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

/// The leading identifier of a derived `Debug` rendering: the variant name.
pub(crate) fn code(value: &impl Debug) -> String {
    let rendered = format!("{value:?}");
    rendered
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn ordinal(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[napi(object, object_from_js = false)]
pub struct StatementCite {
    pub id: u32,
    pub spelling: String,
}

/// One spec resolution issue; `statement` indexes the spec's statements,
/// `relation` and `row` its relations and a closed relation's rows.
#[napi(object, object_from_js = false)]
pub struct SpecIssueOut {
    pub code: String,
    pub message: String,
    pub statement: Option<u32>,
    pub relation: Option<u32>,
    pub row: Option<u32>,
}

/// Why a spec did not compile: unresolved names (`Spec`), or a resolved
/// descriptor the engine refused (`Schema`, citing descriptor statement ids).
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum SchemaDiagnostic {
    Spec {
        issues: Vec<SpecIssueOut>,
    },
    Schema {
        code: String,
        message: String,
        statement: Option<StatementCite>,
        conflict: Option<StatementCite>,
    },
}

fn spec_issue(issue: &SpecIssue) -> SpecIssueOut {
    use bumbledb::schema::spec::LiteralAt;
    let (statement, relation, row) = match issue {
        SpecIssue::UnknownRelation { statement, .. }
        | SpecIssue::UnknownField { statement, .. }
        | SpecIssue::CapacityInverted { statement, .. }
        | SpecIssue::CapacityDependentFloor { statement }
        | SpecIssue::WeightPathRefused { statement, .. }
        | SpecIssue::BoundPathRefused { statement, .. }
        | SpecIssue::DegenerateLiteralSet { statement, .. }
        | SpecIssue::StatementNewtypeMismatch { statement, .. } => (Some(*statement), None, None),
        SpecIssue::NotAHandleField { at, .. } | SpecIssue::UnknownHandle { at, .. } => match at {
            LiteralAt::Selection { statement, .. } => (Some(*statement), None, None),
            LiteralAt::Row { relation, row, .. } => (None, Some(*relation), Some(*row)),
        },
        SpecIssue::RelationTooManyFields { relation, .. } => (None, Some(*relation), None),
        SpecIssue::RowArityExcess { relation, row, .. } => (None, Some(*relation), Some(*row)),
        SpecIssue::DuplicateHandleNewtype {
            second_relation, ..
        } => (None, Some(*second_relation), None),
    };
    SpecIssueOut {
        code: code(issue),
        message: issue.to_string(),
        statement: statement.map(ordinal),
        relation: relation.map(ordinal),
        row: row.map(ordinal),
    }
}

pub(crate) fn schema_diagnostic(
    error: &SchemaError,
    descriptor: &SchemaDescriptor,
) -> SchemaDiagnostic {
    let cite = |id: StatementId| StatementCite {
        id: u32::from(id.0),
        spelling: bumbledb::schema::render::render_expanded(descriptor, id),
    };
    match error {
        SchemaError::Statement { statement, kind } => SchemaDiagnostic::Schema {
            code: code(kind),
            message: error.to_string(),
            statement: Some(cite(*statement)),
            conflict: match kind {
                StatementErrorKind::DuplicateStatement { earlier }
                | StatementErrorKind::DuplicateFunctionality { earlier } => Some(cite(*earlier)),
                _ => None,
            },
        },
        other => SchemaDiagnostic::Schema {
            code: code(other),
            message: other.to_string(),
            statement: None,
            conflict: None,
        },
    }
}

#[napi(string_enum)]
pub enum IntervalElementOut {
    U64,
    I64,
    F64,
}

impl From<IntervalElement> for IntervalElementOut {
    fn from(value: IntervalElement) -> Self {
        match value {
            IntervalElement::U64 => Self::U64,
            IntervalElement::I64 => Self::I64,
            IntervalElement::F64 => Self::F64,
        }
    }
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum ValueTypeOut {
    Bool,
    U64,
    I64,
    F64,
    String,
    Uuid,
    FixedBytes {
        len: u32,
    },
    Interval {
        element: IntervalElementOut,
    },
    FixedInterval {
        element: IntervalElementOut,
        width: u64,
    },
}

impl From<ValueType> for ValueTypeOut {
    fn from(value: ValueType) -> Self {
        match value {
            ValueType::Bool => Self::Bool,
            ValueType::U64 => Self::U64,
            ValueType::I64 => Self::I64,
            ValueType::F64 => Self::F64,
            ValueType::String => Self::String,
            ValueType::Uuid => Self::Uuid,
            ValueType::FixedBytes { len } => Self::FixedBytes {
                len: u32::from(len),
            },
            ValueType::Interval { element } => Self::Interval {
                element: element.into(),
            },
            ValueType::FixedInterval { element, width } => Self::FixedInterval {
                element: element.element().into(),
                width,
            },
        }
    }
}

#[napi(object, object_from_js = false)]
pub struct FieldOut {
    pub name: String,
    pub id: u32,
    pub value_type: ValueTypeOut,
    pub newtype: Option<String>,
}

/// One closed relation row: its handle, declaration-order id and columns.
#[napi(object, object_from_js = false)]
pub struct ClosedRowOut {
    pub handle: String,
    pub id: u64,
    pub values: Vec<NamedValueOut>,
}

#[napi(object, object_from_js = false)]
pub struct RelationOut {
    pub name: String,
    pub id: u32,
    pub fields: Vec<FieldOut>,
    pub extension: Option<Vec<ClosedRowOut>>,
}

#[napi(object, object_from_js = false)]
pub struct SelectionOut {
    pub field: u32,
    #[napi(ts_type = "Array<CellValue>")]
    pub values: Vec<ValueOut>,
}

#[napi(object, object_from_js = false)]
pub struct SideOut {
    pub relation: u32,
    pub projection: Vec<u32>,
    pub selection: Vec<SelectionOut>,
}

impl From<&Side> for SideOut {
    fn from(side: &Side) -> Self {
        Self {
            relation: side.relation.0,
            projection: side.projection.iter().map(|f| u32::from(f.0)).collect(),
            selection: side
                .selection
                .iter()
                .map(|(field, set)| SelectionOut {
                    field: u32::from(field.0),
                    values: set
                        .literals()
                        .iter()
                        .cloned()
                        .map(ValueOut::from_value)
                        .collect(),
                })
                .collect(),
        }
    }
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum WeightOut {
    Unit,
    Field { field: u32 },
    Duration { field: u32 },
}

/// A capacity ceiling; `Unbounded` is the `*` window end.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum CeilingOut {
    Unbounded,
    Lit { value: u64 },
    TargetField { field: u32 },
    TargetDuration { field: u32 },
}

/// One materialized statement; `id` is its `StatementId`.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum StatementOut {
    Functionality {
        id: u32,
        spelling: String,
        relation: u32,
        projection: Vec<u32>,
    },
    Containment {
        id: u32,
        spelling: String,
        source: SideOut,
        target: SideOut,
    },
    Capacity {
        id: u32,
        spelling: String,
        target: SideOut,
        weight: WeightOut,
        lo: u64,
        hi: CeilingOut,
        source: SideOut,
    },
}

/// The sealed descriptor: relations with ids, field types and newtypes,
/// materialized statements, and the hex schema fingerprint.
#[napi(object, object_from_js = false)]
pub struct SchemaDescriptorOut {
    pub relations: Vec<RelationOut>,
    pub statements: Vec<StatementOut>,
    pub fingerprint: String,
}

impl SchemaHandle {
    fn compile(spec: SchemaSpec) -> Result<Self, SchemaDiagnostic> {
        let newtypes = spec
            .relations
            .iter()
            .map(|relation| {
                relation
                    .closed
                    .iter()
                    .map(|closed| Some(closed.newtype.clone()))
                    .chain(relation.fields.iter().map(|field| field.newtype.clone()))
                    .collect()
            })
            .collect();
        let descriptor = spec.descriptor().map_err(|error| SchemaDiagnostic::Spec {
            issues: error.issues().iter().map(spec_issue).collect(),
        })?;
        Self::seal(descriptor, newtypes)
    }

    /// Validate a resolved descriptor; `newtypes` is per relation, in sealed
    /// field order, and may be empty for a descriptor built without a spec.
    pub(crate) fn seal(
        descriptor: SchemaDescriptor,
        newtypes: Vec<Vec<Option<Box<str>>>>,
    ) -> Result<Self, SchemaDiagnostic> {
        let schema = descriptor
            .clone()
            .validate()
            .map_err(|error| schema_diagnostic(&error, &descriptor))?;
        let fingerprint = hex(&bumbledb::schema::fingerprint::fingerprint(&schema).0);
        let rosters = descriptor
            .relations
            .iter()
            .map(|relation| SealedRoster {
                name: relation.name.clone(),
                fields: relation
                    .sealed_fields()
                    .map(|slot| match slot {
                        SealedField::SyntheticId => FieldDescriptor {
                            name: Box::from(slot.name()),
                            value_type: *slot.value_type(),
                        },
                        SealedField::Declared(field) => field.clone(),
                    })
                    .collect(),
            })
            .collect();
        Ok(Self {
            statements: descriptor.materialized_statements(),
            descriptor,
            schema: Arc::new(schema),
            rosters,
            newtypes,
            fingerprint,
        })
    }

    fn descriptor_out(&self) -> SchemaDescriptorOut {
        use bumbledb::schema::manifest::ManifestDescriptor as _;
        let manifest = self.descriptor.manifest();
        let relations = manifest
            .relations
            .into_iter()
            .enumerate()
            .map(|(index, relation)| RelationOut {
                name: relation.name.into(),
                id: relation.id.0,
                fields: relation
                    .fields
                    .into_iter()
                    .enumerate()
                    .map(|(slot, field)| FieldOut {
                        name: field.name.into(),
                        id: u32::from(field.id.0),
                        value_type: field.value_type.into(),
                        newtype: self
                            .newtypes
                            .get(index)
                            .and_then(|fields| fields.get(slot))
                            .and_then(|newtype| newtype.as_deref())
                            .map(Into::into),
                    })
                    .collect(),
                extension: relation.extension.map(|rows| {
                    rows.into_iter()
                        .map(|row| ClosedRowOut {
                            handle: row.handle.into(),
                            id: row.id,
                            values: row
                                .values
                                .into_iter()
                                .map(|(name, value)| NamedValueOut {
                                    name: name.into(),
                                    value: ValueOut::from_value(value),
                                })
                                .collect(),
                        })
                        .collect()
                }),
            })
            .collect();
        let statements = self
            .statements
            .iter()
            .zip(manifest.statements)
            .map(|(statement, cited)| {
                let id = u32::from(cited.id.0);
                let spelling = cited.spelling;
                match statement {
                    StatementDescriptor::Functionality {
                        relation,
                        projection,
                    } => StatementOut::Functionality {
                        id,
                        spelling,
                        relation: relation.0,
                        projection: projection.iter().map(|f| u32::from(f.0)).collect(),
                    },
                    StatementDescriptor::Containment { source, target } => {
                        StatementOut::Containment {
                            id,
                            spelling,
                            source: source.into(),
                            target: target.into(),
                        }
                    }
                    StatementDescriptor::Capacity {
                        target,
                        weight,
                        lo,
                        hi,
                        source,
                    } => StatementOut::Capacity {
                        id,
                        spelling,
                        target: target.into(),
                        weight: match weight {
                            Weight::Unit => WeightOut::Unit,
                            Weight::Field(f) => WeightOut::Field {
                                field: u32::from(f.0),
                            },
                            Weight::DurationOf(f) => WeightOut::Duration {
                                field: u32::from(f.0),
                            },
                        },
                        lo: *lo,
                        hi: match hi {
                            None => CeilingOut::Unbounded,
                            Some(Bound::Lit(value)) => CeilingOut::Lit { value: *value },
                            Some(Bound::TargetField(f)) => CeilingOut::TargetField {
                                field: u32::from(f.0),
                            },
                            Some(Bound::TargetDuration(f)) => CeilingOut::TargetDuration {
                                field: u32::from(f.0),
                            },
                        },
                        source: source.into(),
                    },
                }
            })
            .collect();
        SchemaDescriptorOut {
            relations,
            statements,
            fingerprint: self.fingerprint.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) fn sealed(descriptor: &SchemaDescriptor) -> Arc<SchemaHandle> {
    Arc::new(
        SchemaHandle::seal(descriptor.clone(), Vec::new())
            .unwrap_or_else(|_| panic!("test descriptor validates")),
    )
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum SchemaCompiled {
    Compiled {
        schema: External<Arc<SchemaHandle>>,
        descriptor: SchemaDescriptorOut,
    },
    Invalid {
        diagnostic: SchemaDiagnostic,
    },
    Malformed {
        path: String,
        message: String,
    },
}

/// Compile one schema spec (the schema file's JSON) into a validated handle.
#[napi]
#[must_use]
pub fn compile_schema(spec_json: String) -> SchemaCompiled {
    let spec: SchemaSpecIn = match decode(&spec_json) {
        Ok(spec) => spec,
        Err(Malformed { path, message }) => return SchemaCompiled::Malformed { path, message },
    };
    match SchemaHandle::compile(spec.into()) {
        Ok(handle) => SchemaCompiled::Compiled {
            descriptor: handle.descriptor_out(),
            schema: External::new(Arc::new(handle)),
        },
        Err(diagnostic) => SchemaCompiled::Invalid { diagnostic },
    }
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum SchemaBindings {
    Bindings { source: String },
    Unrepresentable { coordinate: String, reason: String },
}

/// TypeScript SDK declarations for a compiled schema.
#[napi]
#[must_use]
pub fn schema_bindings(schema: &External<Arc<SchemaHandle>>) -> SchemaBindings {
    match crate::bindings::emit(&schema.descriptor) {
        Ok(source) => SchemaBindings::Bindings { source },
        Err(error) => SchemaBindings::Unrepresentable {
            coordinate: error.coordinate,
            reason: error.reason.into(),
        },
    }
}

#[cfg(test)]
mod tests;
