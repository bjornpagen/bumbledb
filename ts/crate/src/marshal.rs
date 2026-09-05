//! JS ⇄ engine-data marshaling — the whole vocabulary the bridge speaks.
//! Nothing here validates semantics: unresolvable names, banned spellings,
//! shape mismatches beyond marshaling, and every dependency judgment belong
//! to the engine's own typed boundaries.
use bumbledb::schema::spec::{
    BoundSpec, CapacityWindowSpec, ClosedSpec, FieldSpec, LiteralSetSpec, LiteralSpec,
    RelationSpec, RowSpec, SideSpec, StatementSpec, WeightSpec,
};
use bumbledb::schema::{
    Bound, FieldDescriptor, Generation, IntervalElement, RelationManifest, SealedField, Side,
    StatementDescriptor, ValueType, Weight,
};
use bumbledb::{
    AcceptedCollection, AllenMask, AnswerValue, Answers, Atom, AtomSource, CmpOp,
    CollectionBuilder, Comparison, ConditionTree, F64, FieldId, FindTerm, FoldOp, HeadOp, HeadTerm,
    Interior, InteriorId, Interval, Manifest, NonEmpty, ParamId, ProjectionRule, Query, Rec,
    RecRule, RecStep, RelationId, RenderedViolation, Rule, SchemaDescriptor, SchemaSpec,
    StatementId, StatementKind, Term, Value, VarId,
};
use napi::bindgen_prelude::{
    Array, BigInt, Env, FromNapiValue, Object, ToNapiValue, Uint8Array, i64n,
};
use napi::{Unknown, ValueType as JsType, sys};

use crate::tags;

pub(crate) fn err(message: String) -> napi::Error {
    napi::Error::from_reason(message)
}

pub(crate) fn engine_message(error: &bumbledb::Error) -> String {
    error.to_string()
}

pub(crate) fn throw_engine(env: Env, error: &bumbledb::Error) -> napi::Error {
    throw_kind_message(
        env,
        crate::tags::error_family::tag(&error.family()),
        engine_message(error),
    )
}

pub(crate) fn throw_kind_message(
    env: Env,
    kind: &'static str,
    message: impl AsRef<str>,
) -> napi::Error {
    match throw_object(env, kind, message.as_ref()) {
        Ok(()) => napi::Error::from_status(napi::Status::PendingException),
        Err(err) => err,
    }
}

fn throw_object(env: Env, kind: &'static str, message: &str) -> napi::Result<()> {
    let mut error = env.create_error(napi::Error::from_reason(message))?;
    error.set("kind", kind)?;
    env.throw(error)
}

fn js_type_name(ty: JsType) -> &'static str {
    match ty {
        JsType::Undefined => "undefined",
        JsType::Null => "null",
        JsType::Boolean => "boolean",
        JsType::Number => "number",
        JsType::String => "string",
        JsType::Symbol => "symbol",
        JsType::Object => "object",
        JsType::Function => "function",
        JsType::External => "external",
        JsType::BigInt => "bigint",
        JsType::Unknown => "unknown",
    }
}

pub(crate) fn req<T: FromNapiValue>(
    obj: &Object,
    key: &str,
    ctx: impl std::fmt::Display,
) -> napi::Result<T> {
    obj.get::<T>(key)?
        .ok_or_else(|| err(format!("bumbledb marshal: missing `{key}` in {ctx}")))
}

pub(crate) fn req_at<T: FromNapiValue>(
    arr: &Array,
    index: u32,
    ctx: impl std::fmt::Display,
) -> napi::Result<T> {
    arr.get::<T>(index)?.ok_or_else(|| {
        err(format!(
            "bumbledb marshal: missing element {index} in {ctx}"
        ))
    })
}

pub(crate) fn u64_in(value: &BigInt, ctx: impl std::fmt::Display) -> napi::Result<u64> {
    let (sign, word, lossless) = value.get_u64();
    if sign || !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of u64 range"
        )));
    }
    Ok(word)
}

pub(crate) fn i64_in(value: &BigInt, ctx: impl std::fmt::Display) -> napi::Result<i64> {
    let (word, lossless) = value.get_i64();
    if !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of i64 range"
        )));
    }
    Ok(word)
}

pub(crate) fn ordinal(value: f64, ctx: &str) -> napi::Result<u32> {
    if !(value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX))
    {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: expected a non-negative integer id, got {value}"
        )));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the guard above proved: finite, non-negative, integral, <= u32::MAX"
    )]
    Ok(value as u32)
}

pub(crate) fn u16_id(value: u32, ctx: &str) -> napi::Result<u16> {
    u16::try_from(value)
        .map_err(|_| err(format!("bumbledb marshal: {ctx}: id {value} exceeds u16")))
}

fn interval_u64_in(
    obj: &Object,
    ctx: impl std::fmt::Display + Copy,
) -> napi::Result<Interval<u64>> {
    let start = u64_in(&req::<BigInt>(obj, "start", ctx)?, ctx)?;
    let end = u64_in(&req::<BigInt>(obj, "end", ctx)?, ctx)?;
    Interval::<u64>::new(start, end).ok_or_else(|| {
        err(format!(
            "bumbledb marshal: {ctx}: empty interval (start {start} >= end {end})"
        ))
    })
}

fn interval_i64_in(
    obj: &Object,
    ctx: impl std::fmt::Display + Copy,
) -> napi::Result<Interval<i64>> {
    let start = i64_in(&req::<BigInt>(obj, "start", ctx)?, ctx)?;
    let end = i64_in(&req::<BigInt>(obj, "end", ctx)?, ctx)?;
    Interval::<i64>::new(start, end).ok_or_else(|| {
        err(format!(
            "bumbledb marshal: {ctx}: empty interval (start {start} >= end {end})"
        ))
    })
}

fn interval_in(
    obj: &Object,
    element: IntervalElement,
    ctx: impl std::fmt::Display + Copy,
) -> napi::Result<Value> {
    match element {
        IntervalElement::U64 => interval_u64_in(obj, ctx).map(Value::IntervalU64),
        IntervalElement::I64 => interval_i64_in(obj, ctx).map(Value::IntervalI64),
    }
}

#[derive(Clone, Copy)]
struct CellCtx<'a> {
    relation: &'a str,
    field: &'a str,
}

impl std::fmt::Display for CellCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "relation `{}` field `{}`", self.relation, self.field)
    }
}

fn cell_mismatch(ctx: CellCtx<'_>, want: &str, got: JsType) -> napi::Error {
    err(format!(
        "bumbledb marshal: {ctx}: expected {want}, got {}",
        js_type_name(got)
    ))
}

fn bytes_width_mismatch(ctx: CellCtx<'_>, len: u16, witnessed: usize) -> napi::Error {
    err(format!(
        "bumbledb marshal: {ctx}: expected bytes<{len}>, got {witnessed} bytes"
    ))
}

#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe (it trusts the caller on the \
              JS type); every cast below is fenced by the `get_type` check in \
              its own arm"
)]
fn schema_value(
    expected: &ValueType,
    value: &Unknown,
    relation: &str,
    field: &str,
) -> napi::Result<Value> {
    let ctx = CellCtx { relation, field };
    let got = value.get_type()?;
    let mismatch = |want: &str| cell_mismatch(ctx, want, got);
    // SAFETY (each `cast` below): the arm's guard just proved `got` is the
    // exact JS type the cast assumes; a mismatch returned before the cast.
    match expected {
        ValueType::Bool => {
            if got != JsType::Boolean {
                return Err(mismatch("boolean"));
            }
            Ok(Value::Bool(unsafe { value.cast::<bool>()? }))
        }
        ValueType::U64 => {
            if got != JsType::BigInt {
                return Err(mismatch("bigint (u64)"));
            }
            Ok(Value::U64(u64_in(
                &unsafe { value.cast::<BigInt>()? },
                ctx,
            )?))
        }
        ValueType::I64 => {
            if got != JsType::BigInt {
                return Err(mismatch("bigint (i64)"));
            }
            Ok(Value::I64(i64_in(
                &unsafe { value.cast::<BigInt>()? },
                ctx,
            )?))
        }
        ValueType::F64 => {
            if got != JsType::Number {
                return Err(mismatch("number (f64)"));
            }
            Ok(Value::F64(F64::from(unsafe { value.cast::<f64>()? })))
        }
        ValueType::String => {
            if got != JsType::String {
                return Err(mismatch("string"));
            }
            let text = unsafe { value.cast::<String>()? };
            Ok(Value::String(text.into()))
        }
        ValueType::FixedBytes { len } => {
            if got != JsType::Object {
                return Err(mismatch("Uint8Array"));
            }
            let bytes = unsafe { value.cast::<Uint8Array>()? };
            if bytes.len() != usize::from(*len) {
                return Err(bytes_width_mismatch(ctx, *len, bytes.len()));
            }
            Ok(Value::FixedBytes(bytes.to_vec().into_boxed_slice()))
        }
        ValueType::Interval { element } | ValueType::FixedInterval { element, .. } => {
            if got != JsType::Object {
                return Err(mismatch("{ start, end } bigint pair"));
            }
            interval_in(&unsafe { value.cast::<Object>()? }, *element, ctx)
        }
    }
}

/// One sealed field slot's spec-only attributes — `fresh` and the host
/// newtype name, which the engine descriptor drops after resolution. The
/// bridge carries them alongside the descriptor so the manifest wire
/// speaks the spec's whole field vocabulary; nothing here is judged.
#[derive(Clone)]
pub struct FieldAttrs {
    pub(crate) fresh: bool,
    pub(crate) newtype: Option<Box<str>>,
}

/// Per-relation sealed-order attribute rows off the parsed spec: a closed
/// relation's synthetic `id` slot leads (never fresh, carrying the handle
/// newtype), then the declared fields in declaration order — the same
/// order `sealed_fields()` walks, restated from the same spec datum.
pub(crate) fn field_attrs(spec: &SchemaSpec) -> Vec<Vec<FieldAttrs>> {
    spec.relations
        .iter()
        .map(|relation| {
            relation
                .closed
                .iter()
                .map(|closed| FieldAttrs {
                    fresh: false,
                    newtype: Some(closed.newtype.clone()),
                })
                .chain(relation.fields.iter().map(|field| FieldAttrs {
                    fresh: field.fresh,
                    newtype: field.newtype.clone(),
                }))
                .collect()
        })
        .collect()
}

pub(crate) struct SealedRoster {
    pub(crate) name: Box<str>,
    pub(crate) fields: Vec<FieldDescriptor>,
}

pub(crate) fn sealed_rosters(descriptor: &SchemaDescriptor) -> Vec<SealedRoster> {
    descriptor
        .relations
        .iter()
        .map(|relation| SealedRoster {
            name: relation.name.clone(),
            fields: relation
                .sealed_fields()
                .map(|slot| match slot {
                    // The synthetic handle slot materializes as an ordinary
                    // descriptor (name "id", u64, no generation): the bridge
                    // only reads name + type, and the sealed ORDER stays the
                    // iterator's law, restated nowhere.
                    SealedField::SyntheticId => FieldDescriptor {
                        name: Box::from(slot.name()),
                        value_type: *slot.value_type(),
                        generation: Generation::None,
                    },
                    SealedField::Declared(field) => field.clone(),
                })
                .collect(),
        })
        .collect()
}

fn roster(rosters: &[SealedRoster], relation: RelationId) -> napi::Result<&SealedRoster> {
    rosters.get(relation.0 as usize).ok_or_else(|| {
        err(format!(
            "bumbledb marshal: unknown relation id {}",
            relation.0
        ))
    })
}

pub(crate) fn fact_row(
    rosters: &[SealedRoster],
    relation: u32,
    values: &Array,
) -> napi::Result<(RelationId, Vec<Value>)> {
    let rel = RelationId(relation);
    let roster = roster(rosters, rel)?;
    Ok((rel, one_fact_row(&roster.name, &roster.fields, values)?))
}

pub(crate) fn accepted_collection(
    env: Env,
    rosters: &[SealedRoster],
    relation: u32,
    rows: u64,
    cells: &Array,
) -> napi::Result<AcceptedCollection> {
    let rel = RelationId(relation);
    if rows == 0 && cells.len() == 0 {
        return CollectionBuilder::new(rel, &[])
            .seal()
            .map_err(|error| throw_engine(env, &error));
    }
    let roster = roster(rosters, rel)?;
    let name: &str = &roster.name;
    let arity = roster.fields.len();
    let len = cells.len() as usize;
    // The stated count against the product, in u128 so `rows × arity`
    // cannot overflow the comparison — the one exact judgment covering
    // the dangling partial row, the fieldless overflow, and a mis-stated
    // count alike.
    let expected = u128::from(rows) * (arity as u128);
    if expected != len as u128 {
        return Err(err(format!(
            "bumbledb marshal: relation `{name}`: expected {expected} values, got {len}"
        )));
    }
    if arity == 0 {
        // An arity-0 collection IS its row count plus at most one
        // distinct fact (set semantics — every row is the empty tuple),
        // so the builder's arity-0 seal takes the stated count directly:
        // O(1), no per-row loop. `rows` is caller DATA on the raw addon
        // surface and the cells wall above is vacuous here (`0 == rows ×
        // 0` for EVERY count — any count is shape-lawful, N empty
        // tuples), so a stated 2^63 must never buy 2^63 bridge pushes
        // from a 16-byte payload; the engine's apply collapses the same
        // way (`apply_accepted`'s arity-0 arm: one judged apply,
        // `submitted = rows` exact, `changed` the one effect).
        let collection = CollectionBuilder::new(rel, &roster.fields)
            .seal_nullary(rows)
            .map_err(|error| throw_engine(env, &error))?;
        return Ok(collection);
    }
    let mut builder = CollectionBuilder::new(rel, &roster.fields);
    for index in 0..cells.len() {
        let field = &roster.fields[(index as usize) % arity];
        let value = req_at::<Unknown>(cells, index, format_args!("relation `{name}` collection"))?;
        push_cell(env, &mut builder, name, field, &value)?;
    }
    let collection = builder.seal().map_err(|error| throw_engine(env, &error))?;
    Ok(collection)
}

#[expect(
    unsafe_code,
    reason = "napi declares `Unknown::cast` unsafe (it trusts the caller on the \
              JS type); every cast below is fenced by the `get_type` check in \
              its own arm"
)]
fn push_cell(
    env: Env,
    builder: &mut CollectionBuilder<'_>,
    relation: &str,
    field: &FieldDescriptor,
    value: &Unknown,
) -> napi::Result<()> {
    let ctx = CellCtx {
        relation,
        field: &field.name,
    };
    let got = value.get_type()?;
    // SAFETY (each `cast` below): the arm's guard just proved `got` is the
    // exact JS type the cast assumes; a mismatch returned before the cast.
    let landed = match &field.value_type {
        ValueType::Bool => {
            if got != JsType::Boolean {
                return Err(cell_mismatch(ctx, "boolean", got));
            }
            builder.push_bool(unsafe { value.cast::<bool>()? })
        }
        ValueType::U64 => {
            if got != JsType::BigInt {
                return Err(cell_mismatch(ctx, "bigint (u64)", got));
            }
            builder.push_u64(u64_in(&unsafe { value.cast::<BigInt>()? }, ctx)?)
        }
        ValueType::I64 => {
            if got != JsType::BigInt {
                return Err(cell_mismatch(ctx, "bigint (i64)", got));
            }
            builder.push_i64(i64_in(&unsafe { value.cast::<BigInt>()? }, ctx)?)
        }
        ValueType::F64 => {
            if got != JsType::Number {
                return Err(cell_mismatch(ctx, "number (f64)", got));
            }
            builder.push_f64(F64::from(unsafe { value.cast::<f64>()? }))
        }
        ValueType::String => {
            if got != JsType::String {
                return Err(cell_mismatch(ctx, "string", got));
            }
            let text = unsafe { value.cast::<String>()? };
            builder.push_str(&text)
        }
        ValueType::FixedBytes { len } => {
            if got != JsType::Object {
                return Err(cell_mismatch(ctx, "Uint8Array", got));
            }
            let bytes = unsafe { value.cast::<Uint8Array>()? };
            if bytes.len() != usize::from(*len) {
                return Err(bytes_width_mismatch(ctx, *len, bytes.len()));
            }
            builder.push_bytes(&bytes)
        }
        ValueType::Interval { element } | ValueType::FixedInterval { element, .. } => {
            if got != JsType::Object {
                return Err(cell_mismatch(ctx, "{ start, end } bigint pair", got));
            }
            let obj = unsafe { value.cast::<Object>()? };
            match element {
                IntervalElement::U64 => builder.push_interval_u64(interval_u64_in(&obj, ctx)?),
                IntervalElement::I64 => builder.push_interval_i64(interval_i64_in(&obj, ctx)?),
            }
        }
    };
    landed.map_err(|error| throw_engine(env, &error))
}

fn one_fact_row(
    name: &str,
    fields: &[FieldDescriptor],
    values: &Array,
) -> napi::Result<Vec<Value>> {
    if values.len() as usize != fields.len() {
        return Err(err(format!(
            "bumbledb marshal: relation `{name}`: expected {} values, got {}",
            fields.len(),
            values.len()
        )));
    }
    let mut row = Vec::with_capacity(fields.len());
    for (index, field) in (0..values.len()).zip(fields.iter()) {
        let value = req_at::<Unknown>(values, index, format_args!("relation `{name}` row"))?;
        row.push(schema_value(&field.value_type, &value, name, &field.name)?);
    }
    Ok(row)
}

pub(crate) fn key_row(
    rosters: &[SealedRoster],
    statements: &[StatementDescriptor],
    relation: u32,
    key_statement: u32,
    values: &Array,
) -> napi::Result<(RelationId, StatementId, Vec<Value>)> {
    let rel = RelationId(relation);
    // The statement refusals below may name a relation the roster table
    // does not know — the id-speak fallback keeps their text unchanged;
    // the roster lookup itself refuses after the statement checks, exactly
    // where the old per-call derivation refused.
    let name = rosters.get(relation as usize).map_or_else(
        || format!("relation#{relation}"),
        |roster| roster.name.to_string(),
    );
    let statement_id = StatementId(u16_id(key_statement, "key statement id")?);
    let Some(StatementDescriptor::Functionality {
        relation: key_relation,
        projection,
    }) = statements.get(key_statement as usize)
    else {
        return Err(err(format!(
            "bumbledb marshal: statement {key_statement} is not a key statement"
        )));
    };
    if *key_relation != rel {
        return Err(err(format!(
            "bumbledb marshal: statement {key_statement} is not a key of relation `{name}`"
        )));
    }
    let fields = &roster(rosters, rel)?.fields;
    if values.len() as usize != projection.len() {
        return Err(err(format!(
            "bumbledb marshal: key of `{name}`: expected {} key values, got {}",
            projection.len(),
            values.len()
        )));
    }
    let mut row = Vec::with_capacity(projection.len());
    // The index rides the Array's own u32 space (arity-checked equal above),
    // so no usize→u32 cast exists to go wrong.
    for (index, field_id) in (0..values.len()).zip(projection.iter()) {
        let field = fields.get(usize::from(field_id.0)).ok_or_else(|| {
            err(format!(
                "bumbledb marshal: key of `{name}`: projection field {} out of range",
                field_id.0
            ))
        })?;
        let value = req_at::<Unknown>(values, index, format_args!("key of `{name}`"))?;
        row.push(schema_value(&field.value_type, &value, &name, &field.name)?);
    }
    Ok((rel, statement_id, row))
}

pub(crate) fn tagged_value(obj: &Object) -> napi::Result<Value> {
    let kind: String = req(obj, "kind", "value")?;
    match kind.as_str() {
        tags::value::BOOL => Ok(Value::Bool(req::<bool>(obj, "value", "bool value")?)),
        tags::value::U64 => Ok(Value::U64(u64_in(
            &req::<BigInt>(obj, "value", "u64 value")?,
            "u64 value",
        )?)),
        tags::value::I64 => Ok(Value::I64(i64_in(
            &req::<BigInt>(obj, "value", "i64 value")?,
            "i64 value",
        )?)),
        tags::value::F64 => Ok(Value::F64(F64::from(req::<f64>(
            obj,
            "value",
            "f64 value",
        )?))),
        tags::value::STRING => Ok(Value::String(
            req::<String>(obj, "value", "string value")?.into(),
        )),
        tags::value::FIXED_BYTES => Ok(Value::FixedBytes(
            req::<Uint8Array>(obj, "value", "fixedBytes value")?
                .to_vec()
                .into_boxed_slice(),
        )),
        tags::value::INTERVAL_U64 => interval_in(obj, IntervalElement::U64, "intervalU64 value"),
        tags::value::INTERVAL_I64 => interval_in(obj, IntervalElement::I64, "intervalI64 value"),
        other => Err(err(format!(
            "bumbledb marshal: unknown value kind `{other}`"
        ))),
    }
}

pub(crate) enum OwnedParam {
    Scalar(Value),
    Set(Vec<Value>),
}

pub(crate) fn params_in(arr: &Array) -> napi::Result<Vec<OwnedParam>> {
    let mut params = Vec::with_capacity(arr.len() as usize);
    for index in 0..arr.len() {
        let obj = req_at::<Object>(arr, index, "params")?;
        let kind: String = req(&obj, "kind", "param")?;
        if kind == tags::param::SET {
            let values: Array = req(&obj, "values", "set param")?;
            let mut set = Vec::with_capacity(values.len() as usize);
            for value_index in 0..values.len() {
                let element = req_at::<Object>(&values, value_index, "set param values")?;
                set.push(tagged_value(&element)?);
            }
            params.push(OwnedParam::Set(set));
        } else {
            params.push(OwnedParam::Scalar(tagged_value(&obj)?));
        }
    }
    Ok(params)
}

pub(crate) fn value_type_in(obj: &Object) -> napi::Result<ValueType> {
    let kind: String = req(obj, "kind", "value type")?;
    match kind.as_str() {
        tags::value_type::BOOL => Ok(ValueType::Bool),
        tags::value_type::U64 => Ok(ValueType::U64),
        tags::value_type::I64 => Ok(ValueType::I64),
        tags::value_type::F64 => Ok(ValueType::F64),
        tags::value_type::STRING => Ok(ValueType::String),
        tags::value_type::FIXED_BYTES => {
            let len = ordinal(req::<f64>(obj, "len", "fixedBytes type")?, "bytes width")?;
            let len = u16::try_from(len)
                .map_err(|_| err(format!("bumbledb marshal: bytes width {len} exceeds u16")))?;
            Ok(ValueType::FixedBytes { len })
        }
        tags::value_type::INTERVAL => {
            let element: String = req(obj, "element", "interval type")?;
            let element = tags::interval_element::parse(&element).ok_or_else(|| {
                err(format!(
                    "bumbledb marshal: unknown interval element `{element}`"
                ))
            })?;
            let width = obj
                .get::<BigInt>("width")?
                .map(|w| u64_in(&w, "interval width"))
                .transpose()?;
            Ok(match width {
                Some(width) => ValueType::FixedInterval { element, width },
                None => ValueType::Interval { element },
            })
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown value type kind `{other}`"
        ))),
    }
}

fn literal_in(obj: &Object) -> napi::Result<LiteralSpec> {
    let kind: String = req(obj, "kind", "literal")?;
    match kind.as_str() {
        tags::literal::HANDLE => Ok(LiteralSpec::Handle(
            req::<String>(obj, "handle", "handle literal")?.into(),
        )),
        tags::literal::VALUE => {
            let value: Object = req(obj, "value", "value literal")?;
            Ok(LiteralSpec::Value(tagged_value(&value)?))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown literal kind `{other}`"
        ))),
    }
}

fn literal_set_in(obj: &Object) -> napi::Result<LiteralSetSpec> {
    let kind: String = req(obj, "kind", "literal set")?;
    match kind.as_str() {
        tags::literal_set::ONE => {
            let literal: Object = req(obj, "literal", "one-literal binding")?;
            Ok(LiteralSetSpec::One(literal_in(&literal)?))
        }
        tags::literal_set::MANY => {
            let literals: Array = req(obj, "literals", "literal set")?;
            let mut many = Vec::with_capacity(literals.len() as usize);
            for index in 0..literals.len() {
                let literal = req_at::<Object>(&literals, index, "literal set")?;
                many.push(literal_in(&literal)?);
            }
            Ok(LiteralSetSpec::Many(many))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown literal-set kind `{other}`"
        ))),
    }
}

fn side_in(obj: &Object) -> napi::Result<SideSpec> {
    let projection: Array = req(obj, "projection", "side")?;
    let mut fields = Vec::with_capacity(projection.len() as usize);
    for index in 0..projection.len() {
        fields.push(req_at::<String>(&projection, index, "side projection")?.into());
    }
    let selection: Array = req(obj, "selection", "side")?;
    let mut bindings = Vec::with_capacity(selection.len() as usize);
    for index in 0..selection.len() {
        let pair = req_at::<Array>(&selection, index, "side selection")?;
        let field: String = req_at(&pair, 0, "selection binding")?;
        let set: Object = req_at(&pair, 1, "selection binding")?;
        bindings.push((field.into(), literal_set_in(&set)?));
    }
    Ok(SideSpec {
        relation: req::<String>(obj, "relation", "side")?.into(),
        projection: fields,
        selection: bindings,
    })
}

fn capacity_bound_in(obj: &Object) -> napi::Result<BoundSpec> {
    let kind: String = req(obj, "kind", "capacity bound")?;
    match kind.as_str() {
        tags::capacity_bound::LIT => Ok(BoundSpec::Lit(u64_in(
            &req::<BigInt>(obj, "value", "lit bound")?,
            "capacity bound",
        )?)),
        tags::capacity_bound::FIELD => Ok(BoundSpec::Field(
            req::<String>(obj, "field", "field bound")?.into(),
        )),
        tags::capacity_bound::DURATION_FIELD => Ok(BoundSpec::Duration(
            req::<String>(obj, "field", "durationField bound")?.into(),
        )),
        other => Err(err(format!(
            "bumbledb marshal: unknown capacity bound kind `{other}`"
        ))),
    }
}

fn capacity_window_in(obj: &Object) -> napi::Result<CapacityWindowSpec> {
    let kind: String = req(obj, "kind", "capacity window")?;
    match kind.as_str() {
        tags::capacity_window::EXACT => {
            Ok(CapacityWindowSpec::Exact(capacity_bound_in(
                &req::<Object>(obj, "n", "exact window")?,
            )?))
        }
        tags::capacity_window::RANGE => Ok(CapacityWindowSpec::Range {
            lo: capacity_bound_in(&req::<Object>(obj, "lo", "range window")?)?,
            hi: capacity_bound_in(&req::<Object>(obj, "hi", "range window")?)?,
        }),
        tags::capacity_window::FLOOR => {
            Ok(CapacityWindowSpec::Floor(capacity_bound_in(
                &req::<Object>(obj, "lo", "floor window")?,
            )?))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown capacity window kind `{other}`"
        ))),
    }
}

fn weight_in(obj: &Object) -> napi::Result<WeightSpec> {
    let kind: String = req(obj, "kind", "weight")?;
    match kind.as_str() {
        tags::weight::UNIT => Ok(WeightSpec::Unit),
        tags::weight::FIELD => Ok(WeightSpec::Field(
            req::<String>(obj, "field", "field weight")?.into(),
        )),
        tags::weight::DURATION_FIELD => Ok(WeightSpec::Duration(
            req::<String>(obj, "field", "durationField weight")?.into(),
        )),
        other => Err(err(format!(
            "bumbledb marshal: unknown weight kind `{other}`"
        ))),
    }
}

fn statement_in(obj: &Object) -> napi::Result<StatementSpec> {
    let kind: String = req(obj, "kind", "statement")?;
    match kind.as_str() {
        tags::statement::FD => {
            let projection: Array = req(obj, "projection", "fd statement")?;
            let mut fields = Vec::with_capacity(projection.len() as usize);
            for index in 0..projection.len() {
                fields.push(req_at::<String>(&projection, index, "fd projection")?.into());
            }
            Ok(StatementSpec::Fd {
                relation: req::<String>(obj, "relation", "fd statement")?.into(),
                projection: fields,
            })
        }
        tags::statement::CONTAINMENT => Ok(StatementSpec::Containment {
            source: side_in(&req::<Object>(obj, "source", "containment")?)?,
            target: side_in(&req::<Object>(obj, "target", "containment")?)?,
            bidirectional: req::<bool>(obj, "bidirectional", "containment")?,
        }),
        tags::statement::CAPACITY => Ok(StatementSpec::Capacity {
            target: side_in(&req::<Object>(obj, "target", "capacity")?)?,
            weight: weight_in(&req::<Object>(obj, "weight", "capacity")?)?,
            window: capacity_window_in(&req::<Object>(obj, "window", "capacity")?)?,
            source: side_in(&req::<Object>(obj, "source", "capacity")?)?,
        }),
        other => Err(err(format!(
            "bumbledb marshal: unknown statement kind `{other}`"
        ))),
    }
}

pub(crate) fn schema_spec(obj: &Object) -> napi::Result<SchemaSpec> {
    let relations: Array = req(obj, "relations", "schema spec")?;
    let mut relation_specs = Vec::with_capacity(relations.len() as usize);
    for index in 0..relations.len() {
        let relation = req_at::<Object>(&relations, index, "spec relations")?;
        let fields: Array = req(&relation, "fields", "relation spec")?;
        let mut field_specs = Vec::with_capacity(fields.len() as usize);
        for field_index in 0..fields.len() {
            let field = req_at::<Object>(&fields, field_index, "relation fields")?;
            let value_type: Object = req(&field, "valueType", "field spec")?;
            field_specs.push(FieldSpec {
                name: req::<String>(&field, "name", "field spec")?.into(),
                value_type: value_type_in(&value_type)?,
                newtype: field.get::<String>("newtype")?.map(Into::into),
                fresh: req::<bool>(&field, "fresh", "field spec")?,
            });
        }
        // Closedness as one sum, mirroring the fused `RelationSpec`
        // (ruled 2026-07-23, R7): an absent `closed` key is an ordinary
        // relation; a present one carries handle newtype + ground axioms
        // together — the two illegal states are unspellable on the wire
        // exactly as they are unrepresentable in the spec.
        let closed = match relation.get::<Object>("closed")? {
            None => None,
            Some(closed) => {
                let rows: Array = req(&closed, "rows", "closed relation")?;
                let mut row_specs = Vec::with_capacity(rows.len() as usize);
                for row_index in 0..rows.len() {
                    let row = req_at::<Object>(&rows, row_index, "closed relation rows")?;
                    let values: Array = req(&row, "values", "closed row")?;
                    let mut literals = Vec::with_capacity(values.len() as usize);
                    for value_index in 0..values.len() {
                        let literal = req_at::<Object>(&values, value_index, "closed row")?;
                        literals.push(literal_in(&literal)?);
                    }
                    row_specs.push(RowSpec {
                        handle: req::<String>(&row, "handle", "closed row")?.into(),
                        values: literals,
                    });
                }
                Some(ClosedSpec {
                    newtype: req::<String>(&closed, "newtype", "closed relation")?.into(),
                    rows: row_specs,
                })
            }
        };
        relation_specs.push(RelationSpec {
            name: req::<String>(&relation, "name", "relation spec")?.into(),
            fields: field_specs,
            closed,
        });
    }
    let statements: Array = req(obj, "statements", "schema spec")?;
    let mut statement_specs = Vec::with_capacity(statements.len() as usize);
    for index in 0..statements.len() {
        let statement = req_at::<Object>(&statements, index, "spec statements")?;
        statement_specs.push(statement_in(&statement)?);
    }
    Ok(SchemaSpec {
        relations: relation_specs,
        statements: statement_specs,
    })
}

fn var_in(obj: &Object, key: &str, ctx: &str) -> napi::Result<VarId> {
    Ok(VarId(u16_id(
        ordinal(req::<f64>(obj, key, ctx)?, ctx)?,
        ctx,
    )?))
}

fn param_in(obj: &Object, key: &str, ctx: &str) -> napi::Result<ParamId> {
    Ok(ParamId(u16_id(
        ordinal(req::<f64>(obj, key, ctx)?, ctx)?,
        ctx,
    )?))
}

fn term_in(obj: &Object) -> napi::Result<Term> {
    let kind: String = req(obj, "kind", "term")?;
    match kind.as_str() {
        tags::term::VAR => Ok(Term::Var(var_in(obj, "var", "var term")?)),
        tags::term::PARAM => Ok(Term::Param(param_in(obj, "param", "param term")?)),
        tags::term::PARAM_SET => Ok(Term::ParamSet(param_in(obj, "param", "paramSet term")?)),
        tags::term::LITERAL => {
            let value: Object = req(obj, "value", "literal term")?;
            Ok(Term::Literal(tagged_value(&value)?))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown term kind `{other}`"
        ))),
    }
}

fn head_term_in(obj: &Object) -> napi::Result<HeadTerm> {
    let kind: String = req(obj, "kind", "head term")?;
    match kind.as_str() {
        tags::head_term::VAR => Ok(HeadTerm::Var),
        tags::head_term::AGGREGATE => {
            let op: String = req(obj, "op", "head aggregate")?;
            let op = tags::head_op::parse(&op)
                .ok_or_else(|| err(format!("bumbledb marshal: unknown head op `{op}`")))?;
            Ok(HeadTerm::Aggregate(op))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown head term kind `{other}`"
        ))),
    }
}

fn fold_op_in(obj: &Object) -> napi::Result<FoldOp> {
    let kind: String = req(obj, "kind", "fold op")?;
    let op = tags::head_op::parse(&kind)
        .ok_or_else(|| err(format!("bumbledb marshal: unknown fold op `{kind}`")))?;
    match op {
        HeadOp::Sum => Ok(FoldOp::Sum),
        HeadOp::Mean => Ok(FoldOp::Mean),
        HeadOp::Min => Ok(FoldOp::Min),
        HeadOp::Max => Ok(FoldOp::Max),
        HeadOp::Count => Err(err(
            "bumbledb marshal: Count is find kind `count`, not a fold".to_string(),
        )),
        HeadOp::Pack => Err(err(
            "bumbledb marshal: Pack is find kind `pack`, not a fold".to_string(),
        )),
    }
}

fn find_term_in(obj: &Object) -> napi::Result<FindTerm> {
    let kind: String = req(obj, "kind", "find term")?;
    match kind.as_str() {
        tags::find_term::VAR => Ok(FindTerm::Var(var_in(obj, "var", "var find")?)),
        tags::find_term::COUNT => {
            if obj.get::<f64>("over")?.is_some() {
                return Err(err("bumbledb marshal: Count carries no over".to_string()));
            }
            Ok(FindTerm::Count)
        }
        tags::find_term::PACK => Ok(FindTerm::Pack {
            over: var_in(obj, "over", "pack find")?,
        }),
        tags::find_term::AGGREGATE => {
            let op: Object = req(obj, "op", "aggregate find")?;
            Ok(FindTerm::Aggregate {
                op: fold_op_in(&op)?,
                over: var_in(obj, "over", "aggregate find")?,
            })
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown find term kind `{other}`"
        ))),
    }
}

fn atom_in(obj: &Object) -> napi::Result<Atom> {
    let source: Object = req(obj, "source", "atom")?;
    let source_kind: String = req(&source, "kind", "atom source")?;
    let source = match source_kind.as_str() {
        tags::atom_source::EDB => AtomSource::Edb(RelationId(ordinal(
            req::<f64>(&source, "relation", "edb source")?,
            "edb relation",
        )?)),
        tags::atom_source::INTERIOR => AtomSource::Interior(InteriorId(ordinal(
            req::<f64>(&source, "interior", "interior source")?,
            "interior id",
        )?)),
        other => {
            return Err(err(format!(
                "bumbledb marshal: unknown atom source kind `{other}`"
            )));
        }
    };
    let bindings: Array = req(obj, "bindings", "atom")?;
    let mut bound = Vec::with_capacity(bindings.len() as usize);
    for index in 0..bindings.len() {
        let pair = req_at::<Array>(&bindings, index, "atom bindings")?;
        let field = FieldId(u16_id(
            ordinal(
                req_at::<f64>(&pair, 0, "atom binding field")?,
                "binding field",
            )?,
            "binding field",
        )?);
        let term: Object = req_at(&pair, 1, "atom binding")?;
        bound.push((field, term_in(&term)?));
    }
    Ok(Atom {
        source,
        bindings: bound,
    })
}

fn comparison_in(obj: &Object) -> napi::Result<Comparison> {
    let op: Object = req(obj, "op", "comparison")?;
    let op_kind: String = req(&op, "kind", "comparison op")?;
    let op = match op_kind.as_str() {
        tags::cmp_op::EQ => CmpOp::Eq,
        tags::cmp_op::NE => CmpOp::Ne,
        tags::cmp_op::LT => CmpOp::Lt,
        tags::cmp_op::LE => CmpOp::Le,
        tags::cmp_op::GT => CmpOp::Gt,
        tags::cmp_op::GE => CmpOp::Ge,
        tags::cmp_op::POINT_IN => CmpOp::PointIn,
        tags::cmp_op::ALLEN => {
            let bits = ordinal(req::<f64>(&op, "mask", "allen mask")?, "allen mask")?;
            let mask = u16::try_from(bits)
                .ok()
                .and_then(AllenMask::new)
                .ok_or_else(|| err(format!("bumbledb marshal: invalid allen mask bits {bits}")))?;
            CmpOp::Allen { mask }
        }
        other => {
            return Err(err(format!(
                "bumbledb marshal: unknown comparison op `{other}`"
            )));
        }
    };
    let lhs: Object = req(obj, "lhs", "comparison")?;
    let rhs: Object = req(obj, "rhs", "comparison")?;
    Ok(Comparison {
        op,
        lhs: term_in(&lhs)?,
        rhs: term_in(&rhs)?,
    })
}

fn condition_in(obj: &Object, depth: usize) -> napi::Result<ConditionTree> {
    if depth > bumbledb::MAX_CONDITION_DEPTH {
        return Err(err(format!(
            "bumbledb marshal: condition tree deeper than {} (the engine's MAX_CONDITION_DEPTH)",
            bumbledb::MAX_CONDITION_DEPTH
        )));
    }
    let kind: String = req(obj, "kind", "condition")?;
    match kind.as_str() {
        tags::condition::LEAF => {
            let cmp: Object = req(obj, "cmp", "leaf condition")?;
            Ok(ConditionTree::Leaf(comparison_in(&cmp)?))
        }
        tags::condition::AND => Ok(ConditionTree::And(condition_children(obj, depth)?)),
        tags::condition::OR => Ok(ConditionTree::Or(condition_children(obj, depth)?)),
        other => Err(err(format!(
            "bumbledb marshal: unknown condition kind `{other}`"
        ))),
    }
}

fn condition_children(obj: &Object, depth: usize) -> napi::Result<Vec<ConditionTree>> {
    let children: Array = req(obj, "children", "condition")?;
    let mut trees = Vec::with_capacity(children.len() as usize);
    for index in 0..children.len() {
        let child = req_at::<Object>(&children, index, "condition children")?;
        trees.push(condition_in(&child, depth + 1)?);
    }
    Ok(trees)
}

fn rule_in(obj: &Object) -> napi::Result<Rule> {
    let finds: Array = req(obj, "finds", "rule")?;
    let mut find_terms = Vec::with_capacity(finds.len() as usize);
    for index in 0..finds.len() {
        let find = req_at::<Object>(&finds, index, "rule finds")?;
        find_terms.push(find_term_in(&find)?);
    }
    let atoms: Array = req(obj, "atoms", "rule")?;
    let mut atom_list = Vec::with_capacity(atoms.len() as usize);
    for index in 0..atoms.len() {
        let atom = req_at::<Object>(&atoms, index, "rule atoms")?;
        atom_list.push(atom_in(&atom)?);
    }
    let negated: Array = req(obj, "negated", "rule")?;
    let mut negated_list = Vec::with_capacity(negated.len() as usize);
    for index in 0..negated.len() {
        let atom = req_at::<Object>(&negated, index, "rule negated atoms")?;
        negated_list.push(atom_in(&atom)?);
    }
    let conditions: Array = req(obj, "conditions", "rule")?;
    let mut condition_list = Vec::with_capacity(conditions.len() as usize);
    for index in 0..conditions.len() {
        let condition = req_at::<Object>(&conditions, index, "rule conditions")?;
        condition_list.push(condition_in(&condition, 1)?);
    }
    Ok(Rule {
        finds: find_terms,
        atoms: atom_list,
        negated: negated_list,
        conditions: condition_list,
    })
}

fn head_in(obj: &Object, ctx: &str) -> napi::Result<Vec<HeadTerm>> {
    let head: Array = req(obj, "head", ctx)?;
    let mut head_terms = Vec::with_capacity(head.len() as usize);
    for head_index in 0..head.len() {
        let term = req_at::<Object>(&head, head_index, ctx)?;
        head_terms.push(head_term_in(&term)?);
    }
    Ok(head_terms)
}

fn rules_in(obj: &Object, key: &str, ctx: &str) -> napi::Result<Vec<Rule>> {
    let rules: Array = req(obj, key, ctx)?;
    let mut rule_list = Vec::with_capacity(rules.len() as usize);
    for rule_index in 0..rules.len() {
        let rule = req_at::<Object>(&rules, rule_index, ctx)?;
        rule_list.push(rule_in(&rule)?);
    }
    Ok(rule_list)
}

fn vars_only(finds: &[FindTerm]) -> napi::Result<Vec<VarId>> {
    finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => Ok(*var),
            _ => Err(err(
                "bumbledb marshal: derived-table finds are variables only".to_string(),
            )),
        })
        .collect()
}

fn projection_rule_in(obj: &Object) -> napi::Result<ProjectionRule> {
    let rule = rule_in(obj)?;
    Ok(ProjectionRule {
        finds: vars_only(&rule.finds)?,
        atoms: rule.atoms,
        negated: rule.negated,
        conditions: rule.conditions,
    })
}

fn rec_rule_in(obj: &Object) -> napi::Result<RecRule> {
    let rule = rule_in(obj)?;
    if !rule.negated.is_empty() {
        return Err(err(
            "bumbledb marshal: negation is unrepresentable in rec".to_string()
        ));
    }
    Ok(RecRule {
        finds: vars_only(&rule.finds)?,
        atoms: rule.atoms,
        conditions: rule.conditions,
    })
}

fn rec_step_in(obj: &Object, rec_id: InteriorId) -> napi::Result<RecStep> {
    let rule = rule_in(obj)?;
    if !rule.negated.is_empty() {
        return Err(err(
            "bumbledb marshal: negation is unrepresentable in rec".to_string()
        ));
    }
    let mut self_bindings = None;
    let mut atoms = Vec::new();
    for atom in rule.atoms {
        if atom.source.interior() == Some(rec_id) {
            if self_bindings.is_some() {
                return Err(err(
                    "bumbledb marshal: rec step has two self-atoms".to_string()
                ));
            }
            self_bindings = Some(atom.bindings);
        } else {
            atoms.push(atom);
        }
    }
    Ok(RecStep {
        finds: vars_only(&rule.finds)?,
        self_bindings: self_bindings
            .ok_or_else(|| err("bumbledb marshal: rec step missing self-atom".to_string()))?,
        atoms,
        conditions: rule.conditions,
    })
}

fn nonempty<T>(items: Vec<T>, what: &str) -> napi::Result<NonEmpty<T>> {
    NonEmpty::from_vec(items).ok_or_else(|| err(format!("bumbledb marshal: empty {what}")))
}

// The wire's rec `head` is the TS builder's alignment datum; the core
// `Rec` carries no head (the engine recomputes it from finds), so the
// bridge reads the arms only.
fn rec_in(obj: &Object, rec_id: InteriorId) -> napi::Result<Rec> {
    let base_arr: Array = req(obj, "base", "rec base")?;
    let mut base = Vec::with_capacity(base_arr.len() as usize);
    for index in 0..base_arr.len() {
        let rule = req_at::<Object>(&base_arr, index, "rec base")?;
        base.push(rec_rule_in(&rule)?);
    }
    let rec_arr: Array = req(obj, "rec", "rec arms")?;
    let mut rec = Vec::with_capacity(rec_arr.len() as usize);
    for index in 0..rec_arr.len() {
        let rule = req_at::<Object>(&rec_arr, index, "rec arms")?;
        rec.push(rec_step_in(&rule, rec_id)?);
    }
    Ok(Rec {
        base: nonempty(base, "rec base")?,
        rec: nonempty(rec, "rec step")?,
    })
}

fn interiors_in(obj: &Object) -> napi::Result<Vec<Interior>> {
    let interiors_arr: Array = req(obj, "interiors", "query")?;
    let mut interiors = Vec::with_capacity(interiors_arr.len() as usize);
    // The wire's interior `head` is the TS builder's alignment datum; the
    // core `Interior` carries no head (the engine recomputes it from
    // finds), so the bridge reads the rules only.
    for index in 0..interiors_arr.len() {
        let interior = req_at::<Object>(&interiors_arr, index, "query interiors")?;
        let rules_arr: Array = req(&interior, "rules", "interior rules")?;
        let mut rules = Vec::with_capacity(rules_arr.len() as usize);
        for rule_index in 0..rules_arr.len() {
            let rule = req_at::<Object>(&rules_arr, rule_index, "interior rules")?;
            rules.push(projection_rule_in(&rule)?);
        }
        interiors.push(Interior { rules });
    }
    Ok(interiors)
}

pub(crate) fn query_in(obj: &Object) -> napi::Result<Query> {
    let kind: String = req(obj, "kind", "query")?;
    let interiors = interiors_in(obj)?;
    match kind.as_str() {
        tags::query::CQ => Ok(Query {
            interiors,
            head: head_in(obj, "query head")?,
            rules: rules_in(obj, "rules", "query rules")?,
            rec: None,
        }),
        tags::query::REACH => {
            let rec_obj: Object = req(obj, "rec", "reach query")?;
            let rec_id = InteriorId(
                u32::try_from(interiors.len())
                    .map_err(|_| err("bumbledb marshal: interior count".to_string()))?,
            );
            Ok(Query {
                interiors,
                rec: Some(rec_in(&rec_obj, rec_id)?),
                head: head_in(obj, "query head")?,
                rules: rules_in(obj, "rules", "query rules")?,
            })
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown query kind `{other}`"
        ))),
    }
}

pub enum ValueOut {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(F64),
    Text(String),
    Bytes(Vec<u8>),
    IntervalU64 { start: u64, end: u64 },
    IntervalI64 { start: i64, end: i64 },
}

impl ValueOut {
    /// Consumes the engine value — string and bytes payloads MOVE (the
    /// one-copy crossing: every call site owns its `Value`, so a borrowing
    /// twin would only re-copy what is about to drop). Non-UTF-8 string
    /// bytes are refused typed, the outbound twin of `param_args`'s
    /// inbound refusal — the store's decode lanes can surface at-rest
    /// damage, and a repair (`from_utf8_lossy`) would silently corrupt
    /// what the engine's own corruption taxonomy convicts.
    pub(crate) fn from_value(value: Value) -> Self {
        match value {
            Value::Bool(v) => Self::Bool(v),
            Value::U64(v) => Self::U64(v),
            Value::I64(v) => Self::I64(v),
            Value::F64(v) => Self::F64(v),
            Value::String(text) => Self::Text(text.into()),
            Value::FixedBytes(bytes) => Self::Bytes(bytes.into_vec()),
            Value::IntervalU64(interval) => Self::IntervalU64 {
                start: interval.start(),
                end: interval.end(),
            },
            Value::IntervalI64(interval) => Self::IntervalI64 {
                start: interval.start(),
                end: interval.end(),
            },
        }
    }
}

impl ToNapiValue for ValueOut {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; every arm \
                  delegates to napi's own impls on the same live env"
    )]
    // SAFETY (each delegation below): `env` is the live environment napi
    // handed this very call; the interval arms' objects were created
    // against it lines above.
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        match val {
            Self::Bool(v) => unsafe { bool::to_napi_value(env, v) },
            Self::U64(v) => unsafe { u64::to_napi_value(env, v) },
            Self::I64(v) => unsafe { i64n::to_napi_value(env, i64n(v)) },
            Self::F64(v) => unsafe { f64::to_napi_value(env, v.to_f64()) },
            Self::Text(v) => unsafe { String::to_napi_value(env, v) },
            Self::Bytes(v) => unsafe { Uint8Array::to_napi_value(env, Uint8Array::new(v)) },
            Self::IntervalU64 { start, end } => {
                let env_handle = Env::from_raw(env);
                let mut obj = Object::new(&env_handle)?;
                obj.set("start", start)?;
                obj.set("end", end)?;
                unsafe { Object::to_napi_value(env, obj) }
            }
            Self::IntervalI64 { start, end } => {
                let env_handle = Env::from_raw(env);
                let mut obj = Object::new(&env_handle)?;
                obj.set("start", i64n(start))?;
                obj.set("end", i64n(end))?;
                unsafe { Object::to_napi_value(env, obj) }
            }
        }
    }
}

pub(crate) fn rows_out(rows: Vec<Vec<Value>>) -> Vec<Vec<ValueOut>> {
    rows.into_iter()
        .map(|row| row.into_iter().map(ValueOut::from_value).collect())
        .collect()
}

pub(crate) fn answers_out(answers: &Answers) -> Vec<Vec<ValueOut>> {
    (0..answers.len())
        .map(|row| {
            (0..answers.arity())
                .map(|column| match answers.get(row, column) {
                    AnswerValue::Bool(v) => ValueOut::Bool(v),
                    AnswerValue::U64(v) => ValueOut::U64(v),
                    AnswerValue::I64(v) => ValueOut::I64(v),
                    AnswerValue::F64(v) => ValueOut::F64(v),
                    AnswerValue::String(v) => ValueOut::Text(v.to_owned()),
                    AnswerValue::FixedBytes(v) => ValueOut::Bytes(v.to_vec()),
                    AnswerValue::IntervalU64(v) => ValueOut::IntervalU64 {
                        start: v.start(),
                        end: v.end(),
                    },
                    AnswerValue::IntervalI64(v) => ValueOut::IntervalI64 {
                        start: v.start(),
                        end: v.end(),
                    },
                })
                .collect()
        })
        .collect()
}

fn statement_kind_out(kind: StatementKind) -> &'static str {
    tags::statement_kind::tag(&kind)
}

#[expect(
    unsafe_code,
    reason = "the rendered object crosses back through napi's own \
              `Object::to_napi_value` on the same live env"
)]
fn value_type_out(env: sys::napi_env, ty: &ValueType) -> napi::Result<sys::napi_value> {
    let env_handle = Env::from_raw(env);
    let mut obj = Object::new(&env_handle)?;
    // The kind rides THE one table (`tags::value_type` — the same table
    // `value_type_in` parses: the old in/out twin is one datum); only the
    // payload attributes are matched here.
    obj.set("kind", tags::value_type::tag(ty))?;
    match ty {
        ValueType::Bool | ValueType::U64 | ValueType::I64 | ValueType::F64 | ValueType::String => {}
        ValueType::FixedBytes { len } => {
            obj.set("len", u32::from(*len))?;
        }
        ValueType::Interval { element } => {
            obj.set("element", tags::interval_element::tag(element))?;
        }
        ValueType::FixedInterval { element, width } => {
            obj.set("element", tags::interval_element::tag(element))?;
            obj.set("width", *width)?;
        }
    }
    // SAFETY: `env` is the live environment the calling impl received from
    // napi, and `obj` was created against it.
    unsafe { Object::to_napi_value(env, obj) }
}

#[expect(
    unsafe_code,
    reason = "napi's Unknown::from_raw_unchecked rewraps a value this helper just rendered"
)]
fn relation_objects<'env>(
    env: sys::napi_env,
    env_handle: &'env Env,
    relations: Vec<RelationManifest>,
    attrs: &[Vec<FieldAttrs>],
) -> napi::Result<Vec<Object<'env>>> {
    let mut out = Vec::with_capacity(relations.len());
    for (rel_index, relation) in relations.into_iter().enumerate() {
        let rel_attrs = attrs.get(rel_index).ok_or_else(|| {
            err(format!(
                "bumbledb marshal: relation `{}` has no spec attribute rows",
                relation.name
            ))
        })?;
        let mut rel_obj = Object::new(env_handle)?;
        rel_obj.set("name", relation.name.as_ref())?;
        rel_obj.set("id", relation.id.0)?;
        let mut fields = Vec::with_capacity(relation.fields.len());
        for (field_index, field) in relation.fields.into_iter().enumerate() {
            let attr = rel_attrs.get(field_index).ok_or_else(|| {
                err(format!(
                    "bumbledb marshal: relation `{}` field `{}` has no spec attribute row",
                    relation.name, field.name
                ))
            })?;
            let mut field_obj = Object::new(env_handle)?;
            field_obj.set("name", field.name.as_ref())?;
            field_obj.set("id", u32::from(field.id.0))?;
            let ty = value_type_out(env, &field.value_type)?;
            // SAFETY: `ty` is the napi value `value_type_out` just
            // rendered against this same live `env`, one line up.
            let ty = unsafe { Unknown::from_raw_unchecked(env, ty) };
            field_obj.set("valueType", ty)?;
            field_obj.set("fresh", attr.fresh)?;
            if let Some(newtype) = &attr.newtype {
                field_obj.set("newtype", newtype.as_ref())?;
            }
            fields.push(field_obj);
        }
        rel_obj.set("fields", fields)?;
        if let Some(extension) = relation.extension {
            let mut rows = Vec::with_capacity(extension.len());
            for row in extension {
                let mut row_obj = Object::new(env_handle)?;
                row_obj.set("handle", row.handle.as_ref())?;
                row_obj.set("id", row.id)?;
                let mut values = Vec::with_capacity(row.values.len());
                for (name, value) in row.values {
                    let mut value_obj = Object::new(env_handle)?;
                    value_obj.set("name", name.as_ref())?;
                    value_obj.set("value", ValueOut::from_value(value))?;
                    values.push(value_obj);
                }
                row_obj.set("values", values)?;
                rows.push(row_obj);
            }
            rel_obj.set("extension", rows)?;
        }
        out.push(rel_obj);
    }
    Ok(out)
}

fn side_object<'env>(env_handle: &'env Env, side: &Side) -> napi::Result<Object<'env>> {
    let mut obj = Object::new(env_handle)?;
    obj.set("relation", side.relation.0)?;
    let projection: Vec<u32> = side
        .projection
        .iter()
        .map(|field| u32::from(field.0))
        .collect();
    obj.set("projection", projection)?;
    let mut selection = Vec::with_capacity(side.selection.len());
    for (field, set) in &side.selection {
        let mut binding = Object::new(env_handle)?;
        binding.set("field", u32::from(field.0))?;
        let values: Vec<ValueOut> = set
            .literals()
            .iter()
            .cloned()
            .map(ValueOut::from_value)
            .collect();
        binding.set("values", values)?;
        selection.push(binding);
    }
    obj.set("selection", selection)?;
    Ok(obj)
}

fn weight_object(env_handle: &Env, weight: Weight) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env_handle)?;
    match weight {
        Weight::Unit => {
            obj.set("kind", "unit")?;
        }
        Weight::Field(field) => {
            obj.set("kind", "field")?;
            obj.set("field", u32::from(field.0))?;
        }
        Weight::DurationOf(field) => {
            obj.set("kind", "duration")?;
            obj.set("field", u32::from(field.0))?;
        }
    }
    Ok(obj)
}

fn hi_object(env_handle: &Env, hi: Option<Bound>) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env_handle)?;
    match hi {
        None => {
            obj.set("kind", "unbounded")?;
        }
        Some(Bound::Lit(value)) => {
            obj.set("kind", "lit")?;
            obj.set("value", value)?;
        }
        Some(Bound::TargetField(field)) => {
            obj.set("kind", "targetField")?;
            obj.set("field", u32::from(field.0))?;
        }
        Some(Bound::TargetDuration(field)) => {
            obj.set("kind", "targetDuration")?;
            obj.set("field", u32::from(field.0))?;
        }
    }
    Ok(obj)
}

fn statement_object(
    env_handle: &Env,
    id: u32,
    statement: StatementDescriptor,
) -> napi::Result<Object<'_>> {
    let mut obj = Object::new(env_handle)?;
    obj.set("id", id)?;
    match statement {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => {
            obj.set("kind", statement_kind_out(StatementKind::Functionality))?;
            obj.set("relation", relation.0)?;
            let fields: Vec<u32> = projection.iter().map(|field| u32::from(field.0)).collect();
            obj.set("projection", fields)?;
        }
        StatementDescriptor::Containment { source, target } => {
            obj.set("kind", statement_kind_out(StatementKind::Containment))?;
            obj.set("source", side_object(env_handle, &source)?)?;
            obj.set("target", side_object(env_handle, &target)?)?;
        }
        StatementDescriptor::Capacity {
            target,
            weight,
            lo,
            hi,
            source,
        } => {
            obj.set("kind", statement_kind_out(StatementKind::Capacity))?;
            obj.set("target", side_object(env_handle, &target)?)?;
            obj.set("weight", weight_object(env_handle, weight)?)?;
            obj.set("lo", lo)?;
            obj.set("hi", hi_object(env_handle, hi)?)?;
            obj.set("source", side_object(env_handle, &source)?)?;
        }
    }
    Ok(obj)
}

pub struct ManifestWire {
    pub(crate) manifest: Manifest,
    pub(crate) attrs: Vec<Vec<FieldAttrs>>,
}

impl ToNapiValue for ManifestWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl \
                  builds plain objects on the live env and rewraps one raw \
                  value it just rendered against that same env"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let manifest = val.manifest;
        let mut root = Object::new(&env_handle)?;
        root.set(
            "relations",
            relation_objects(env, &env_handle, manifest.relations, &val.attrs)?,
        )?;
        let mut statements = Vec::with_capacity(manifest.statements.len());
        for statement in manifest.statements {
            let mut statement_obj = Object::new(&env_handle)?;
            statement_obj.set("id", u32::from(statement.id.0))?;
            statement_obj.set("kind", statement_kind_out(statement.kind))?;
            statement_obj.set("spelling", statement.spelling)?;
            statements.push(statement_obj);
        }
        root.set("statements", statements)?;
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

pub struct DescriptorWire {
    pub(crate) manifest: Manifest,
    pub(crate) statements: Vec<StatementDescriptor>,
    pub(crate) fingerprint: String,
    pub(crate) attrs: Vec<Vec<FieldAttrs>>,
}

impl ToNapiValue for DescriptorWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl \
                  builds plain objects on the live env and rewraps one raw \
                  value it just rendered against that same env"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut root = Object::new(&env_handle)?;
        root.set(
            "relations",
            relation_objects(env, &env_handle, val.manifest.relations, &val.attrs)?,
        )?;
        let mut statements = Vec::with_capacity(val.statements.len());
        for (idx, statement) in val.statements.into_iter().enumerate() {
            let id = u32::try_from(idx).map_err(|_| {
                err(format!(
                    "bumbledb marshal: statement ordinal {idx} exceeds u32"
                ))
            })?;
            statements.push(statement_object(&env_handle, id, statement)?);
        }
        root.set("statements", statements)?;
        root.set("fingerprint", val.fingerprint)?;
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `root` was created against it.
        unsafe { Object::to_napi_value(env, root) }
    }
}

pub struct ViolationWire {
    pub(crate) statement: u16,
    pub(crate) kind: StatementKind,
    pub(crate) canonical: String,
    pub(crate) direction: Option<&'static str>,
    pub(crate) measure: Option<u128>,
    pub(crate) facts: Vec<(String, Vec<(String, Value)>)>,
}

impl ViolationWire {
    pub(crate) fn from_rendered(rendered: RenderedViolation) -> Self {
        let facts = |facts: Vec<bumbledb::RenderedFact>| {
            facts
                .into_iter()
                .map(|fact| {
                    (
                        fact.relation.into_string(),
                        fact.fields
                            .into_iter()
                            .map(|(name, value)| (name.into_string(), value))
                            .collect(),
                    )
                })
                .collect()
        };
        match rendered {
            RenderedViolation::Functionality {
                statement,
                spelling,
                facts: rendered_facts,
            } => Self {
                statement: statement.0,
                kind: StatementKind::Functionality,
                canonical: spelling,
                direction: None,
                measure: None,
                facts: facts(rendered_facts),
            },
            RenderedViolation::Containment {
                statement,
                spelling,
                direction,
                facts: rendered_facts,
            } => Self {
                statement: statement.0,
                kind: StatementKind::Containment,
                canonical: spelling,
                direction: Some(tags::direction::tag(&direction)),
                measure: None,
                facts: facts(rendered_facts),
            },
            RenderedViolation::Capacity {
                statement,
                spelling,
                measure,
                facts: rendered_facts,
            } => Self {
                statement: statement.0,
                kind: StatementKind::Capacity,
                canonical: spelling,
                direction: None,
                measure: Some(measure),
                facts: facts(rendered_facts),
            },
        }
    }
}

impl ToNapiValue for ViolationWire {
    #[expect(
        unsafe_code,
        reason = "napi declares `ToNapiValue::to_napi_value` unsafe; the impl \
                  only builds plain objects and delegates to napi's own impls"
    )]
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        obj.set("statementId", u32::from(val.statement))?;
        obj.set("kind", statement_kind_out(val.kind))?;
        obj.set("canonical", val.canonical)?;
        if let Some(direction) = val.direction {
            obj.set("direction", direction)?;
        }
        if let Some(measure) = val.measure {
            // u128 → BigInt, whole (C3): two little-endian u64 words.
            obj.set("measure", BigInt::from(measure))?;
        }
        let mut facts = Vec::with_capacity(val.facts.len());
        for (relation, fields) in val.facts {
            let mut fact_obj = Object::new(&env_handle)?;
            fact_obj.set("relation", relation)?;
            let mut field_objs = Vec::with_capacity(fields.len());
            for (name, value) in fields {
                let mut field_obj = Object::new(&env_handle)?;
                field_obj.set("name", name)?;
                field_obj.set("value", ValueOut::from_value(value))?;
                field_objs.push(field_obj);
            }
            fact_obj.set("fields", field_objs)?;
            facts.push(fact_obj);
        }
        obj.set("facts", facts)?;
        // SAFETY: `env` is the live environment napi handed this very call,
        // and `obj` was created against it.
        unsafe { Object::to_napi_value(env, obj) }
    }
}
