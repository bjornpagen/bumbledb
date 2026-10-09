//! The data plane: JS-native cells, params and point-read keys in, owned
//! row values out. Schema-dependent and hand-walked; nothing here judges
//! semantics beyond the declared cell types.
use bumbledb::schema::{IntervalElement, StatementDescriptor, ValueType};
use bumbledb::{
    AnswerValue, Direction, F64, Interval, RelationId, RenderedViolation, StatementId, Uuid, Value,
};
use napi::bindgen_prelude::{
    Array, BigInt, Either, Either6, Env, FromNapiValue, Object, ToNapiValue, Uint8Array,
    Utf16String, i64n,
};
use napi::{Unknown, ValueType as JsType, sys};
use napi_derive::napi;

use crate::runtime::RuntimeError;
use crate::schema::SchemaHandle;
use crate::tags;

pub(crate) fn err(message: String) -> RuntimeError {
    RuntimeError::InvalidValue { message }
}

pub(crate) fn engine_message(error: &bumbledb::Error) -> String {
    error.to_string()
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
) -> Result<T, RuntimeError> {
    obj.get::<T>(key)?
        .ok_or_else(|| err(format!("bumbledb marshal: missing `{key}` in {ctx}")))
}

pub(crate) fn req_at<T: FromNapiValue>(
    arr: &Array,
    index: u32,
    ctx: impl std::fmt::Display,
) -> Result<T, RuntimeError> {
    arr.get::<T>(index)?.ok_or_else(|| {
        err(format!(
            "bumbledb marshal: missing element {index} in {ctx}"
        ))
    })
}

// N-API's UTF-8 conversion replaces unpaired JS surrogates. Read code units
// instead so malformed input is refused, never changed into a different fact.
fn string_in(value: &Utf16String, ctx: impl std::fmt::Display) -> Result<String, RuntimeError> {
    String::from_utf16(value).map_err(|_| {
        err(format!(
            "bumbledb marshal: {ctx}: expected well-formed Unicode text"
        ))
    })
}

fn req_text(
    obj: &Object,
    key: &str,
    ctx: impl std::fmt::Display + Copy,
) -> Result<String, RuntimeError> {
    string_in(&req::<Utf16String>(obj, key, ctx)?, ctx)
}

pub(crate) fn u64_in(value: &BigInt, ctx: impl std::fmt::Display) -> Result<u64, RuntimeError> {
    let (sign, word, lossless) = value.get_u64();
    if sign || !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of u64 range"
        )));
    }
    Ok(word)
}

pub(crate) fn i64_in(value: &BigInt, ctx: impl std::fmt::Display) -> Result<i64, RuntimeError> {
    let (word, lossless) = value.get_i64();
    if !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of i64 range"
        )));
    }
    Ok(word)
}

pub(crate) fn u16_id(value: u32, ctx: &str) -> Result<u16, RuntimeError> {
    u16::try_from(value)
        .map_err(|_| err(format!("bumbledb marshal: {ctx}: id {value} exceeds u16")))
}

fn interval_u64_in(
    obj: &Object,
    ctx: impl std::fmt::Display + Copy,
) -> Result<Interval<u64>, RuntimeError> {
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
) -> Result<Interval<i64>, RuntimeError> {
    let start = i64_in(&req::<BigInt>(obj, "start", ctx)?, ctx)?;
    let end = i64_in(&req::<BigInt>(obj, "end", ctx)?, ctx)?;
    Interval::<i64>::new(start, end).ok_or_else(|| {
        err(format!(
            "bumbledb marshal: {ctx}: empty interval (start {start} >= end {end})"
        ))
    })
}

fn interval_f64_in(
    obj: &Object,
    ctx: impl std::fmt::Display + Copy,
) -> Result<Interval<F64>, RuntimeError> {
    let start_raw = req::<f64>(obj, "start", ctx)?;
    let end_raw = req::<f64>(obj, "end", ctx)?;
    let start = F64::from(start_raw);
    let end = F64::from(end_raw);
    Interval::<F64>::new(start, end).ok_or_else(|| {
        err(format!(
            "bumbledb marshal: {ctx}: not a dense-line interval (need non-NaN start < end, got start {start_raw} end {end_raw})"
        ))
    })
}

pub(crate) fn uuid_in(
    text: &str,
    ctx: impl std::fmt::Display + Copy,
) -> Result<Uuid, RuntimeError> {
    let id =
        Uuid::parse_str(text).map_err(|error| err(format!("bumbledb marshal: {ctx}: {error}")))?;
    let mut buffer = Uuid::encode_buffer();
    if id.hyphenated().encode_lower(&mut buffer) != text {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: expected canonical UUID"
        )));
    }
    Ok(id)
}

pub(crate) fn uuid_text(id: Uuid) -> String {
    id.to_string()
}

fn interval_in(
    obj: &Object,
    element: IntervalElement,
    ctx: impl std::fmt::Display + Copy,
) -> Result<Value, RuntimeError> {
    match element {
        IntervalElement::U64 => interval_u64_in(obj, ctx).map(Value::IntervalU64),
        IntervalElement::I64 => interval_i64_in(obj, ctx).map(Value::IntervalI64),
        IntervalElement::F64 => interval_f64_in(obj, ctx).map(Value::IntervalF64),
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

fn cell_mismatch(ctx: CellCtx<'_>, want: &str, got: JsType) -> RuntimeError {
    err(format!(
        "bumbledb marshal: {ctx}: expected {want}, got {}",
        js_type_name(got)
    ))
}

fn bytes_width_mismatch(ctx: CellCtx<'_>, len: u16, witnessed: usize) -> RuntimeError {
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
pub(crate) fn schema_value_in(
    expected: &ValueType,
    value: &Unknown,
    relation: &str,
    field: &str,
) -> Result<Value, RuntimeError> {
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
            let text = string_in(&unsafe { value.cast::<Utf16String>()? }, ctx)?;
            Ok(Value::String(text.into()))
        }
        ValueType::Uuid => {
            if got != JsType::String {
                return Err(mismatch("string (canonical UUID text)"));
            }
            let text = string_in(&unsafe { value.cast::<Utf16String>()? }, ctx)?;
            Ok(Value::Uuid(uuid_in(&text, ctx)?))
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
        ValueType::Interval { element } => {
            if got != JsType::Object {
                return Err(mismatch(interval_pair_name(*element)));
            }
            interval_in(&unsafe { value.cast::<Object>()? }, *element, ctx)
        }
        ValueType::FixedInterval { element, .. } => {
            if got != JsType::Object {
                return Err(mismatch(interval_pair_name(element.element())));
            }
            interval_in(&unsafe { value.cast::<Object>()? }, element.element(), ctx)
        }
    }
}

const fn interval_pair_name(element: IntervalElement) -> &'static str {
    match element {
        IntervalElement::U64 | IntervalElement::I64 => "{ start, end } bigint pair",
        IntervalElement::F64 => "{ start, end } number pair",
    }
}

pub(crate) fn key_row(
    schema: &SchemaHandle,
    relation: u32,
    key_statement: u32,
    values: &Array,
) -> Result<(RelationId, StatementId, Vec<Value>), RuntimeError> {
    let rel = RelationId(relation);
    let roster = schema
        .rosters
        .get(relation as usize)
        .ok_or_else(|| err(format!("bumbledb marshal: unknown relation id {relation}")))?;
    let name = &roster.name;
    let statement_id = StatementId(u16_id(key_statement, "key statement id")?);
    let Some(StatementDescriptor::Functionality {
        relation: key_relation,
        projection,
    }) = schema.statements.get(key_statement as usize)
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
    if values.len() as usize != projection.len() {
        return Err(err(format!(
            "bumbledb marshal: key of `{name}`: expected {} key values, got {}",
            projection.len(),
            values.len()
        )));
    }
    let mut row = Vec::with_capacity(projection.len());
    for (index, field_id) in (0..values.len()).zip(projection.iter()) {
        let field = roster.fields.get(usize::from(field_id.0)).ok_or_else(|| {
            err(format!(
                "bumbledb marshal: key of `{name}`: projection field {} out of range",
                field_id.0
            ))
        })?;
        let value = req_at::<Unknown>(values, index, format_args!("key of `{name}`"))?;
        row.push(schema_value_in(
            &field.value_type,
            &value,
            name,
            &field.name,
        )?);
    }
    Ok((rel, statement_id, row))
}

pub(crate) fn tagged_value(obj: &Object) -> Result<Value, RuntimeError> {
    let kind: String = req_text(obj, "kind", "value")?;
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
            req_text(obj, "value", "string value")?.into(),
        )),
        tags::value::UUID => Ok(Value::Uuid(uuid_in(
            &req_text(obj, "value", "uuid value")?,
            "uuid value",
        )?)),
        tags::value::FIXED_BYTES => Ok(Value::FixedBytes(
            req::<Uint8Array>(obj, "value", "fixedBytes value")?
                .to_vec()
                .into_boxed_slice(),
        )),
        tags::value::INTERVAL_U64 => interval_in(obj, IntervalElement::U64, "intervalU64 value"),
        tags::value::INTERVAL_I64 => interval_in(obj, IntervalElement::I64, "intervalI64 value"),
        tags::value::INTERVAL_F64 => interval_in(obj, IntervalElement::F64, "intervalF64 value"),
        other => Err(err(format!(
            "bumbledb marshal: unknown value kind `{other}`"
        ))),
    }
}

pub(crate) enum OwnedParam {
    Scalar(Value),
    Set(Vec<Value>),
}

pub(crate) fn params_in(arr: &Array) -> Result<Vec<OwnedParam>, RuntimeError> {
    let mut params = Vec::with_capacity(arr.len() as usize);
    for index in 0..arr.len() {
        let obj = req_at::<Object>(arr, index, "params")?;
        let kind: String = req_text(&obj, "kind", "param")?;
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

/// One row cell as JavaScript sees it: `u64`/`i64` as `bigint`, `f64` as
/// `number`, text and UUIDs as `string`, bytes as `Uint8Array`, intervals as
/// `{ start, end }` in their element's representation.
#[napi]
pub type CellValue = Either6<bool, BigInt, f64, String, Uint8Array, CellInterval>;

#[napi(object, object_to_js = false, object_from_js = false)]
pub struct CellInterval {
    #[napi(ts_type = "bigint | number")]
    pub start: (),
    #[napi(ts_type = "bigint | number")]
    pub end: (),
}

/// One tagged data-plane value: an execute param's scalar, or a set member.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
pub enum CellIn {
    Bool { value: bool },
    U64 { value: BigInt },
    I64 { value: BigInt },
    F64 { value: f64 },
    String { value: String },
    Uuid { value: String },
    FixedBytes { value: Uint8Array },
    IntervalU64 { start: BigInt, end: BigInt },
    IntervalI64 { start: BigInt, end: BigInt },
    IntervalF64 { start: f64, end: f64 },
}

/// A set param: membership in any of `values`.
#[napi(object, object_to_js = false, object_from_js = false)]
pub struct ParamSetIn {
    #[napi(ts_type = "'Set'")]
    pub kind: (),
    pub values: Vec<CellIn>,
}

/// One execute param, positional by `ParamId`.
#[napi]
pub type ParamIn = Either<CellIn, ParamSetIn>;

#[derive(Debug)]
pub enum ValueOut {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(F64),
    Text(String),
    /// Canonical hyphenated UUID text — the TypeScript spelling of an
    /// application-owned `Uuid`.
    Uuid(String),
    Bytes(Vec<u8>),
    IntervalU64 {
        start: u64,
        end: u64,
    },
    IntervalI64 {
        start: i64,
        end: i64,
    },
    IntervalF64 {
        start: F64,
        end: F64,
    },
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
            Value::Uuid(v) => Self::Uuid(uuid_text(v)),
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
            Value::IntervalF64(interval) => Self::IntervalF64 {
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
            Self::Text(v) | Self::Uuid(v) => unsafe { String::to_napi_value(env, v) },
            Self::Bytes(v) => unsafe { Uint8Array::to_napi_value(env, Uint8Array::new(v)) },
            Self::IntervalF64 { start, end } => {
                let env_handle = Env::from_raw(env);
                let mut obj = Object::new(&env_handle)?;
                obj.set("start", start.to_f64())?;
                obj.set("end", end.to_f64())?;
                unsafe { Object::to_napi_value(env, obj) }
            }
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

fn allocation_error(_: std::collections::TryReserveError) -> crate::runtime::RuntimeError {
    crate::runtime::RuntimeError::OutOfMemory
}

/// Reserve the final destination once; no result-sized staging owner.
pub(crate) fn output_vec<T>(len: usize) -> Result<Vec<T>, crate::runtime::RuntimeError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(allocation_error)?;
    Ok(values)
}

fn value_out_from_answer(value: AnswerValue<'_>) -> ValueOut {
    match value {
        AnswerValue::Bool(v) => ValueOut::Bool(v),
        AnswerValue::U64(v) => ValueOut::U64(v),
        AnswerValue::I64(v) => ValueOut::I64(v),
        AnswerValue::F64(v) => ValueOut::F64(v),
        AnswerValue::String(v) => ValueOut::Text(v.to_owned()),
        AnswerValue::Uuid(v) => ValueOut::Uuid(uuid_text(v)),
        AnswerValue::FixedBytes(v) => ValueOut::Bytes(v.to_owned()),
        AnswerValue::IntervalU64(v) => ValueOut::IntervalU64 {
            start: v.start(),
            end: v.end(),
        },
        AnswerValue::IntervalI64(v) => ValueOut::IntervalI64 {
            start: v.start(),
            end: v.end(),
        },
        AnswerValue::IntervalF64(v) => ValueOut::IntervalF64 {
            start: v.start(),
            end: v.end(),
        },
    }
}

fn borrowed_value(value: &Value) -> AnswerValue<'_> {
    match value {
        Value::Bool(v) => AnswerValue::Bool(*v),
        Value::U64(v) => AnswerValue::U64(*v),
        Value::I64(v) => AnswerValue::I64(*v),
        Value::F64(v) => AnswerValue::F64(*v),
        Value::Uuid(v) => AnswerValue::Uuid(*v),
        Value::String(v) => AnswerValue::String(v),
        Value::FixedBytes(v) => AnswerValue::FixedBytes(v),
        Value::IntervalU64(v) => AnswerValue::IntervalU64(*v),
        Value::IntervalI64(v) => AnswerValue::IntervalI64(*v),
        Value::IntervalF64(v) => AnswerValue::IntervalF64(*v),
    }
}

pub(crate) fn queued_row(
    work: &bumbledb::work::WorkContext,
    row: &bumbledb::canonical::DecodedRow,
) -> Result<crate::runtime::QueuedRow, crate::runtime::RuntimeError> {
    Ok(crate::runtime::QueuedRow {
        values: row_out(work, row)?,
    })
}

/// Convert each borrowed cell directly into its final native owner.
pub(crate) fn row_out(
    work: &bumbledb::work::WorkContext,
    row: &bumbledb::canonical::DecodedRow,
) -> Result<Vec<ValueOut>, crate::runtime::RuntimeError> {
    work.checkpoint()?;
    let mut values = output_vec(row.len())?;
    for value in row {
        work.checkpoint()?;
        values.push(value_out_from_answer(borrowed_value(value)));
    }
    work.checkpoint()?;
    Ok(values)
}

fn result_work_error(error: bumbledb::work::WorkError) -> bumbledb::Error {
    bumbledb::Error::Store(Box::new(bumbledb::store::StoreError::Work(error)))
}

fn result_allocation_error(_: std::collections::TryReserveError) -> bumbledb::Error {
    result_work_error(bumbledb::work::WorkError::Allocation)
}

/// Collection reserves its known row count. Page delivery grows only its
/// bounded batch; neither path materializes another Answers.
pub(crate) fn result_rows(
    work: &bumbledb::work::WorkContext,
    capacity: usize,
) -> Result<crate::runtime::QueuedOutput, bumbledb::Error> {
    work.checkpoint().map_err(result_work_error)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(result_allocation_error)?;
    Ok(crate::runtime::QueuedOutput { rows })
}

/// One pass through borrowed values into final worker-to-JavaScript output.
/// Text/blob copies are required by the JS ownership boundary, not sizing.
pub(crate) fn push_result_row(
    work: &bumbledb::work::WorkContext,
    output: &mut crate::runtime::QueuedOutput,
    row: &bumbledb::ResultRow<'_>,
) -> Result<(), bumbledb::Error> {
    work.checkpoint().map_err(result_work_error)?;
    output
        .rows
        .try_reserve(1)
        .map_err(result_allocation_error)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(row.arity())
        .map_err(result_allocation_error)?;
    for value in row.values() {
        work.checkpoint().map_err(result_work_error)?;
        values.push(value_out_from_answer(value));
    }
    work.checkpoint().map_err(result_work_error)?;
    output.rows.push(values);
    Ok(())
}

/// One named cell of a rendered fact or closed row.
#[napi(object, object_from_js = false)]
pub struct NamedValueOut {
    pub name: String,
    #[napi(ts_type = "CellValue")]
    pub value: ValueOut,
}

/// One fact a violation cites, by relation and field names.
#[napi(object, object_from_js = false)]
pub struct FactOut {
    pub relation: String,
    pub fields: Vec<NamedValueOut>,
}

#[napi(string_enum)]
pub enum DirectionOut {
    SourceUnsatisfied,
    TargetRequired,
}

/// One violated statement: its id, canonical spelling and the cited facts.
#[napi(discriminant = "_tag", object_from_js = false)]
pub enum ViolationOut {
    Functionality {
        statement: u32,
        spelling: String,
        facts: Vec<FactOut>,
    },
    Containment {
        statement: u32,
        spelling: String,
        direction: DirectionOut,
        facts: Vec<FactOut>,
    },
    /// `measure` is the grouped measure that left the capacity window.
    Capacity {
        statement: u32,
        spelling: String,
        measure: u128,
        facts: Vec<FactOut>,
    },
}

fn facts_out(facts: Vec<bumbledb::RenderedFact>) -> Vec<FactOut> {
    facts
        .into_iter()
        .map(|fact| FactOut {
            relation: fact.relation.into_string(),
            fields: fact
                .fields
                .into_iter()
                .map(|(name, value)| NamedValueOut {
                    name: name.into_string(),
                    value: ValueOut::from_value(value),
                })
                .collect(),
        })
        .collect()
}

impl From<RenderedViolation> for ViolationOut {
    fn from(rendered: RenderedViolation) -> Self {
        match rendered {
            RenderedViolation::Functionality {
                statement,
                spelling,
                facts,
            } => Self::Functionality {
                statement: u32::from(statement.0),
                spelling,
                facts: facts_out(facts),
            },
            RenderedViolation::Containment {
                statement,
                spelling,
                direction,
                facts,
            } => Self::Containment {
                statement: u32::from(statement.0),
                spelling,
                direction: match direction {
                    Direction::SourceUnsatisfied => DirectionOut::SourceUnsatisfied,
                    Direction::TargetRequired => DirectionOut::TargetRequired,
                },
                facts: facts_out(facts),
            },
            RenderedViolation::Capacity {
                statement,
                spelling,
                measure,
                facts,
            } => Self::Capacity {
                statement: u32::from(statement.0),
                spelling,
                measure,
                facts: facts_out(facts),
            },
        }
    }
}
