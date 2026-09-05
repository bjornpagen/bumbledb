//! The canonical migration plan (C11): finite declarative data, one codec.
//!
//! A plan is generated data, reviewed in the repository as JSON and hashed
//! as a canonical FRAME — JSON formatting can never change identity, and no
//! module path, closure, helper source or executable text exists anywhere in
//! it. Expressions are the CORE scalar roster spelled by source-field NAME;
//! [`super::compile`] lowers them onto [`bumbledb::ScalarExpr`] and the core
//! `ScalarEvaluator` executes them — there is no second evaluator.
//!
//! The stable human label (`id`) is for people and file names. It is never
//! matched as identity: plans are matched by [`plan_digest`], and a changed
//! plan under a reused label is drift, not a takeover.

use bumbledb::Value;
use bumbledb::scalar::NumericCast;

use crate::history::{FrameError, SchemaId};

use super::frame::{self, Frame, KIND_PLAN, PLAN_DIGEST_DOMAIN, Reader, keyed_digest};
use super::json::{
    Json, parse_u64, parse_value, push_hex, push_indent, push_string, read_tree, render_value,
    unhex_exact,
};

/// The maximum expression nesting a plan may spell; matches the core
/// evaluator's own depth fence so a plan that parses always typechecks
/// within the same bound.
pub const MAX_EXPR_DEPTH: usize = 128;

/// Why plan text or plan bytes refused. `Text` arms are grammar refusals
/// with the first offense named; `Frame` arms are canonical-byte refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    Json(&'static str),
    Shape(&'static str),
    Frame(FrameError),
}

impl From<FrameError> for PlanError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(why) => write!(f, "plan json: {why}"),
            Self::Shape(why) => write!(f, "plan: {why}"),
            Self::Frame(error) => write!(f, "plan frame: {error:?}"),
        }
    }
}

impl std::error::Error for PlanError {}

/// A stable human step label: 1..=64 bytes of `[a-z0-9-]`. A label names a
/// file and a review conversation; a digest names the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepLabel(Box<str>);

impl StepLabel {
    /// # Errors
    /// Refuses the empty label, oversize labels and foreign characters.
    pub fn new(text: &str) -> Result<Self, PlanError> {
        if text.is_empty() || text.len() > 64 {
            return Err(PlanError::Shape("label length"));
        }
        if !text
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-'))
        {
            return Err(PlanError::Shape("label characters"));
        }
        Ok(Self(text.into()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One typed expression over the SOURCE row, spelled by field name. This is
/// canonical plan data; the core `ScalarExpr` roster is its one meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanExpr {
    Field(Box<str>),
    Literal(Value),
    Negate(Box<PlanExpr>),
    Add(Box<PlanExpr>, Box<PlanExpr>),
    Subtract(Box<PlanExpr>, Box<PlanExpr>),
    Multiply(Box<PlanExpr>, Box<PlanExpr>),
    Divide(Box<PlanExpr>, Box<PlanExpr>),
    Cast {
        kind: NumericCast,
        expr: Box<PlanExpr>,
    },
    IsNaN(Box<PlanExpr>),
    IsFinite(Box<PlanExpr>),
}

/// One produced target field: the target field's name and its expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMap {
    pub target: Box<str>,
    pub expression: PlanExpr,
}

/// The finite operation roster. Coverage rules live in [`super::compile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    MapRelation {
        source: Box<str>,
        target: Box<str>,
        fields: Vec<FieldMap>,
    },
    EmptyRelation {
        target: Box<str>,
    },
    DropRelation {
        source: Box<str>,
    },
    Seed {
        target: Box<str>,
        rows: Vec<Box<[Value]>>,
    },
    ValidateSchema {
        schema: SchemaId,
    },
}

/// One explicit data-loss acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    pub relation: Box<str>,
    pub field: Option<Box<str>>,
}

/// One canonical migration plan. `sequence` is the global manifest index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub sequence: u64,
    pub label: StepLabel,
    pub from_schema: SchemaId,
    pub to_schema: SchemaId,
    pub operations: Vec<Operation>,
    pub destructive: Vec<Loss>,
}

/// The full authoritative content digest of exactly these canonical plan
/// bytes. This — never the label, never the JSON text — is plan identity.
#[must_use]
pub fn plan_digest(canonical_bytes: &[u8]) -> [u8; 32] {
    keyed_digest(PLAN_DIGEST_DOMAIN, canonical_bytes)
}

// ---------------------------------------------------------------------------
// Canonical frame codec.
// ---------------------------------------------------------------------------

const OP_MAP: u8 = 0;
const OP_EMPTY: u8 = 1;
const OP_DROP: u8 = 2;
const OP_SEED: u8 = 3;
const OP_VALIDATE: u8 = 4;

const EXPR_FIELD: u8 = 0;
const EXPR_LITERAL: u8 = 1;
const EXPR_ADD: u8 = 2;
const EXPR_SUBTRACT: u8 = 3;
const EXPR_MULTIPLY: u8 = 4;
const EXPR_DIVIDE: u8 = 5;
const EXPR_NEGATE: u8 = 6;
const EXPR_CAST: u8 = 7;
const EXPR_IS_NAN: u8 = 8;
const EXPR_IS_FINITE: u8 = 9;

const VALUE_BOOL: u8 = 0;
const VALUE_U64: u8 = 1;
const VALUE_I64: u8 = 2;
const VALUE_F64: u8 = 3;
const VALUE_STRING: u8 = 4;
const VALUE_FIXED_BYTES: u8 = 5;
const VALUE_INTERVAL_U64: u8 = 6;
const VALUE_INTERVAL_I64: u8 = 7;
const VALUE_ID128: u8 = 8;
const VALUE_INTERVAL_F64: u8 = 9;

/// Encode the canonical plan frame — the exact bytes [`plan_digest`] hashes.
/// # Errors
/// Refuses oversized frames and allocation failure.
pub fn canonical_plan_bytes(plan: &Plan, cap: usize) -> Result<Vec<u8>, FrameError> {
    let mut out = frame::begin(KIND_PLAN, cap)?;
    out.u64(plan.sequence)?;
    out.span(plan.label.as_str().as_bytes())?;
    out.bytes(&plan.from_schema.0)?;
    out.bytes(&plan.to_schema.0)?;
    out.u64(plan.operations.len() as u64)?;
    for operation in &plan.operations {
        put_operation(&mut out, operation)?;
    }
    out.u64(plan.destructive.len() as u64)?;
    for loss in &plan.destructive {
        out.span(loss.relation.as_bytes())?;
        match &loss.field {
            None => out.tag(0)?,
            Some(field) => {
                out.tag(1)?;
                out.span(field.as_bytes())?;
            }
        }
    }
    Ok(out.finish())
}

fn put_operation(out: &mut Frame, operation: &Operation) -> Result<(), FrameError> {
    match operation {
        Operation::MapRelation {
            source,
            target,
            fields,
        } => {
            out.tag(OP_MAP)?;
            out.span(source.as_bytes())?;
            out.span(target.as_bytes())?;
            out.u64(fields.len() as u64)?;
            for field in fields {
                out.span(field.target.as_bytes())?;
                put_expr(out, &field.expression, 0)?;
            }
        }
        Operation::EmptyRelation { target } => {
            out.tag(OP_EMPTY)?;
            out.span(target.as_bytes())?;
        }
        Operation::DropRelation { source } => {
            out.tag(OP_DROP)?;
            out.span(source.as_bytes())?;
        }
        Operation::Seed { target, rows } => {
            out.tag(OP_SEED)?;
            out.span(target.as_bytes())?;
            out.u64(rows.len() as u64)?;
            for row in rows {
                out.u64(row.len() as u64)?;
                for value in row {
                    put_value(out, value)?;
                }
            }
        }
        Operation::ValidateSchema { schema } => {
            out.tag(OP_VALIDATE)?;
            out.bytes(&schema.0)?;
        }
    }
    Ok(())
}

fn put_expr(out: &mut Frame, expr: &PlanExpr, depth: usize) -> Result<(), FrameError> {
    if depth > MAX_EXPR_DEPTH {
        return Err(FrameError::LimitExceeded);
    }
    match expr {
        PlanExpr::Field(name) => {
            out.tag(EXPR_FIELD)?;
            out.span(name.as_bytes())?;
        }
        PlanExpr::Literal(value) => {
            out.tag(EXPR_LITERAL)?;
            put_value(out, value)?;
        }
        PlanExpr::Add(left, right) => {
            out.tag(EXPR_ADD)?;
            put_expr(out, left, depth + 1)?;
            put_expr(out, right, depth + 1)?;
        }
        PlanExpr::Subtract(left, right) => {
            out.tag(EXPR_SUBTRACT)?;
            put_expr(out, left, depth + 1)?;
            put_expr(out, right, depth + 1)?;
        }
        PlanExpr::Multiply(left, right) => {
            out.tag(EXPR_MULTIPLY)?;
            put_expr(out, left, depth + 1)?;
            put_expr(out, right, depth + 1)?;
        }
        PlanExpr::Divide(left, right) => {
            out.tag(EXPR_DIVIDE)?;
            put_expr(out, left, depth + 1)?;
            put_expr(out, right, depth + 1)?;
        }
        PlanExpr::Negate(inner) => {
            out.tag(EXPR_NEGATE)?;
            put_expr(out, inner, depth + 1)?;
        }
        PlanExpr::Cast { kind, expr } => {
            out.tag(EXPR_CAST)?;
            out.tag(match kind {
                NumericCast::ToF64 => 0,
                NumericCast::ToF64Exact => 1,
                NumericCast::ToI64Exact => 2,
                NumericCast::ToU64Exact => 3,
            })?;
            put_expr(out, expr, depth + 1)?;
        }
        PlanExpr::IsNaN(inner) => {
            out.tag(EXPR_IS_NAN)?;
            put_expr(out, inner, depth + 1)?;
        }
        PlanExpr::IsFinite(inner) => {
            out.tag(EXPR_IS_FINITE)?;
            put_expr(out, inner, depth + 1)?;
        }
    }
    Ok(())
}

fn put_value(out: &mut Frame, value: &Value) -> Result<(), FrameError> {
    match value {
        Value::Bool(v) => {
            out.tag(VALUE_BOOL)?;
            out.tag(u8::from(*v))?;
        }
        Value::U64(v) => {
            out.tag(VALUE_U64)?;
            out.u64(*v)?;
        }
        Value::I64(v) => {
            out.tag(VALUE_I64)?;
            out.bytes(&v.to_be_bytes())?;
        }
        Value::F64(v) => {
            out.tag(VALUE_F64)?;
            out.bytes(&v.to_be_bytes())?;
        }
        Value::String(text) => {
            out.tag(VALUE_STRING)?;
            out.span(text.as_bytes())?;
        }
        Value::FixedBytes(bytes) => {
            out.tag(VALUE_FIXED_BYTES)?;
            out.span(bytes)?;
        }
        Value::IntervalU64(interval) => {
            out.tag(VALUE_INTERVAL_U64)?;
            out.u64(interval.start())?;
            out.u64(interval.end())?;
        }
        Value::IntervalI64(interval) => {
            out.tag(VALUE_INTERVAL_I64)?;
            out.bytes(&interval.start().to_be_bytes())?;
            out.bytes(&interval.end().to_be_bytes())?;
        }
        Value::Id128(id) => {
            out.tag(VALUE_ID128)?;
            out.bytes(id.as_bytes())?;
        }
        Value::IntervalF64(interval) => {
            out.tag(VALUE_INTERVAL_F64)?;
            out.bytes(&interval.start().to_be_bytes())?;
            out.bytes(&interval.end().to_be_bytes())?;
        }
    }
    Ok(())
}

/// Decode one canonical plan frame strictly. Grammar only — coverage,
/// typing and schema binding are [`super::compile`]'s judgment.
/// # Errors
/// Refuses malformed frames, unknown tags and trailing bytes.
pub fn decode_plan(bytes: &[u8], cap: usize) -> Result<Plan, PlanError> {
    let mut input = Reader::begin(bytes, KIND_PLAN, cap)?;
    let sequence = input.u64()?;
    let label = StepLabel::new(
        std::str::from_utf8(input.span(cap)?).map_err(|_| PlanError::Shape("label utf8"))?,
    )?;
    let from_schema = SchemaId(input.array()?);
    let to_schema = SchemaId(input.array()?);
    let operation_count = input.count(2)?;
    let mut operations = Vec::new();
    operations
        .try_reserve_exact(operation_count)
        .map_err(|_| PlanError::Frame(FrameError::Allocation))?;
    for _ in 0..operation_count {
        operations.push(read_operation(&mut input, cap)?);
    }
    let loss_count = input.count(9)?;
    let mut destructive = Vec::new();
    destructive
        .try_reserve_exact(loss_count)
        .map_err(|_| PlanError::Frame(FrameError::Allocation))?;
    for _ in 0..loss_count {
        let relation = read_name(&mut input, cap)?;
        let field = match input.tag()? {
            (_, 0) => None,
            (_, 1) => Some(read_name(&mut input, cap)?),
            (at, got) => return Err(PlanError::Frame(FrameError::Tag { at, got })),
        };
        destructive.push(Loss { relation, field });
    }
    input.end()?;
    Ok(Plan {
        sequence,
        label,
        from_schema,
        to_schema,
        operations,
        destructive,
    })
}

fn read_name(input: &mut Reader<'_>, cap: usize) -> Result<Box<str>, PlanError> {
    Ok(std::str::from_utf8(input.span(cap)?)
        .map_err(|_| PlanError::Shape("name utf8"))?
        .into())
}

fn read_operation(input: &mut Reader<'_>, cap: usize) -> Result<Operation, PlanError> {
    match input.tag()? {
        (_, OP_MAP) => {
            let source = read_name(input, cap)?;
            let target = read_name(input, cap)?;
            let field_count = input.count(10)?;
            let mut fields = Vec::new();
            fields
                .try_reserve_exact(field_count)
                .map_err(|_| PlanError::Frame(FrameError::Allocation))?;
            for _ in 0..field_count {
                let target_field = read_name(input, cap)?;
                let expression = read_expr(input, cap, 0)?;
                fields.push(FieldMap {
                    target: target_field,
                    expression,
                });
            }
            Ok(Operation::MapRelation {
                source,
                target,
                fields,
            })
        }
        (_, OP_EMPTY) => Ok(Operation::EmptyRelation {
            target: read_name(input, cap)?,
        }),
        (_, OP_DROP) => Ok(Operation::DropRelation {
            source: read_name(input, cap)?,
        }),
        (_, OP_SEED) => {
            let target = read_name(input, cap)?;
            let row_count = input.count(8)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(row_count)
                .map_err(|_| PlanError::Frame(FrameError::Allocation))?;
            for _ in 0..row_count {
                let value_count = input.count(2)?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(value_count)
                    .map_err(|_| PlanError::Frame(FrameError::Allocation))?;
                for _ in 0..value_count {
                    values.push(read_value(input, cap)?);
                }
                rows.push(values.into_boxed_slice());
            }
            Ok(Operation::Seed { target, rows })
        }
        (_, OP_VALIDATE) => Ok(Operation::ValidateSchema {
            schema: SchemaId(input.array()?),
        }),
        (at, got) => Err(PlanError::Frame(FrameError::Tag { at, got })),
    }
}

fn read_expr(input: &mut Reader<'_>, cap: usize, depth: usize) -> Result<PlanExpr, PlanError> {
    if depth > MAX_EXPR_DEPTH {
        return Err(PlanError::Shape("expression too deep"));
    }
    let binary = |input: &mut Reader<'_>| -> Result<(Box<PlanExpr>, Box<PlanExpr>), PlanError> {
        let left = Box::new(read_expr(input, cap, depth + 1)?);
        let right = Box::new(read_expr(input, cap, depth + 1)?);
        Ok((left, right))
    };
    match input.tag()? {
        (_, EXPR_FIELD) => Ok(PlanExpr::Field(read_name(input, cap)?)),
        (_, EXPR_LITERAL) => Ok(PlanExpr::Literal(read_value(input, cap)?)),
        (_, EXPR_ADD) => {
            let (left, right) = binary(input)?;
            Ok(PlanExpr::Add(left, right))
        }
        (_, EXPR_SUBTRACT) => {
            let (left, right) = binary(input)?;
            Ok(PlanExpr::Subtract(left, right))
        }
        (_, EXPR_MULTIPLY) => {
            let (left, right) = binary(input)?;
            Ok(PlanExpr::Multiply(left, right))
        }
        (_, EXPR_DIVIDE) => {
            let (left, right) = binary(input)?;
            Ok(PlanExpr::Divide(left, right))
        }
        (_, EXPR_NEGATE) => Ok(PlanExpr::Negate(Box::new(read_expr(
            input,
            cap,
            depth + 1,
        )?))),
        (_, EXPR_CAST) => {
            let kind = match input.tag()? {
                (_, 0) => NumericCast::ToF64,
                (_, 1) => NumericCast::ToF64Exact,
                (_, 2) => NumericCast::ToI64Exact,
                (_, 3) => NumericCast::ToU64Exact,
                (at, got) => return Err(PlanError::Frame(FrameError::Tag { at, got })),
            };
            Ok(PlanExpr::Cast {
                kind,
                expr: Box::new(read_expr(input, cap, depth + 1)?),
            })
        }
        (_, EXPR_IS_NAN) => Ok(PlanExpr::IsNaN(Box::new(read_expr(input, cap, depth + 1)?))),
        (_, EXPR_IS_FINITE) => Ok(PlanExpr::IsFinite(Box::new(read_expr(
            input,
            cap,
            depth + 1,
        )?))),
        (at, got) => Err(PlanError::Frame(FrameError::Tag { at, got })),
    }
}

fn read_value(input: &mut Reader<'_>, cap: usize) -> Result<Value, PlanError> {
    let value = match input.tag()? {
        (_, VALUE_BOOL) => match input.tag()? {
            (_, 0) => Value::Bool(false),
            (_, 1) => Value::Bool(true),
            (at, got) => return Err(PlanError::Frame(FrameError::Tag { at, got })),
        },
        (_, VALUE_U64) => Value::U64(input.u64()?),
        (_, VALUE_I64) => Value::I64(i64::from_be_bytes(input.array()?)),
        (_, VALUE_F64) => Value::F64(
            bumbledb::F64::from_canonical_be_bytes(input.array()?)
                .map_err(|_| PlanError::Shape("noncanonical f64"))?,
        ),
        (_, VALUE_STRING) => Value::String(
            std::str::from_utf8(input.span(cap)?)
                .map_err(|_| PlanError::Shape("string utf8"))?
                .into(),
        ),
        (_, VALUE_FIXED_BYTES) => Value::FixedBytes(Box::from(input.span(cap)?)),
        (_, VALUE_INTERVAL_U64) => {
            let start = input.u64()?;
            let end = input.u64()?;
            Value::IntervalU64(
                bumbledb::Interval::new(start, end).ok_or(PlanError::Shape("interval"))?,
            )
        }
        (_, VALUE_INTERVAL_I64) => {
            let start = i64::from_be_bytes(input.array()?);
            let end = i64::from_be_bytes(input.array()?);
            Value::IntervalI64(
                bumbledb::Interval::new(start, end).ok_or(PlanError::Shape("interval"))?,
            )
        }
        (_, VALUE_ID128) => Value::Id128(bumbledb::Id128::from_bytes(input.array()?)),
        (_, VALUE_INTERVAL_F64) => {
            let start = bumbledb::F64::from_canonical_be_bytes(input.array()?)
                .map_err(|_| PlanError::Shape("noncanonical f64"))?;
            let end = bumbledb::F64::from_canonical_be_bytes(input.array()?)
                .map_err(|_| PlanError::Shape("noncanonical f64"))?;
            Value::IntervalF64(
                bumbledb::Interval::new(start, end).ok_or(PlanError::Shape("interval"))?,
            )
        }
        (at, got) => return Err(PlanError::Frame(FrameError::Tag { at, got })),
    };
    Ok(value)
}

// ---------------------------------------------------------------------------
// Repo JSON boundary.
// ---------------------------------------------------------------------------

/// Parse one repo `NNNN-label.plan.json` text into canonical plan data.
/// # Errors
/// Grammar refusals name the first offense; unknown `planVersion` refuses
/// before anything else, so an unsupported codec never half-parses.
pub fn parse_plan(raw: &str) -> Result<Plan, PlanError> {
    let json = read_tree(raw).map_err(PlanError::Json)?;
    if json["planVersion"].as_u64() != Some(u64::from(frame::LAYOUT)) {
        return Err(PlanError::Shape("unsupported planVersion"));
    }
    let sequence = parse_u64(&json["sequence"]).map_err(PlanError::Shape)?;
    let label = StepLabel::new(json["id"].as_str().ok_or(PlanError::Shape("id"))?)?;
    let from_schema = parse_schema_id(&json["fromSchemaId"])?;
    let to_schema = parse_schema_id(&json["toSchemaId"])?;
    let operations = json["operations"]
        .as_array()
        .ok_or(PlanError::Shape("operations"))?
        .iter()
        .map(parse_operation)
        .collect::<Result<Vec<_>, _>>()?;
    let destructive = match json.get("destructive") {
        None => Vec::new(),
        Some(list) => list
            .as_array()
            .ok_or(PlanError::Shape("destructive"))?
            .iter()
            .map(|entry| {
                Ok(Loss {
                    relation: entry["relation"]
                        .as_str()
                        .ok_or(PlanError::Shape("destructive relation"))?
                        .into(),
                    field: match entry.get("field") {
                        None | Some(Json::Null) => None,
                        Some(field) => Some(
                            field
                                .as_str()
                                .ok_or(PlanError::Shape("destructive field"))?
                                .into(),
                        ),
                    },
                })
            })
            .collect::<Result<Vec<_>, PlanError>>()?,
    };
    Ok(Plan {
        sequence,
        label,
        from_schema,
        to_schema,
        operations,
        destructive,
    })
}

pub(crate) fn parse_schema_id(json: &Json) -> Result<SchemaId, PlanError> {
    Ok(SchemaId(
        unhex_exact::<32>(json.as_str().ok_or(PlanError::Shape("schema id"))?)
            .map_err(PlanError::Shape)?,
    ))
}

fn parse_operation(json: &Json) -> Result<Operation, PlanError> {
    match json["kind"].as_str() {
        Some("map-relation") => {
            let fields = json["fields"]
                .as_array()
                .ok_or(PlanError::Shape("fields"))?
                .iter()
                .map(|field| {
                    Ok(FieldMap {
                        target: field["target"]
                            .as_str()
                            .ok_or(PlanError::Shape("field target"))?
                            .into(),
                        expression: parse_expr(&field["expression"], 0)?,
                    })
                })
                .collect::<Result<Vec<_>, PlanError>>()?;
            Ok(Operation::MapRelation {
                source: name(json, "source")?,
                target: name(json, "target")?,
                fields,
            })
        }
        Some("empty-relation") => Ok(Operation::EmptyRelation {
            target: name(json, "target")?,
        }),
        Some("drop-relation") => Ok(Operation::DropRelation {
            source: name(json, "source")?,
        }),
        Some("seed") => {
            let rows = json["rows"]
                .as_array()
                .ok_or(PlanError::Shape("seed rows"))?
                .iter()
                .map(|row| {
                    Ok(row
                        .as_array()
                        .ok_or(PlanError::Shape("seed row"))?
                        .iter()
                        .map(|value| parse_value(value).map_err(PlanError::Shape))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_boxed_slice())
                })
                .collect::<Result<Vec<_>, PlanError>>()?;
            Ok(Operation::Seed {
                target: name(json, "target")?,
                rows,
            })
        }
        Some("validate-schema") => Ok(Operation::ValidateSchema {
            schema: parse_schema_id(&json["schemaId"])?,
        }),
        _ => Err(PlanError::Shape("unknown operation kind")),
    }
}

fn name(json: &Json, field: &'static str) -> Result<Box<str>, PlanError> {
    Ok(json[field].as_str().ok_or(PlanError::Shape(field))?.into())
}

fn parse_expr(json: &Json, depth: usize) -> Result<PlanExpr, PlanError> {
    if depth > MAX_EXPR_DEPTH {
        return Err(PlanError::Shape("expression too deep"));
    }
    match json["kind"].as_str() {
        Some("field") => Ok(PlanExpr::Field(name(json, "name")?)),
        Some("literal") => Ok(PlanExpr::Literal(
            parse_value(&json["value"]).map_err(PlanError::Shape)?,
        )),
        Some("negate") => Ok(PlanExpr::Negate(Box::new(parse_expr(
            &json["expr"],
            depth + 1,
        )?))),
        Some("isNaN") => Ok(PlanExpr::IsNaN(Box::new(parse_expr(
            &json["expr"],
            depth + 1,
        )?))),
        Some("isFinite") => Ok(PlanExpr::IsFinite(Box::new(parse_expr(
            &json["expr"],
            depth + 1,
        )?))),
        Some("cast") => {
            let kind = match json["cast"].as_str() {
                Some("toF64") => NumericCast::ToF64,
                Some("toF64Exact") => NumericCast::ToF64Exact,
                Some("toI64Exact") => NumericCast::ToI64Exact,
                Some("toU64Exact") => NumericCast::ToU64Exact,
                _ => return Err(PlanError::Shape("unknown cast")),
            };
            Ok(PlanExpr::Cast {
                kind,
                expr: Box::new(parse_expr(&json["expr"], depth + 1)?),
            })
        }
        Some(binary @ ("add" | "subtract" | "multiply" | "divide")) => {
            let left = Box::new(parse_expr(&json["left"], depth + 1)?);
            let right = Box::new(parse_expr(&json["right"], depth + 1)?);
            Ok(match binary {
                "add" => PlanExpr::Add(left, right),
                "subtract" => PlanExpr::Subtract(left, right),
                "multiply" => PlanExpr::Multiply(left, right),
                _ => PlanExpr::Divide(left, right),
            })
        }
        _ => Err(PlanError::Shape("unknown expression kind")),
    }
}

/// Render the deterministic repo JSON for a plan: fixed key order, two-space
/// indentation, one trailing newline. `parse_plan(render_plan(p)) == p` and
/// the digest of the reparsed plan's canonical bytes is unchanged.
#[must_use]
pub fn render_plan(plan: &Plan) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"planVersion\": 1,\n  \"sequence\": \"");
    out.push_str(&plan.sequence.to_string());
    out.push_str("\",\n  \"id\": ");
    push_string(&mut out, plan.label.as_str());
    out.push_str(",\n  \"fromSchemaId\": \"");
    push_hex(&mut out, &plan.from_schema.0);
    out.push_str("\",\n  \"toSchemaId\": \"");
    push_hex(&mut out, &plan.to_schema.0);
    out.push_str("\",\n  \"operations\": [");
    for (index, operation) in plan.operations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        render_operation(&mut out, operation);
    }
    if plan.operations.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("\n  ],\n");
    }
    out.push_str("  \"destructive\": [");
    for (index, loss) in plan.destructive.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        push_indent(&mut out, 2);
        out.push_str("{\"relation\":");
        push_string(&mut out, &loss.relation);
        if let Some(field) = &loss.field {
            out.push_str(",\"field\":");
            push_string(&mut out, field);
        }
        out.push('}');
    }
    if plan.destructive.is_empty() {
        out.push_str("]\n}\n");
    } else {
        out.push_str("\n  ]\n}\n");
    }
    out
}

fn render_operation(out: &mut String, operation: &Operation) {
    push_indent(out, 2);
    match operation {
        Operation::MapRelation {
            source,
            target,
            fields,
        } => {
            out.push_str("{\n");
            push_indent(out, 3);
            out.push_str("\"kind\": \"map-relation\",\n");
            push_indent(out, 3);
            out.push_str("\"source\": ");
            push_string(out, source);
            out.push_str(",\n");
            push_indent(out, 3);
            out.push_str("\"target\": ");
            push_string(out, target);
            out.push_str(",\n");
            push_indent(out, 3);
            out.push_str("\"fields\": [");
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(out, 4);
                out.push_str("{\"target\":");
                push_string(out, &field.target);
                out.push_str(",\"expression\":");
                render_expr(out, &field.expression);
                out.push('}');
            }
            if fields.is_empty() {
                out.push(']');
            } else {
                out.push('\n');
                push_indent(out, 3);
                out.push(']');
            }
            out.push('\n');
            push_indent(out, 2);
            out.push('}');
        }
        Operation::EmptyRelation { target } => {
            out.push_str("{\"kind\":\"empty-relation\",\"target\":");
            push_string(out, target);
            out.push('}');
        }
        Operation::DropRelation { source } => {
            out.push_str("{\"kind\":\"drop-relation\",\"source\":");
            push_string(out, source);
            out.push('}');
        }
        Operation::Seed { target, rows } => {
            out.push_str("{\n");
            push_indent(out, 3);
            out.push_str("\"kind\": \"seed\",\n");
            push_indent(out, 3);
            out.push_str("\"target\": ");
            push_string(out, target);
            out.push_str(",\n");
            push_indent(out, 3);
            out.push_str("\"rows\": [");
            for (index, row) in rows.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(out, 4);
                out.push('[');
                for (value_index, value) in row.iter().enumerate() {
                    if value_index > 0 {
                        out.push(',');
                    }
                    render_value(out, value);
                }
                out.push(']');
            }
            if rows.is_empty() {
                out.push(']');
            } else {
                out.push('\n');
                push_indent(out, 3);
                out.push(']');
            }
            out.push('\n');
            push_indent(out, 2);
            out.push('}');
        }
        Operation::ValidateSchema { schema } => {
            out.push_str("{\"kind\":\"validate-schema\",\"schemaId\":\"");
            push_hex(out, &schema.0);
            out.push_str("\"}");
        }
    }
}

fn render_expr(out: &mut String, expr: &PlanExpr) {
    match expr {
        PlanExpr::Field(field) => {
            out.push_str("{\"kind\":\"field\",\"name\":");
            push_string(out, field);
            out.push('}');
        }
        PlanExpr::Literal(value) => {
            out.push_str("{\"kind\":\"literal\",\"value\":");
            render_value(out, value);
            out.push('}');
        }
        PlanExpr::Negate(inner) => render_unary(out, "negate", inner),
        PlanExpr::IsNaN(inner) => render_unary(out, "isNaN", inner),
        PlanExpr::IsFinite(inner) => render_unary(out, "isFinite", inner),
        PlanExpr::Cast { kind, expr } => {
            out.push_str("{\"kind\":\"cast\",\"cast\":\"");
            out.push_str(match kind {
                NumericCast::ToF64 => "toF64",
                NumericCast::ToF64Exact => "toF64Exact",
                NumericCast::ToI64Exact => "toI64Exact",
                NumericCast::ToU64Exact => "toU64Exact",
            });
            out.push_str("\",\"expr\":");
            render_expr(out, expr);
            out.push('}');
        }
        PlanExpr::Add(left, right) => render_binary(out, "add", left, right),
        PlanExpr::Subtract(left, right) => render_binary(out, "subtract", left, right),
        PlanExpr::Multiply(left, right) => render_binary(out, "multiply", left, right),
        PlanExpr::Divide(left, right) => render_binary(out, "divide", left, right),
    }
}

fn render_unary(out: &mut String, kind: &str, inner: &PlanExpr) {
    out.push_str("{\"kind\":\"");
    out.push_str(kind);
    out.push_str("\",\"expr\":");
    render_expr(out, inner);
    out.push('}');
}

fn render_binary(out: &mut String, kind: &str, left: &PlanExpr, right: &PlanExpr) {
    out.push_str("{\"kind\":\"");
    out.push_str(kind);
    out.push_str("\",\"left\":");
    render_expr(out, left);
    out.push_str(",\"right\":");
    render_expr(out, right);
    out.push('}');
}
