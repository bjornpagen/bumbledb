//! The schema-spec crossing: borrowed C view structs
//! mirroring `bumbledb::SchemaSpec` field for field, copied IMMEDIATELY
//! into the Rust-owned spec — no borrowed caller memory survives
//! `bdb_db_create` / `bdb_db_open` / `bdb_db_ephemeral`. The engine's
//! `SchemaSpec::descriptor()` remains the canonical lowering; this module
//! judges nothing.

use bumbledb::SchemaSpec;
use bumbledb::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FieldSpec, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SideSpec, StatementSpec, WeightSpec,
};
use bumbledb::schema::{IntervalElement, ValueType};

use crate::error::fail_shape;
use crate::value::{bdb_string_view, bdb_value, value_in};
use crate::{BridgeResult, bool_in, c_tag, ref_in, slice_in, tag_in};

/// The structural value-type tag (`bumbledb::schema::ValueType`, spelled
/// C).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_value_type_kind {
    Bool,
    U64,
    I64,
    String,
    FixedBytes,
    Interval,
}

c_tag!(bdb_value_type_kind {
    Bool,
    U64,
    I64,
    String,
    FixedBytes,
    Interval,
});

/// An interval's element domain.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_interval_element {
    U64,
    I64,
}

c_tag!(bdb_interval_element { U64, I64 });

/// One structural value type. `fixed_len` is read for `FixedBytes`;
/// `element` / `has_width` / `width` for `Interval` (`has_width == false`
/// is the general 16-byte interval; `true` the fixed-width family).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_value_type {
    pub kind: u32,
    pub fixed_len: u16,
    pub element: u32,
    pub has_width: u8,
    pub width: u64,
}

/// One field: name, structural type, optional host newtype label (null
/// `data` = absent; carried for closed-handle resolution only, dropped at
/// descriptor lowering), and the `fresh` mark.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_field_spec {
    pub name: bdb_string_view,
    pub value_type: bdb_value_type,
    pub newtype: bdb_string_view,
    pub fresh: u8,
}

/// A literal's tag: a plain tagged value, or a closed relation's handle
/// by name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_literal_kind {
    Value,
    Handle,
}

c_tag!(bdb_literal_kind { Value, Handle });

/// One literal as spelled.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_literal {
    pub kind: u32,
    pub value: bdb_value,
    pub handle: bdb_string_view,
}

/// A σ binding's right side: one literal or a literal set (read
/// disjunctively).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_literal_set_kind {
    One,
    Many,
}

c_tag!(bdb_literal_set_kind { One, Many });

/// One literal set. `One` reads `literals[0]` (`literal_count` must be
/// 1); `Many` reads all `literal_count` entries.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_literal_set {
    pub kind: u32,
    pub literals: *const bdb_literal,
    pub literal_count: usize,
}

/// One σ binding: `field == literal-or-set`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_selection_binding {
    pub field: bdb_string_view,
    pub set: bdb_literal_set,
}

/// One side of a containment/capacity statement:
/// `Relation(projection… | selection…)`, all names.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_side {
    pub relation: bdb_string_view,
    pub projection: *const bdb_string_view,
    pub projection_count: usize,
    pub selection: *const bdb_selection_binding,
    pub selection_count: usize,
}

/// A capacity weight's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_weight_kind {
    Unit,
    Field,
    DurationField,
}

c_tag!(bdb_weight_kind {
    Unit,
    Field,
    DurationField
});

/// A capacity weight; `field` is read for `Field`/`DurationField`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_weight {
    pub kind: u32,
    pub field: bdb_string_view,
}

/// A capacity bound's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_bound_kind {
    Lit,
    Field,
    DurationField,
}

c_tag!(bdb_bound_kind {
    Lit,
    Field,
    DurationField
});

/// One capacity bound; `lit` for `Lit`, `field` for
/// `Field`/`DurationField`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_bound {
    pub kind: u32,
    pub lit: u64,
    pub field: bdb_string_view,
}

/// A capacity window's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_capacity_window_kind {
    Exact,
    Range,
    Floor,
}

c_tag!(bdb_capacity_window_kind {
    Exact,
    Range,
    Floor
});

/// One capacity window: `Exact` reads `lo` as the exact bound; `Floor`
/// reads `lo`; `Range` reads `lo` and `hi`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_capacity_window {
    pub kind: u32,
    pub lo: bdb_bound,
    pub hi: bdb_bound,
}

/// A statement's form tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_statement_spec_kind {
    Fd,
    Containment,
    Capacity,
}

c_tag!(bdb_statement_spec_kind {
    Fd,
    Containment,
    Capacity
});

/// One dependency statement. `Fd` reads `fd_relation` +
/// `fd_projection`; `Containment` reads `source`/`target`/`bidirectional`;
/// `Capacity` reads `target`/`weight`/`window`/`source` (the operator's
/// read order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_statement_spec {
    pub kind: u32,
    pub fd_relation: bdb_string_view,
    pub fd_projection: *const bdb_string_view,
    pub fd_projection_count: usize,
    pub source: bdb_side,
    pub target: bdb_side,
    pub bidirectional: u8,
    pub weight: bdb_weight,
    pub window: bdb_capacity_window,
}

/// One ground axiom of a closed relation: the handle plus one literal per
/// declared intrinsic column, in field-declaration order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_closed_row {
    pub handle: bdb_string_view,
    pub values: *const bdb_literal,
    pub value_count: usize,
}

/// A closed relation's closed half: the handle newtype and the ground
/// axioms, fused (absent `closed` on the relation = ordinary relation).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_closed_spec {
    pub newtype: bdb_string_view,
    pub rows: *const bdb_closed_row,
    pub row_count: usize,
}

/// One relation: name, declared fields, and closedness (`closed` null =
/// ordinary).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_relation_spec {
    pub name: bdb_string_view,
    pub fields: *const bdb_field_spec,
    pub field_count: usize,
    pub closed: *const bdb_closed_spec,
}

/// The whole schema spec: relations then statements, declaration order —
/// the order IS the id mint.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_schema_spec {
    pub relations: *const bdb_relation_spec,
    pub relation_count: usize,
    pub statements: *const bdb_statement_spec,
    pub statement_count: usize,
}

fn value_type_in(view: &bdb_value_type) -> BridgeResult<ValueType> {
    Ok(match tag_in::<bdb_value_type_kind>(view.kind)? {
        bdb_value_type_kind::Bool => ValueType::Bool,
        bdb_value_type_kind::U64 => ValueType::U64,
        bdb_value_type_kind::I64 => ValueType::I64,
        bdb_value_type_kind::String => ValueType::String,
        bdb_value_type_kind::FixedBytes => ValueType::FixedBytes {
            len: view.fixed_len,
        },
        bdb_value_type_kind::Interval => {
            let element = match tag_in::<bdb_interval_element>(view.element)? {
                bdb_interval_element::U64 => IntervalElement::U64,
                bdb_interval_element::I64 => IntervalElement::I64,
            };
            if bool_in(view.has_width)? {
                ValueType::FixedInterval {
                    element,
                    width: view.width,
                }
            } else {
                ValueType::Interval { element }
            }
        }
    })
}

fn literal_in(view: &bdb_literal) -> BridgeResult<LiteralSpec> {
    Ok(match tag_in::<bdb_literal_kind>(view.kind)? {
        bdb_literal_kind::Value => LiteralSpec::Value(value_in(&view.value)?),
        bdb_literal_kind::Handle => {
            LiteralSpec::Handle(view.handle.as_str("handle literal")?.into())
        }
    })
}

fn literal_set_in(view: &bdb_literal_set) -> BridgeResult<LiteralSetSpec> {
    let literals = slice_in(view.literals, view.literal_count)?;
    Ok(match tag_in::<bdb_literal_set_kind>(view.kind)? {
        bdb_literal_set_kind::One => {
            let [literal] = literals else {
                return Err(fail_shape(&format!(
                    "a One literal set carries exactly one literal, got {}",
                    literals.len()
                )));
            };
            LiteralSetSpec::One(literal_in(literal)?)
        }
        bdb_literal_set_kind::Many => LiteralSetSpec::Many(
            literals
                .iter()
                .map(literal_in)
                .collect::<BridgeResult<Vec<_>>>()?,
        ),
    })
}

fn names_in(
    views: *const bdb_string_view,
    count: usize,
    what: &str,
) -> BridgeResult<Vec<Box<str>>> {
    slice_in(views, count)?
        .iter()
        .map(|view| Ok(view.as_str(what)?.into()))
        .collect()
}

fn side_in(view: &bdb_side) -> BridgeResult<SideSpec> {
    let selection = slice_in(view.selection, view.selection_count)?
        .iter()
        .map(|binding| {
            Ok((
                Box::<str>::from(binding.field.as_str("selection field name")?),
                literal_set_in(&binding.set)?,
            ))
        })
        .collect::<BridgeResult<Vec<_>>>()?;
    Ok(SideSpec {
        relation: view.relation.as_str("side relation name")?.into(),
        projection: names_in(
            view.projection,
            view.projection_count,
            "projection field name",
        )?,
        selection,
    })
}

fn weight_in(view: &bdb_weight) -> BridgeResult<WeightSpec> {
    Ok(match tag_in::<bdb_weight_kind>(view.kind)? {
        bdb_weight_kind::Unit => WeightSpec::Unit,
        bdb_weight_kind::Field => WeightSpec::Field(view.field.as_str("weight field name")?.into()),
        bdb_weight_kind::DurationField => {
            WeightSpec::Duration(view.field.as_str("weight field name")?.into())
        }
    })
}

fn bound_in(view: &bdb_bound) -> BridgeResult<BoundSpec> {
    Ok(match tag_in::<bdb_bound_kind>(view.kind)? {
        bdb_bound_kind::Lit => BoundSpec::Lit(view.lit),
        bdb_bound_kind::Field => BoundSpec::Field(view.field.as_str("bound field name")?.into()),
        bdb_bound_kind::DurationField => {
            BoundSpec::Duration(view.field.as_str("bound field name")?.into())
        }
    })
}

fn window_in(view: &bdb_capacity_window) -> BridgeResult<CapacityWindowSpec> {
    Ok(match tag_in::<bdb_capacity_window_kind>(view.kind)? {
        bdb_capacity_window_kind::Exact => CapacityWindowSpec::Exact(bound_in(&view.lo)?),
        bdb_capacity_window_kind::Range => CapacityWindowSpec::Range {
            lo: bound_in(&view.lo)?,
            hi: bound_in(&view.hi)?,
        },
        bdb_capacity_window_kind::Floor => CapacityWindowSpec::Floor(bound_in(&view.lo)?),
    })
}

fn statement_in(view: &bdb_statement_spec) -> BridgeResult<StatementSpec> {
    Ok(match tag_in::<bdb_statement_spec_kind>(view.kind)? {
        bdb_statement_spec_kind::Fd => StatementSpec::Fd {
            relation: view.fd_relation.as_str("fd relation name")?.into(),
            projection: names_in(
                view.fd_projection,
                view.fd_projection_count,
                "fd projection field name",
            )?,
        },
        bdb_statement_spec_kind::Containment => StatementSpec::Containment {
            source: side_in(&view.source)?,
            target: side_in(&view.target)?,
            bidirectional: bool_in(view.bidirectional)?,
        },
        bdb_statement_spec_kind::Capacity => StatementSpec::Capacity {
            target: side_in(&view.target)?,
            weight: weight_in(&view.weight)?,
            window: window_in(&view.window)?,
            source: side_in(&view.source)?,
        },
    })
}

fn relation_in(view: &bdb_relation_spec) -> BridgeResult<RelationSpec> {
    let fields = slice_in(view.fields, view.field_count)?
        .iter()
        .map(|field| {
            Ok(FieldSpec {
                name: field.name.as_str("field name")?.into(),
                value_type: value_type_in(&field.value_type)?,
                newtype: field.newtype.as_opt_str("field newtype")?.map(Into::into),
                fresh: bool_in(field.fresh)?,
            })
        })
        .collect::<BridgeResult<Vec<_>>>()?;
    let closed = if view.closed.is_null() {
        None
    } else {
        let closed = ref_in(view.closed)?;
        let rows = slice_in(closed.rows, closed.row_count)?
            .iter()
            .map(|row| {
                Ok(RowSpec {
                    handle: row.handle.as_str("closed row handle")?.into(),
                    values: slice_in(row.values, row.value_count)?
                        .iter()
                        .map(literal_in)
                        .collect::<BridgeResult<Vec<_>>>()?,
                })
            })
            .collect::<BridgeResult<Vec<_>>>()?;
        Some(ClosedSpec {
            newtype: closed.newtype.as_str("closed newtype")?.into(),
            rows,
        })
    };
    Ok(RelationSpec {
        name: view.name.as_str("relation name")?.into(),
        fields,
        closed,
    })
}

/// The whole inbound spec, copied into the Rust-owned `SchemaSpec`
/// before any engine call.
pub(crate) fn schema_spec_in(view: &bdb_schema_spec) -> BridgeResult<SchemaSpec> {
    Ok(SchemaSpec {
        relations: slice_in(view.relations, view.relation_count)?
            .iter()
            .map(relation_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        statements: slice_in(view.statements, view.statement_count)?
            .iter()
            .map(statement_in)
            .collect::<BridgeResult<Vec<_>>>()?,
    })
}
