//! The schema spec: a theory as named plain data, mirroring
//! `bumbledb::SchemaSpec` field for field. This JSON is also the schema file.
use bumbledb::SchemaSpec;
use bumbledb::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FieldSpec, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SideSpec, StatementSpec, WeightSpec,
};
use napi_derive::napi;
use serde::Deserialize;

use super::value::{Literal, U64Text, ValueTypeIn};

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaSpecIn {
    pub relations: Vec<RelationSpecIn>,
    pub statements: Vec<StatementSpecIn>,
}

/// One relation; `closed` present declares it closed.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationSpecIn {
    pub name: String,
    pub fields: Vec<FieldSpecIn>,
    pub closed: Option<ClosedSpecIn>,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldSpecIn {
    pub name: String,
    pub value_type: ValueTypeIn,
    pub newtype: Option<String>,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClosedSpecIn {
    pub newtype: String,
    pub rows: Vec<RowSpecIn>,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RowSpecIn {
    pub handle: String,
    pub values: Vec<LiteralSpecIn>,
}

/// A literal as spelled: a plain value, or a closed relation's handle.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum LiteralSpecIn {
    Value {
        #[napi(ts_type = "ValueIn")]
        value: Literal,
    },
    Handle {
        handle: String,
    },
}

/// One selection binding's right side; a literal set reads disjunctively.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum LiteralSetSpecIn {
    One { literal: LiteralSpecIn },
    Many { literals: Vec<LiteralSpecIn> },
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionIn {
    pub field: String,
    pub set: LiteralSetSpecIn,
}

/// One side of a containment or capacity statement, all by name.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SideSpecIn {
    pub relation: String,
    pub projection: Vec<String>,
    pub selection: Vec<SelectionIn>,
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum WeightSpecIn {
    Unit,
    Field { field: String },
    Duration { field: String },
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum BoundSpecIn {
    Lit {
        #[napi(ts_type = "string")]
        value: U64Text,
    },
    Field {
        field: String,
    },
    Duration {
        field: String,
    },
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CapacityWindowSpecIn {
    Exact { n: BoundSpecIn },
    Range { lo: BoundSpecIn, hi: BoundSpecIn },
    Floor { lo: BoundSpecIn },
}

/// One dependency statement; `bidirectional` containment is the `==` form.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum StatementSpecIn {
    Fd {
        relation: String,
        projection: Vec<String>,
    },
    Containment {
        source: SideSpecIn,
        target: SideSpecIn,
        bidirectional: bool,
    },
    Capacity {
        target: SideSpecIn,
        weight: WeightSpecIn,
        window: CapacityWindowSpecIn,
        source: SideSpecIn,
    },
}

fn names(names: Vec<String>) -> Vec<Box<str>> {
    names.into_iter().map(Into::into).collect()
}

impl From<LiteralSpecIn> for LiteralSpec {
    fn from(value: LiteralSpecIn) -> Self {
        match value {
            LiteralSpecIn::Value { value } => Self::Value(value.0),
            LiteralSpecIn::Handle { handle } => Self::Handle(handle.into()),
        }
    }
}

impl From<LiteralSetSpecIn> for LiteralSetSpec {
    fn from(value: LiteralSetSpecIn) -> Self {
        match value {
            LiteralSetSpecIn::One { literal } => Self::One(literal.into()),
            LiteralSetSpecIn::Many { literals } => {
                Self::Many(literals.into_iter().map(Into::into).collect())
            }
        }
    }
}

impl From<SideSpecIn> for SideSpec {
    fn from(value: SideSpecIn) -> Self {
        Self {
            relation: value.relation.into(),
            projection: names(value.projection),
            selection: value
                .selection
                .into_iter()
                .map(|binding| (binding.field.into(), binding.set.into()))
                .collect(),
        }
    }
}

impl From<BoundSpecIn> for BoundSpec {
    fn from(value: BoundSpecIn) -> Self {
        match value {
            BoundSpecIn::Lit { value } => Self::Lit(value.0),
            BoundSpecIn::Field { field } => Self::Field(field.into()),
            BoundSpecIn::Duration { field } => Self::Duration(field.into()),
        }
    }
}

impl From<StatementSpecIn> for StatementSpec {
    fn from(value: StatementSpecIn) -> Self {
        match value {
            StatementSpecIn::Fd {
                relation,
                projection,
            } => Self::Fd {
                relation: relation.into(),
                projection: names(projection),
            },
            StatementSpecIn::Containment {
                source,
                target,
                bidirectional,
            } => Self::Containment {
                source: source.into(),
                target: target.into(),
                bidirectional,
            },
            StatementSpecIn::Capacity {
                target,
                weight,
                window,
                source,
            } => Self::Capacity {
                target: target.into(),
                weight: match weight {
                    WeightSpecIn::Unit => WeightSpec::Unit,
                    WeightSpecIn::Field { field } => WeightSpec::Field(field.into()),
                    WeightSpecIn::Duration { field } => WeightSpec::Duration(field.into()),
                },
                window: match window {
                    CapacityWindowSpecIn::Exact { n } => CapacityWindowSpec::Exact(n.into()),
                    CapacityWindowSpecIn::Range { lo, hi } => CapacityWindowSpec::Range {
                        lo: lo.into(),
                        hi: hi.into(),
                    },
                    CapacityWindowSpecIn::Floor { lo } => CapacityWindowSpec::Floor(lo.into()),
                },
                source: source.into(),
            },
        }
    }
}

impl From<SchemaSpecIn> for SchemaSpec {
    fn from(value: SchemaSpecIn) -> Self {
        Self {
            relations: value
                .relations
                .into_iter()
                .map(|relation| RelationSpec {
                    name: relation.name.into(),
                    fields: relation
                        .fields
                        .into_iter()
                        .map(|field| FieldSpec {
                            name: field.name.into(),
                            value_type: field.value_type.into(),
                            newtype: field.newtype.map(Into::into),
                        })
                        .collect(),
                    closed: relation.closed.map(|closed| ClosedSpec {
                        newtype: closed.newtype.into(),
                        rows: closed
                            .rows
                            .into_iter()
                            .map(|row| RowSpec {
                                handle: row.handle.into(),
                                values: row.values.into_iter().map(Into::into).collect(),
                            })
                            .collect(),
                    }),
                })
                .collect(),
            statements: value.statements.into_iter().map(Into::into).collect(),
        }
    }
}
