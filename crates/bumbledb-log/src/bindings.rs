//! Mechanical SDK declarations from a verified native schema descriptor.
//!
//! The declarations use the SDK's existing field, relation and law algebra.
//! There is no transformation program or second row representation.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Write as _};

use bumbledb::Value;
use bumbledb::schema::{
    Bound, FieldId, RelationId, SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight,
};

use crate::json::push_string;

#[derive(Debug)]
pub enum BindingError {
    Schema(bumbledb::Error),
    Unrepresentable {
        coordinate: String,
        reason: &'static str,
    },
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(error) => write!(f, "{error}"),
            Self::Unrepresentable { coordinate, reason } => write!(f, "{coordinate}: {reason}"),
        }
    }
}

impl std::error::Error for BindingError {}

type Coordinate = (usize, usize);
type Rosters = BTreeMap<Coordinate, usize>;

fn refuse(coordinate: impl Into<String>, reason: &'static str) -> BindingError {
    BindingError::Unrepresentable {
        coordinate: coordinate.into(),
        reason,
    }
}

fn quote(value: &str) -> String {
    let mut out = String::new();
    push_string(&mut out, value);
    out
}

fn name(value: &str, coordinate: &str) -> Result<(), BindingError> {
    let integer = value == "0"
        || (value.starts_with(|c: char| c.is_ascii_digit() && c != '0')
            && value.bytes().all(|b| b.is_ascii_digit()));
    if integer || value.contains('.') {
        return Err(refuse(
            coordinate,
            "SDK declaration names cannot be numeric indices or contain dots",
        ));
    }
    Ok(())
}

fn field_name(schema: &SchemaDescriptor, relation: usize, field: usize) -> &str {
    let descriptor = &schema.relations[relation];
    if descriptor.extension.is_some() {
        if field == 0 {
            "id"
        } else {
            &descriptor.fields[field - 1].name
        }
    } else {
        &descriptor.fields[field].name
    }
}

fn coordinate(schema: &SchemaDescriptor, (relation, field): Coordinate) -> String {
    format!(
        "{}.{}",
        schema.relations[relation].name,
        field_name(schema, relation, field)
    )
}

/// A source field has a roster only when an unconditional inclusion proves
/// every source value belongs to it. Classes and conditional inclusions do
/// not prove global membership. A work queue traverses each edge once.
fn rosters(schema: &SchemaDescriptor) -> Result<Rosters, BindingError> {
    let mut edges: BTreeMap<Coordinate, Vec<Coordinate>> = BTreeMap::new();
    for statement in &schema.statements {
        if let StatementDescriptor::Containment { source, target } = statement
            && source.selection.is_empty()
        {
            for (a, b) in source.projection.iter().zip(&target.projection) {
                edges
                    .entry((target.relation.0 as usize, usize::from(b.0)))
                    .or_default()
                    .push((source.relation.0 as usize, usize::from(a.0)));
            }
        }
    }
    let mut known = Rosters::new();
    let mut pending = VecDeque::new();
    for (r, relation) in schema.relations.iter().enumerate() {
        if relation.extension.is_some() {
            known.insert((r, 0), r);
            pending.push_back((r, 0));
        }
    }
    while let Some(at) = pending.pop_front() {
        let roster = known[&at];
        for &source in edges.get(&at).into_iter().flatten() {
            if let Some(&prior) = known.get(&source) {
                if prior != roster {
                    return Err(refuse(
                        coordinate(schema, source),
                        "multiple closed rosters constrain this field",
                    ));
                }
            } else {
                known.insert(source, roster);
                pending.push_back(source);
            }
        }
    }
    for (index, statement) in schema.statements.iter().enumerate() {
        match statement {
            StatementDescriptor::Containment { source, target }
            | StatementDescriptor::Capacity { source, target, .. } => {
                for (a, b) in source.projection.iter().zip(&target.projection) {
                    let a = (source.relation.0 as usize, usize::from(a.0));
                    let b = (target.relation.0 as usize, usize::from(b.0));
                    if known.get(&a) != known.get(&b) {
                        return Err(refuse(
                            format!(
                                "statement {index}: {} / {}",
                                coordinate(schema, a),
                                coordinate(schema, b)
                            ),
                            "paired SDK fields require the same globally proven closed roster; conditional membership alone is insufficient",
                        ));
                    }
                }
            }
            StatementDescriptor::Functionality { .. } => {}
        }
    }
    Ok(known)
}

fn field_type(value: &ValueType) -> String {
    match value {
        ValueType::Bool => "db.bool".into(),
        ValueType::U64 => "db.u64".into(),
        ValueType::I64 => "db.i64".into(),
        ValueType::F64 => "db.f64".into(),
        ValueType::String => "db.str".into(),
        ValueType::Uuid => "db.uuid".into(),
        ValueType::FixedBytes { len } => format!("db.bytes({len})"),
        ValueType::Interval { element } => format!(
            "db.interval(db.{})",
            match element {
                bumbledb::schema::IntervalElement::U64 => "u64",
                bumbledb::schema::IntervalElement::I64 => "i64",
                bumbledb::schema::IntervalElement::F64 => "f64",
            }
        ),
        ValueType::FixedInterval { element, width } => format!(
            "db.interval(db.{}, {width}n)",
            match element {
                bumbledb::schema::FixedIntervalElement::U64 => "u64",
                bumbledb::schema::FixedIntervalElement::I64 => "i64",
            }
        ),
    }
}

fn float(value: bumbledb::F64) -> String {
    let value = value.to_f64();
    if value.is_nan() {
        "NaN".into()
    } else if value == f64::INFINITY {
        "Infinity".into()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".into()
    } else {
        value.to_string()
    }
}

fn value(
    schema: &SchemaDescriptor,
    rosters: &Rosters,
    at: Coordinate,
    value: &Value,
) -> Result<String, BindingError> {
    if let Some(&r) = rosters.get(&at) {
        if let Value::U64(ordinal) = value
            && let Some(row) = usize::try_from(*ordinal)
                .ok()
                .and_then(|i| schema.relations[r].extension.as_ref()?.get(i))
        {
            return Ok(quote(&row.handle));
        }
        return Err(refuse(
            coordinate(schema, at),
            "value is outside the globally proven closed roster",
        ));
    }
    Ok(match value {
        Value::Bool(v) => v.to_string(),
        Value::U64(v) => format!("{v}n"),
        Value::I64(v) => format!("{v}n"),
        Value::F64(v) => float(*v),
        Value::String(v) => quote(v),
        Value::Uuid(v) => quote(&v.to_string()),
        Value::FixedBytes(v) => format!(
            "new Uint8Array([{}])",
            v.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
        ),
        Value::IntervalU64(v) => format!("{{ start: {}n, end: {}n }}", v.start(), v.end()),
        Value::IntervalI64(v) => format!("{{ start: {}n, end: {}n }}", v.start(), v.end()),
        Value::IntervalF64(v) => {
            format!("{{ start: {}, end: {} }}", float(v.start()), float(v.end()))
        }
    })
}

fn projection(schema: &SchemaDescriptor, r: RelationId, fields: &[FieldId]) -> String {
    format!(
        "[{}]",
        fields
            .iter()
            .map(|f| quote(field_name(schema, r.0 as usize, usize::from(f.0))))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn face(schema: &SchemaDescriptor, rosters: &Rosters, side: &Side) -> Result<String, BindingError> {
    let r = side.relation.0 as usize;
    let mut owner = format!("r{r}");
    if !side.selection.is_empty() {
        let mut selections = Vec::new();
        for (f, literals) in &side.selection {
            if let Some(&roster) = rosters.get(&(r, usize::from(f.0))) {
                let intrinsic = r == roster && f.0 == 0;
                let direct = schema.statements.iter().any(|statement| matches!(statement,
                    StatementDescriptor::Containment { source, target }
                        if source.relation == side.relation && source.projection.as_ref() == [*f]
                            && target.relation.0 as usize == roster && target.projection.as_ref() == [FieldId(0)]
                ));
                if !intrinsic && !direct {
                    return Err(refuse(
                        coordinate(schema, (r, usize::from(f.0))),
                        "the SDK requires a direct closed-reference containment to author a handle selection",
                    ));
                }
            }
            let values = literals
                .literals()
                .iter()
                .map(|v| value(schema, rosters, (r, usize::from(f.0)), v))
                .collect::<Result<Vec<_>, _>>()?;
            let entry = if values.len() == 1 {
                values[0].clone()
            } else {
                format!("[{}]", values.join(", "))
            };
            selections.push(format!(
                "[{}]: {entry}",
                quote(field_name(schema, r, usize::from(f.0)))
            ));
        }
        owner = format!("db.select({owner}, {{ {} }})", selections.join(", "));
    }
    Ok(format!(
        "db.on({owner}, {})",
        projection(schema, side.relation, &side.projection)
    ))
}

/// Emit deterministic, ordinary TypeScript bindings for one native schema.
///
/// # Errors
/// Refuses invalid native schemas or descriptors the existing SDK cannot
/// represent without changing names, types, laws or statement order.
pub fn emit(schema: &SchemaDescriptor) -> Result<String, BindingError> {
    crate::schema_file::schema_id(schema).map_err(BindingError::Schema)?;
    for relation in &schema.relations {
        name(&relation.name, &relation.name)?;
        for field in &relation.fields {
            name(&field.name, &format!("{}.{}", relation.name, field.name))?;
        }
    }
    let rosters = rosters(schema)?;
    let mut out = String::from(
        "// Generated from a native-verified schema snapshot.\nimport * as db from \"@bjornpagen/bumbledb\"\n\n",
    );
    declarations(schema, &rosters, &mut out)?;
    // The runtime retains every law. An array avoids recursive type-level
    // evaluation of thousands of laws; it invents no static class proofs.
    out.push_str("\nconst laws: db.Statement[] = [\n");
    for law in &schema.statements {
        let text = statement(schema, &rosters, law)?;
        let _ = writeln!(out, "  {text},");
    }
    out.push_str("]\n\nexport const schema = db.schema(\"Snapshot\", {\n");
    for (r, relation) in schema.relations.iter().enumerate() {
        let _ = writeln!(out, "  [{}]: r{r},", quote(&relation.name));
    }
    out.push_str("}, laws)\n");
    Ok(out)
}

fn declarations(
    schema: &SchemaDescriptor,
    rosters: &Rosters,
    out: &mut String,
) -> Result<(), BindingError> {
    // Roster descriptors are ordinary public fields. Declaring them before
    // relations supports mutually referring closed payloads without casts.
    for (r, relation) in schema.relations.iter().enumerate() {
        if let Some(rows) = &relation.extension {
            let handles = rows
                .iter()
                .map(|row| quote(&row.handle))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "const h{r} = [{handles}] as const");
            if rosters
                .iter()
                .any(|(at, roster)| *roster == r && *at != (r, 0))
            {
                let _ = writeln!(
                    out,
                    "const c{r} = {{ kind: \"u64\", closed: {{ name: {}, handles: h{r} }} }} as const",
                    quote(&relation.name)
                );
            }
        }
    }
    for (r, relation) in schema.relations.iter().enumerate() {
        let offset = usize::from(relation.extension.is_some());
        let fields = relation
            .fields
            .iter()
            .enumerate()
            .map(|(f, field)| {
                let descriptor = rosters
                    .get(&(r, f + offset))
                    .map_or_else(|| field_type(&field.value_type), |r| format!("c{r}"));
                format!("[{}]: {descriptor}", quote(&field.name))
            })
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(rows) = &relation.extension {
            let _ = writeln!(
                out,
                "export const r{r} = db.closed({}, h{r}, {{ {fields} }}, {{",
                quote(&relation.name)
            );
            for row in rows {
                let payload = row
                    .values
                    .iter()
                    .enumerate()
                    .map(|(f, v)| {
                        Ok(format!(
                            "[{}]: {}",
                            quote(&relation.fields[f].name),
                            value(schema, rosters, (r, f + 1), v)?
                        ))
                    })
                    .collect::<Result<Vec<_>, BindingError>>()?
                    .join(", ");
                let _ = writeln!(out, "  [{}]: {{ {payload} }},", quote(&row.handle));
            }
            out.push_str("})\n");
        } else {
            let _ = writeln!(
                out,
                "export const r{r} = db.relation({}, {{ {fields} }})",
                quote(&relation.name)
            );
        }
    }
    Ok(())
}

fn statement(
    schema: &SchemaDescriptor,
    rosters: &Rosters,
    statement: &StatementDescriptor,
) -> Result<String, BindingError> {
    Ok(match statement {
        StatementDescriptor::Functionality {
            relation,
            projection: fields,
        } => {
            if schema.relations[relation.0 as usize].extension.is_some() {
                return Err(refuse(
                    format!("relation {}", relation.0),
                    "explicit keys on closed relations cannot be represented by the SDK",
                ));
            }
            format!(
                "db.key(r{}, {})",
                relation.0,
                projection(schema, *relation, fields)
            )
        }
        StatementDescriptor::Containment { source, target } => format!(
            "db.contained({}, {})",
            face(schema, rosters, source)?,
            face(schema, rosters, target)?
        ),
        StatementDescriptor::Capacity {
            source,
            target,
            weight,
            lo,
            hi,
        } => {
            let upper = match hi {
                None => quote("*"),
                Some(Bound::Lit(v)) => format!("{v}n"),
                Some(Bound::TargetField(f)) => format!(
                    "db.ref({})",
                    quote(field_name(
                        schema,
                        target.relation.0 as usize,
                        usize::from(f.0)
                    ))
                ),
                Some(Bound::TargetDuration(f)) => format!(
                    "db.duration({})",
                    quote(field_name(
                        schema,
                        target.relation.0 as usize,
                        usize::from(f.0)
                    ))
                ),
            };
            let weight = match weight {
                Weight::Unit => String::new(),
                Weight::Field(f) => format!(
                    ", weight: db.weigh({})",
                    quote(field_name(
                        schema,
                        source.relation.0 as usize,
                        usize::from(f.0)
                    ))
                ),
                Weight::DurationOf(f) => format!(
                    ", weight: db.weigh(db.duration({}))",
                    quote(field_name(
                        schema,
                        source.relation.0 as usize,
                        usize::from(f.0)
                    ))
                ),
            };
            format!(
                "db.capacity({}, {{ from: {}, within: db.within({lo}n, {upper}){weight} }})",
                face(schema, rosters, target)?,
                face(schema, rosters, source)?
            )
        }
    })
}
