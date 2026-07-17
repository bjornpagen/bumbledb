//! JS ⇄ engine-data marshaling — the whole vocabulary the bridge speaks.
//!
//! Direction and shape law (docs/graph-builder-rebirth/prd-04-ffi-surface.md):
//!
//! - **Fact rows are natural JS values, schema-directed**: `boolean ⇄ bool`,
//!   `bigint ⇄ u64/i64` (range-checked, the error names relation and field),
//!   `string ⇄ str`, `Uint8Array ⇄ bytes<N>` (width-checked), a
//!   `{start, end}` bigint pair ⇄ interval. The expected type always comes
//!   from the schema descriptor — marshaling never guesses.
//! - **IR, spec, and query params are tagged plain objects mirroring the
//!   engine's own data types 1:1** (`bumbledb::ir`, `bumbledb::SchemaSpec`):
//!   there is no anchoring schema position to direct an arbitrary literal, so
//!   the tag carries the type, exactly as the Rust enum does.
//! - **Values crossing outward are natural JS values** (`ValueOut`): answer
//!   rows, scans, point reads, decoded violation facts, manifest extension
//!   values. u64/i64 always cross as `bigint` — never `number`.
//!
//! Nothing here validates semantics: unresolvable names, banned spellings,
//! shape mismatches beyond marshaling, and every dependency judgment belong
//! to the engine's own typed boundaries.

use bumbledb::schema::spec::{
    FieldSpec, LiteralSetSpec, LiteralSpec, RelationSpec, RowSpec, SideSpec, StatementSpec,
    WindowSpec,
};
use bumbledb::schema::{IntervalElement, StatementDescriptor, ValueType};
use bumbledb::{
    AggOp, AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm,
    HeadOp, HeadTerm, Interval, Manifest, MaskTerm, ParamId, PredId, PredicateDef, Program,
    RelationId, RenderedViolation, Rule, SchemaDescriptor, SchemaSpec, StatementId, StatementKind,
    Term, Value, VarId,
};
use napi::bindgen_prelude::{
    Array, BigInt, Env, FromNapiValue, Object, ToNapiValue, Uint8Array, i64n,
};
use napi::{Unknown, ValueType as JsType, sys};

/// A thrown bridge error: marshaling and shape violations throw across the
/// boundary (domain outcomes never do — they return as data).
pub(crate) fn err(message: String) -> napi::Error {
    napi::Error::from_reason(message)
}

/// The engine error rendered for the wire — one string, `Display`'s own
/// spelling; the TS layer wraps it into `@superbuilders/errors`.
pub(crate) fn engine_err(error: &bumbledb::Error) -> String {
    format!("bumbledb: {error}")
}

/// A JS value-type name for shape-error messages.
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
        _ => "unknown",
    }
}

/// A required object property, with the missing-key error naming its context.
fn req<T: FromNapiValue>(obj: &Object, key: &str, ctx: &str) -> napi::Result<T> {
    obj.get::<T>(key)?
        .ok_or_else(|| err(format!("bumbledb marshal: missing `{key}` in {ctx}")))
}

/// A required array element.
fn req_at<T: FromNapiValue>(arr: &Array, index: u32, ctx: &str) -> napi::Result<T> {
    arr.get::<T>(index)?.ok_or_else(|| {
        err(format!(
            "bumbledb marshal: missing element {index} in {ctx}"
        ))
    })
}

/// A `bigint` as `u64`, lossless or a typed error naming its position.
pub(crate) fn u64_in(value: &BigInt, ctx: &str) -> napi::Result<u64> {
    let (sign, word, lossless) = value.get_u64();
    if sign || !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of u64 range"
        )));
    }
    Ok(word)
}

/// A `bigint` as `i64`, lossless or a typed error naming its position.
pub(crate) fn i64_in(value: &BigInt, ctx: &str) -> napi::Result<i64> {
    let (word, lossless) = value.get_i64();
    if !lossless {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: bigint out of i64 range"
        )));
    }
    Ok(word)
}

/// A JS number as a dense id ordinal, refusing fractions and overflow.
fn ordinal(value: f64, ctx: &str) -> napi::Result<u32> {
    if !(value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX))
    {
        return Err(err(format!(
            "bumbledb marshal: {ctx}: expected a non-negative integer id, got {value}"
        )));
    }
    Ok(value as u32)
}

fn u16_id(value: u32, ctx: &str) -> napi::Result<u16> {
    u16::try_from(value)
        .map_err(|_| err(format!("bumbledb marshal: {ctx}: id {value} exceeds u16")))
}

/// A half-open interval from a `{start, end}` bigint pair, per element
/// domain; an empty interval (`start >= end`) is a shape error — the engine
/// value is nonempty by construction (`bumbledb::Interval`).
fn interval_in(obj: &Object, element: IntervalElement, ctx: &str) -> napi::Result<Value> {
    match element {
        IntervalElement::U64 => {
            let start = u64_in(&req::<BigInt>(obj, "start", ctx)?, ctx)?;
            let end = u64_in(&req::<BigInt>(obj, "end", ctx)?, ctx)?;
            Interval::<u64>::new(start, end)
                .map(Value::IntervalU64)
                .ok_or_else(|| {
                    err(format!(
                        "bumbledb marshal: {ctx}: empty interval (start {start} >= end {end})"
                    ))
                })
        }
        IntervalElement::I64 => {
            let start = i64_in(&req::<BigInt>(obj, "start", ctx)?, ctx)?;
            let end = i64_in(&req::<BigInt>(obj, "end", ctx)?, ctx)?;
            Interval::<i64>::new(start, end)
                .map(Value::IntervalI64)
                .ok_or_else(|| {
                    err(format!(
                        "bumbledb marshal: {ctx}: empty interval (start {start} >= end {end})"
                    ))
                })
        }
    }
}

/// One natural JS value marshaled against the schema-declared type of its
/// field — the fact-row lane's one conversion.
fn schema_value(
    expected: &ValueType,
    value: &Unknown,
    relation: &str,
    field: &str,
) -> napi::Result<Value> {
    let ctx = format!("relation `{relation}` field `{field}`");
    let got = value.get_type()?;
    let mismatch = |want: &str| {
        err(format!(
            "bumbledb marshal: {ctx}: expected {want}, got {}",
            js_type_name(got)
        ))
    };
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
                &ctx,
            )?))
        }
        ValueType::I64 => {
            if got != JsType::BigInt {
                return Err(mismatch("bigint (i64)"));
            }
            Ok(Value::I64(i64_in(
                &unsafe { value.cast::<BigInt>()? },
                &ctx,
            )?))
        }
        ValueType::String => {
            if got != JsType::String {
                return Err(mismatch("string"));
            }
            let text = unsafe { value.cast::<String>()? };
            Ok(Value::String(text.into_bytes().into_boxed_slice()))
        }
        ValueType::FixedBytes { len } => {
            if got != JsType::Object {
                return Err(mismatch("Uint8Array"));
            }
            let bytes = unsafe { value.cast::<Uint8Array>()? };
            if bytes.len() != usize::from(*len) {
                return Err(err(format!(
                    "bumbledb marshal: {ctx}: expected bytes<{len}>, got {} bytes",
                    bytes.len()
                )));
            }
            Ok(Value::FixedBytes(bytes.to_vec().into_boxed_slice()))
        }
        ValueType::Interval { element, .. } => {
            if got != JsType::Object {
                return Err(mismatch("{ start, end } bigint pair"));
            }
            interval_in(&unsafe { value.cast::<Object>()? }, *element, &ctx)
        }
    }
}

/// A relation's SEALED field roster: a closed relation's synthetic
/// (`id`, u64) handle field first, declared fields after — the numbering
/// every dynamic-surface row and statement addresses.
fn sealed_fields(
    descriptor: &SchemaDescriptor,
    relation: RelationId,
) -> napi::Result<Vec<(Box<str>, ValueType)>> {
    let rel = descriptor
        .relations
        .get(relation.0 as usize)
        .ok_or_else(|| {
            err(format!(
                "bumbledb marshal: unknown relation id {}",
                relation.0
            ))
        })?;
    let mut fields = Vec::with_capacity(rel.fields.len() + 1);
    if rel.extension.is_some() {
        fields.push((Box::from("id"), ValueType::U64));
    }
    for field in &rel.fields {
        fields.push((field.name.clone(), field.value_type.clone()));
    }
    Ok(fields)
}

fn relation_name(descriptor: &SchemaDescriptor, relation: RelationId) -> String {
    descriptor.relations.get(relation.0 as usize).map_or_else(
        || format!("relation#{}", relation.0),
        |rel| rel.name.to_string(),
    )
}

/// One dynamic fact row: natural JS values in sealed field order,
/// schema-directed, arity-checked.
pub(crate) fn fact_row(
    descriptor: &SchemaDescriptor,
    relation: u32,
    values: &Array,
) -> napi::Result<(RelationId, Vec<Value>)> {
    let rel = RelationId(relation);
    let fields = sealed_fields(descriptor, rel)?;
    let name = relation_name(descriptor, rel);
    if values.len() as usize != fields.len() {
        return Err(err(format!(
            "bumbledb marshal: relation `{name}`: expected {} values, got {}",
            fields.len(),
            values.len()
        )));
    }
    let mut row = Vec::with_capacity(fields.len());
    for (index, (field, expected)) in fields.iter().enumerate() {
        let value = req_at::<Unknown>(values, index as u32, &format!("relation `{name}` row"))?;
        row.push(schema_value(expected, &value, &name, field)?);
    }
    Ok((rel, row))
}

/// One point-read key row: natural JS values in the key statement's
/// projection order, schema-directed through the projected fields' types.
pub(crate) fn key_row(
    descriptor: &SchemaDescriptor,
    statements: &[StatementDescriptor],
    relation: u32,
    key_statement: u32,
    values: &Array,
) -> napi::Result<(RelationId, StatementId, Vec<Value>)> {
    let rel = RelationId(relation);
    let name = relation_name(descriptor, rel);
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
    let fields = sealed_fields(descriptor, rel)?;
    if values.len() as usize != projection.len() {
        return Err(err(format!(
            "bumbledb marshal: key of `{name}`: expected {} key values, got {}",
            projection.len(),
            values.len()
        )));
    }
    let mut row = Vec::with_capacity(projection.len());
    for (index, field_id) in projection.iter().enumerate() {
        let (field, expected) = fields.get(usize::from(field_id.0)).ok_or_else(|| {
            err(format!(
                "bumbledb marshal: key of `{name}`: projection field {} out of range",
                field_id.0
            ))
        })?;
        let value = req_at::<Unknown>(values, index as u32, &format!("key of `{name}`"))?;
        row.push(schema_value(expected, &value, &name, field)?);
    }
    Ok((rel, statement_id, row))
}

/// One TAGGED value — the 1:1 mirror of `bumbledb::Value` (the spec/IR/param
/// lane, where no schema position directs the type).
pub(crate) fn tagged_value(obj: &Object) -> napi::Result<Value> {
    let kind: String = req(obj, "kind", "value")?;
    match kind.as_str() {
        "bool" => Ok(Value::Bool(req::<bool>(obj, "value", "bool value")?)),
        "u64" => Ok(Value::U64(u64_in(
            &req::<BigInt>(obj, "value", "u64 value")?,
            "u64 value",
        )?)),
        "i64" => Ok(Value::I64(i64_in(
            &req::<BigInt>(obj, "value", "i64 value")?,
            "i64 value",
        )?)),
        "string" => Ok(Value::String(
            req::<String>(obj, "value", "string value")?
                .into_bytes()
                .into_boxed_slice(),
        )),
        "fixedBytes" => Ok(Value::FixedBytes(
            req::<Uint8Array>(obj, "value", "fixedBytes value")?
                .to_vec()
                .into_boxed_slice(),
        )),
        "intervalU64" => interval_in(obj, IntervalElement::U64, "intervalU64 value"),
        "intervalI64" => interval_in(obj, IntervalElement::I64, "intervalI64 value"),
        "allenMask" => {
            let bits = ordinal(req::<f64>(obj, "mask", "allenMask value")?, "allen mask")?;
            let bits = u16::try_from(bits)
                .ok()
                .and_then(AllenMask::new)
                .ok_or_else(|| err(format!("bumbledb marshal: invalid allen mask bits {bits}")))?;
            Ok(Value::AllenMask(bits))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown value kind `{other}`"
        ))),
    }
}

/// One positional execution argument, owned: the tagged mirror of
/// `bumbledb::ParamArg` (`{ kind: "set", values }` is the set arm; every
/// scalar kind is a `Value` tag).
pub(crate) enum OwnedParam {
    Scalar(Value),
    Set(Vec<Value>),
}

/// The execute-call params array.
pub(crate) fn params_in(arr: &Array) -> napi::Result<Vec<OwnedParam>> {
    let mut params = Vec::with_capacity(arr.len() as usize);
    for index in 0..arr.len() {
        let obj = req_at::<Object>(arr, index, "params")?;
        let kind: String = req(&obj, "kind", "param")?;
        if kind == "set" {
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

/// The structural value-type mirror (`ValueTypeSpec` in `#spec.ts`).
fn value_type_in(obj: &Object) -> napi::Result<ValueType> {
    let kind: String = req(obj, "kind", "value type")?;
    match kind.as_str() {
        "bool" => Ok(ValueType::Bool),
        "u64" => Ok(ValueType::U64),
        "i64" => Ok(ValueType::I64),
        "string" => Ok(ValueType::String),
        "fixedBytes" => {
            let len = ordinal(req::<f64>(obj, "len", "fixedBytes type")?, "bytes width")?;
            let len = u16::try_from(len)
                .map_err(|_| err(format!("bumbledb marshal: bytes width {len} exceeds u16")))?;
            Ok(ValueType::FixedBytes { len })
        }
        "interval" => {
            let element: String = req(obj, "element", "interval type")?;
            let element = match element.as_str() {
                "u64" => IntervalElement::U64,
                "i64" => IntervalElement::I64,
                other => {
                    return Err(err(format!(
                        "bumbledb marshal: unknown interval element `{other}`"
                    )));
                }
            };
            let width = obj
                .get::<BigInt>("width")?
                .map(|w| u64_in(&w, "interval width"))
                .transpose()?;
            Ok(ValueType::Interval { element, width })
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown value type kind `{other}`"
        ))),
    }
}

fn literal_in(obj: &Object) -> napi::Result<LiteralSpec> {
    let kind: String = req(obj, "kind", "literal")?;
    match kind.as_str() {
        "handle" => Ok(LiteralSpec::Handle(
            req::<String>(obj, "handle", "handle literal")?.into(),
        )),
        "value" => {
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
        "one" => {
            let literal: Object = req(obj, "literal", "one-literal binding")?;
            Ok(LiteralSetSpec::One(literal_in(&literal)?))
        }
        "many" => {
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

fn window_in(obj: &Object) -> napi::Result<WindowSpec> {
    let kind: String = req(obj, "kind", "window")?;
    match kind.as_str() {
        "exact" => Ok(WindowSpec::Exact(u64_in(
            &req::<BigInt>(obj, "n", "exact window")?,
            "window count",
        )?)),
        "range" => Ok(WindowSpec::Range {
            lo: u64_in(&req::<BigInt>(obj, "lo", "range window")?, "window lo")?,
            hi: u64_in(&req::<BigInt>(obj, "hi", "range window")?, "window hi")?,
        }),
        "floor" => Ok(WindowSpec::Floor(u64_in(
            &req::<BigInt>(obj, "lo", "floor window")?,
            "window lo",
        )?)),
        other => Err(err(format!(
            "bumbledb marshal: unknown window kind `{other}`"
        ))),
    }
}

fn statement_in(obj: &Object) -> napi::Result<StatementSpec> {
    let kind: String = req(obj, "kind", "statement")?;
    match kind.as_str() {
        "fd" => {
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
        "containment" => Ok(StatementSpec::Containment {
            source: side_in(&req::<Object>(obj, "source", "containment")?)?,
            target: side_in(&req::<Object>(obj, "target", "containment")?)?,
            bidirectional: req::<bool>(obj, "bidirectional", "containment")?,
        }),
        "cardinality" => Ok(StatementSpec::Cardinality {
            target: side_in(&req::<Object>(obj, "target", "cardinality")?)?,
            window: window_in(&req::<Object>(obj, "window", "cardinality")?)?,
            source: side_in(&req::<Object>(obj, "source", "cardinality")?)?,
        }),
        other => Err(err(format!(
            "bumbledb marshal: unknown statement kind `{other}`"
        ))),
    }
}

/// The whole `SchemaSpec`, mirroring `#spec.ts` key for key.
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
        let extension = match relation.get::<Array>("extension")? {
            None => None,
            Some(rows) => {
                let mut row_specs = Vec::with_capacity(rows.len() as usize);
                for row_index in 0..rows.len() {
                    let row = req_at::<Object>(&rows, row_index, "relation extension")?;
                    let values: Array = req(&row, "values", "extension row")?;
                    let mut literals = Vec::with_capacity(values.len() as usize);
                    for value_index in 0..values.len() {
                        let literal = req_at::<Object>(&values, value_index, "extension row")?;
                        literals.push(literal_in(&literal)?);
                    }
                    row_specs.push(RowSpec {
                        handle: req::<String>(&row, "handle", "extension row")?.into(),
                        values: literals,
                    });
                }
                Some(row_specs)
            }
        };
        relation_specs.push(RelationSpec {
            name: req::<String>(&relation, "name", "relation spec")?.into(),
            newtype: relation.get::<String>("newtype")?.map(Into::into),
            fields: field_specs,
            extension,
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
        "var" => Ok(Term::Var(var_in(obj, "var", "var term")?)),
        "param" => Ok(Term::Param(param_in(obj, "param", "param term")?)),
        "paramSet" => Ok(Term::ParamSet(param_in(obj, "param", "paramSet term")?)),
        "measure" => Ok(Term::Measure(var_in(obj, "var", "measure term")?)),
        "literal" => {
            let value: Object = req(obj, "value", "literal term")?;
            Ok(Term::Literal(tagged_value(&value)?))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown term kind `{other}`"
        ))),
    }
}

fn agg_op_in(obj: &Object) -> napi::Result<AggOp> {
    let kind: String = req(obj, "kind", "aggregate op")?;
    match kind.as_str() {
        "sum" => Ok(AggOp::Sum),
        "min" => Ok(AggOp::Min),
        "max" => Ok(AggOp::Max),
        "count" => Ok(AggOp::Count),
        "countDistinct" => Ok(AggOp::CountDistinct),
        "argMax" => Ok(AggOp::ArgMax {
            key: var_in(obj, "key", "argMax op")?,
        }),
        "argMin" => Ok(AggOp::ArgMin {
            key: var_in(obj, "key", "argMin op")?,
        }),
        "pack" => Ok(AggOp::Pack),
        other => Err(err(format!(
            "bumbledb marshal: unknown aggregate op `{other}`"
        ))),
    }
}

fn head_term_in(obj: &Object) -> napi::Result<HeadTerm> {
    let kind: String = req(obj, "kind", "head term")?;
    match kind.as_str() {
        "var" => Ok(HeadTerm::Var),
        "aggregate" => {
            let op: String = req(obj, "op", "head aggregate")?;
            let op = match op.as_str() {
                "sum" => HeadOp::Sum,
                "min" => HeadOp::Min,
                "max" => HeadOp::Max,
                "count" => HeadOp::Count,
                "countDistinct" => HeadOp::CountDistinct,
                "argMax" => HeadOp::ArgMax,
                "argMin" => HeadOp::ArgMin,
                "pack" => HeadOp::Pack,
                other => {
                    return Err(err(format!("bumbledb marshal: unknown head op `{other}`")));
                }
            };
            Ok(HeadTerm::Aggregate(op))
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown head term kind `{other}`"
        ))),
    }
}

fn find_term_in(obj: &Object) -> napi::Result<FindTerm> {
    let kind: String = req(obj, "kind", "find term")?;
    match kind.as_str() {
        "var" => Ok(FindTerm::Var(var_in(obj, "var", "var find")?)),
        "measure" => Ok(FindTerm::Measure(var_in(obj, "var", "measure find")?)),
        "aggregate" => {
            let op: Object = req(obj, "op", "aggregate find")?;
            let over = match obj.get::<f64>("over")? {
                None => None,
                Some(over) => Some(VarId(u16_id(
                    ordinal(over, "aggregate over")?,
                    "aggregate over",
                )?)),
            };
            Ok(FindTerm::Aggregate {
                op: agg_op_in(&op)?,
                over,
            })
        }
        "aggregateMeasure" => {
            let op: Object = req(obj, "op", "aggregateMeasure find")?;
            Ok(FindTerm::AggregateMeasure {
                op: agg_op_in(&op)?,
                over: var_in(obj, "over", "aggregateMeasure find")?,
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
        "edb" => AtomSource::Edb(RelationId(ordinal(
            req::<f64>(&source, "relation", "edb source")?,
            "edb relation",
        )?)),
        "idb" => AtomSource::Idb(PredId(u16_id(
            ordinal(req::<f64>(&source, "pred", "idb source")?, "idb pred")?,
            "idb pred",
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
        "eq" => CmpOp::Eq,
        "ne" => CmpOp::Ne,
        "lt" => CmpOp::Lt,
        "le" => CmpOp::Le,
        "gt" => CmpOp::Gt,
        "ge" => CmpOp::Ge,
        "pointIn" => CmpOp::PointIn,
        "allen" => {
            let mask: Object = req(&op, "mask", "allen op")?;
            let mask_kind: String = req(&mask, "kind", "allen mask")?;
            let mask = match mask_kind.as_str() {
                "literal" => {
                    let bits = ordinal(
                        req::<f64>(&mask, "mask", "allen mask literal")?,
                        "allen mask",
                    )?;
                    let mask = u16::try_from(bits)
                        .ok()
                        .and_then(AllenMask::new)
                        .ok_or_else(|| {
                            err(format!("bumbledb marshal: invalid allen mask bits {bits}"))
                        })?;
                    MaskTerm::Literal(mask)
                }
                "param" => MaskTerm::Param(param_in(&mask, "param", "allen mask param")?),
                other => {
                    return Err(err(format!(
                        "bumbledb marshal: unknown allen mask kind `{other}`"
                    )));
                }
            };
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

/// One condition tree, marshaled with an explicit depth ceiling of
/// `bumbledb::MAX_CONDITION_DEPTH` — the engine's own validated bound
/// (`bumbledb::ir`): the roster rejects deeper trees anyway, and refusing at
/// marshal keeps this recursion stack-safe on hostile input for the same
/// reason the engine measures depth iteratively before its recursive walks.
fn condition_in(obj: &Object, depth: usize) -> napi::Result<ConditionTree> {
    if depth > bumbledb::MAX_CONDITION_DEPTH {
        return Err(err(format!(
            "bumbledb marshal: condition tree deeper than {} (the engine's MAX_CONDITION_DEPTH)",
            bumbledb::MAX_CONDITION_DEPTH
        )));
    }
    let kind: String = req(obj, "kind", "condition")?;
    match kind.as_str() {
        "leaf" => {
            let cmp: Object = req(obj, "cmp", "leaf condition")?;
            Ok(ConditionTree::Leaf(comparison_in(&cmp)?))
        }
        "and" | "or" => {
            let children: Array = req(obj, "children", "condition")?;
            let mut trees = Vec::with_capacity(children.len() as usize);
            for index in 0..children.len() {
                let child = req_at::<Object>(&children, index, "condition children")?;
                trees.push(condition_in(&child, depth + 1)?);
            }
            if kind == "and" {
                Ok(ConditionTree::And(trees))
            } else {
                Ok(ConditionTree::Or(trees))
            }
        }
        other => Err(err(format!(
            "bumbledb marshal: unknown condition kind `{other}`"
        ))),
    }
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

/// The whole program IR, mirroring `bumbledb::ir::Program` 1:1: relations,
/// fields, and predicates by numeric id — the TS layer resolves names
/// through the manifest and sends ids; the bridge never sees names here.
pub(crate) fn program_in(obj: &Object) -> napi::Result<Program> {
    let predicates: Array = req(obj, "predicates", "program")?;
    let mut predicate_defs = Vec::with_capacity(predicates.len() as usize);
    for index in 0..predicates.len() {
        let predicate = req_at::<Object>(&predicates, index, "program predicates")?;
        let head: Array = req(&predicate, "head", "predicate")?;
        let mut head_terms = Vec::with_capacity(head.len() as usize);
        for head_index in 0..head.len() {
            let term = req_at::<Object>(&head, head_index, "predicate head")?;
            head_terms.push(head_term_in(&term)?);
        }
        let rules: Array = req(&predicate, "rules", "predicate")?;
        let mut rule_list = Vec::with_capacity(rules.len() as usize);
        for rule_index in 0..rules.len() {
            let rule = req_at::<Object>(&rules, rule_index, "predicate rules")?;
            rule_list.push(rule_in(&rule)?);
        }
        predicate_defs.push(PredicateDef {
            head: head_terms,
            rules: rule_list,
        });
    }
    let output = PredId(u16_id(
        ordinal(req::<f64>(obj, "output", "program")?, "program output")?,
        "program output",
    )?);
    Ok(Program {
        predicates: predicate_defs,
        output,
    })
}

/// One engine value crossing OUT as a natural JS value: `bool → boolean`,
/// `u64/i64 → bigint`, `str → string`, `bytes<N> → Uint8Array`,
/// `interval → { start, end }` bigint pair. The Allen-mask arm is total but
/// unreachable from any row surface (masks are bind-time-only values); it
/// crosses as its bits so the conversion stays a bijection on everything the
/// engine can actually hand back.
pub enum ValueOut {
    Bool(bool),
    U64(u64),
    I64(i64),
    Text(String),
    Bytes(Vec<u8>),
    IntervalU64 { start: u64, end: u64 },
    IntervalI64 { start: i64, end: i64 },
}

impl ValueOut {
    pub(crate) fn from_value(value: &Value) -> Self {
        match value {
            Value::Bool(v) => Self::Bool(*v),
            Value::U64(v) => Self::U64(*v),
            Value::I64(v) => Self::I64(*v),
            Value::String(bytes) => Self::Text(String::from_utf8_lossy(bytes).into_owned()),
            Value::FixedBytes(bytes) => Self::Bytes(bytes.to_vec()),
            Value::IntervalU64(interval) => Self::IntervalU64 {
                start: interval.start(),
                end: interval.end(),
            },
            Value::IntervalI64(interval) => Self::IntervalI64 {
                start: interval.start(),
                end: interval.end(),
            },
            Value::AllenMask(mask) => Self::U64(u64::from(mask.bits())),
        }
    }
}

impl ToNapiValue for ValueOut {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        match val {
            Self::Bool(v) => unsafe { bool::to_napi_value(env, v) },
            Self::U64(v) => unsafe { u64::to_napi_value(env, v) },
            Self::I64(v) => unsafe { i64n::to_napi_value(env, i64n(v)) },
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

/// Owned rows to their outward form.
pub(crate) fn rows_out(rows: Vec<Vec<Value>>) -> Vec<Vec<ValueOut>> {
    rows.into_iter()
        .map(|row| row.iter().map(ValueOut::from_value).collect())
        .collect()
}

fn statement_kind_out(kind: StatementKind) -> &'static str {
    match kind {
        StatementKind::Functionality => "functionality",
        StatementKind::Containment => "containment",
        StatementKind::Cardinality => "cardinality",
    }
}

fn value_type_out(env: sys::napi_env, ty: &ValueType) -> napi::Result<sys::napi_value> {
    let env_handle = Env::from_raw(env);
    let mut obj = Object::new(&env_handle)?;
    match ty {
        ValueType::Bool => obj.set("kind", "bool")?,
        ValueType::U64 => obj.set("kind", "u64")?,
        ValueType::I64 => obj.set("kind", "i64")?,
        ValueType::String => obj.set("kind", "string")?,
        ValueType::FixedBytes { len } => {
            obj.set("kind", "fixedBytes")?;
            obj.set("len", u32::from(*len))?;
        }
        ValueType::Interval { element, width } => {
            obj.set("kind", "interval")?;
            obj.set(
                "element",
                match element {
                    IntervalElement::U64 => "u64",
                    IntervalElement::I64 => "i64",
                },
            )?;
            if let Some(width) = width {
                obj.set("width", *width)?;
            }
        }
    }
    unsafe { Object::to_napi_value(env, obj) }
}

/// The manifest as one plain JS object — PRD-02's name→id tables verbatim.
pub struct ManifestWire(pub(crate) Manifest);

impl ToNapiValue for ManifestWire {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let manifest = val.0;
        let mut root = Object::new(&env_handle)?;
        let mut relations = Vec::with_capacity(manifest.relations.len());
        for relation in manifest.relations {
            let mut rel_obj = Object::new(&env_handle)?;
            rel_obj.set("name", relation.name.as_ref())?;
            rel_obj.set("id", relation.id.0)?;
            let mut fields = Vec::with_capacity(relation.fields.len());
            for field in relation.fields {
                let mut field_obj = Object::new(&env_handle)?;
                field_obj.set("name", field.name.as_ref())?;
                field_obj.set("id", u32::from(field.id.0))?;
                let ty = value_type_out(env, &field.value_type)?;
                let ty = unsafe { Unknown::from_raw_unchecked(env, ty) };
                field_obj.set("valueType", ty)?;
                fields.push(field_obj);
            }
            rel_obj.set("fields", fields)?;
            if let Some(extension) = relation.extension {
                let mut rows = Vec::with_capacity(extension.len());
                for row in extension {
                    let mut row_obj = Object::new(&env_handle)?;
                    row_obj.set("handle", row.handle.as_ref())?;
                    row_obj.set("id", row.id)?;
                    let mut values = Vec::with_capacity(row.values.len());
                    for (name, value) in row.values {
                        let mut value_obj = Object::new(&env_handle)?;
                        value_obj.set("name", name.as_ref())?;
                        value_obj.set("value", ValueOut::from_value(&value))?;
                        values.push(value_obj);
                    }
                    row_obj.set("values", values)?;
                    rows.push(row_obj);
                }
                rel_obj.set("extension", rows)?;
            }
            relations.push(rel_obj);
        }
        root.set("relations", relations)?;
        let mut statements = Vec::with_capacity(manifest.statements.len());
        for statement in manifest.statements {
            let mut statement_obj = Object::new(&env_handle)?;
            statement_obj.set("id", u32::from(statement.id.0))?;
            statement_obj.set("kind", statement_kind_out(statement.kind))?;
            statement_obj.set("spelling", statement.spelling)?;
            statements.push(statement_obj);
        }
        root.set("statements", statements)?;
        unsafe { Object::to_napi_value(env, root) }
    }
}

/// One rendered violation as wire data — PRD-02's rejection rendering,
/// carried whole: statement id, form tag, canonical spelling, the
/// direction/count payloads where the form has them, and the offending facts
/// as named decoded values.
pub struct ViolationWire {
    pub(crate) statement: u16,
    pub(crate) kind: StatementKind,
    pub(crate) canonical: String,
    pub(crate) direction: Option<&'static str>,
    pub(crate) count: Option<u64>,
    pub(crate) facts: Vec<(String, Vec<(String, Value)>)>,
}

impl ViolationWire {
    pub(crate) fn from_rendered(rendered: RenderedViolation) -> Self {
        Self {
            statement: rendered.statement.0,
            kind: rendered.kind,
            canonical: rendered.spelling,
            direction: rendered.direction.map(|direction| match direction {
                bumbledb::Direction::SourceUnsatisfied => "sourceUnsatisfied",
                bumbledb::Direction::TargetRequired => "targetRequired",
            }),
            count: rendered.count,
            facts: rendered
                .facts
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
                .collect(),
        }
    }
}

impl ToNapiValue for ViolationWire {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        obj.set("statementId", u32::from(val.statement))?;
        obj.set("kind", statement_kind_out(val.kind))?;
        obj.set("canonical", val.canonical)?;
        if let Some(direction) = val.direction {
            obj.set("direction", direction)?;
        }
        if let Some(count) = val.count {
            obj.set("count", count)?;
        }
        let mut facts = Vec::with_capacity(val.facts.len());
        for (relation, fields) in val.facts {
            let mut fact_obj = Object::new(&env_handle)?;
            fact_obj.set("relation", relation)?;
            let mut field_objs = Vec::with_capacity(fields.len());
            for (name, value) in fields {
                let mut field_obj = Object::new(&env_handle)?;
                field_obj.set("name", name)?;
                field_obj.set("value", ValueOut::from_value(&value))?;
                field_objs.push(field_obj);
            }
            fact_obj.set("fields", field_objs)?;
            facts.push(fact_obj);
        }
        obj.set("facts", facts)?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}

/// The staleness report as wire data: per participating occurrence, the
/// pinned/live counts and the drift ratio, plus the max.
pub struct StalenessWire {
    pub(crate) per_occurrence: Vec<(u32, u64, u64, f64)>,
    pub(crate) max_ratio: f64,
}

impl ToNapiValue for StalenessWire {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let env_handle = Env::from_raw(env);
        let mut obj = Object::new(&env_handle)?;
        let mut drifts = Vec::with_capacity(val.per_occurrence.len());
        for (relation, pinned, live, ratio) in val.per_occurrence {
            let mut drift = Object::new(&env_handle)?;
            drift.set("relation", relation)?;
            drift.set("pinned", pinned)?;
            drift.set("live", live)?;
            drift.set("ratio", ratio)?;
            drifts.push(drift);
        }
        obj.set("perOccurrence", drifts)?;
        obj.set("maxRatio", val.max_ratio)?;
        unsafe { Object::to_napi_value(env, obj) }
    }
}
