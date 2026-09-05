//! The one native canonical schema-file grammar. The spelling is the
//! crate's corpus schema object — `{relations, statements}` — so a second
//! descriptor grammar cannot exist: the duty boundary, the migration
//! snapshots (`meta/NNNN.schema.json`) and the TypeScript generator all
//! read and write exactly this text through these entrypoints (C11).
//!
//! [`parse`] is the strict reader, [`render`] the deterministic writer
//! (`parse(render(d)) == d`, byte-stable output), and [`schema_id`] the
//! canonical identity: core validation plus the core v6 schema fingerprint.
//! There is no per-field generation attribute — the fresh machinery is
//! deleted, and a `"generation"` key refuses loudly rather than being
//! silently ignored.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, FixedIntervalElement, IntervalElement, LiteralSet,
    RelationDescriptor, RelationId, Row, SchemaDescriptor, Side, StatementDescriptor,
    ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{SchemaFingerprint, Value};

use crate::migration::json::{
    Json, pair2, parse_u64, parse_value, push_hex, push_indent, push_string, read_tree,
    render_value,
};

/// Why a theory file refused to become a descriptor.
#[derive(Debug)]
pub enum TheoryFile {
    Io(io::Error),
    Json(&'static str),
    Shape(&'static str),
}

impl fmt::Display for TheoryFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "theory file: {error}"),
            Self::Json(error) => write!(f, "theory file json: {error}"),
            Self::Shape(why) => write!(f, "theory file: {why}"),
        }
    }
}

impl std::error::Error for TheoryFile {}

/// Parse `{relations, statements}` once. Every later open takes the
/// descriptor.
///
/// # Errors
pub fn load(path: &Path) -> Result<SchemaDescriptor, TheoryFile> {
    let raw = fs::read_to_string(path).map_err(TheoryFile::Io)?;
    parse(&raw)
}

/// Parse the theory-file grammar from bytes already in memory.
///
/// # Errors
pub fn parse(raw: &str) -> Result<SchemaDescriptor, TheoryFile> {
    parse_schema(&read_tree(raw).map_err(TheoryFile::Json)?)
}

/// The canonical schema identity: validate the descriptor with the core and
/// fingerprint the sealed schema (the core v6 canonical stream). This is the
/// `SchemaId` every plan, manifest and history record cites; the log never
/// re-hashes schema text.
///
/// # Errors
/// The core's typed validation refusal for an inadmissible declaration.
pub fn schema_id(descriptor: &SchemaDescriptor) -> Result<SchemaFingerprint, bumbledb::Error> {
    let schema = descriptor.clone().validate()?;
    Ok(bumbledb::schema::fingerprint::fingerprint(&schema))
}

/// Render the deterministic canonical text for a descriptor: fixed key
/// order, two-space indentation, one trailing newline. `parse(render(d))`
/// reproduces `d` exactly; repo snapshot files are byte-stable.
#[must_use]
pub fn render(descriptor: &SchemaDescriptor) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"relations\": [");
    for (index, relation) in descriptor.relations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        render_relation(&mut out, relation);
    }
    if descriptor.relations.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("\n  ],\n");
    }
    out.push_str("  \"statements\": [");
    for (index, statement) in descriptor.statements.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        push_indent(&mut out, 2);
        render_statement(&mut out, statement);
    }
    if descriptor.statements.is_empty() {
        out.push_str("]\n}\n");
    } else {
        out.push_str("\n  ]\n}\n");
    }
    out
}

fn render_relation(out: &mut String, relation: &RelationDescriptor) {
    push_indent(out, 2);
    out.push_str("{\n");
    push_indent(out, 3);
    out.push_str("\"name\": ");
    push_string(out, &relation.name);
    out.push_str(",\n");
    push_indent(out, 3);
    out.push_str("\"fields\": [");
    for (index, field) in relation.fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('\n');
        push_indent(out, 4);
        out.push_str("{\"name\":");
        push_string(out, &field.name);
        out.push_str(",\"type\":");
        render_type(out, &field.value_type);
        out.push('}');
    }
    if relation.fields.is_empty() {
        out.push(']');
    } else {
        out.push('\n');
        push_indent(out, 3);
        out.push(']');
    }
    if let Some(rows) = &relation.extension {
        out.push_str(",\n");
        push_indent(out, 3);
        out.push_str("\"extension\": [");
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('\n');
            push_indent(out, 4);
            out.push_str("{\"handle\":");
            push_string(out, &row.handle);
            out.push_str(",\"values\":[");
            for (value_index, value) in row.values.iter().enumerate() {
                if value_index > 0 {
                    out.push(',');
                }
                render_value(out, value);
            }
            out.push_str("]}");
        }
        if rows.is_empty() {
            out.push(']');
        } else {
            out.push('\n');
            push_indent(out, 3);
            out.push(']');
        }
    }
    out.push('\n');
    push_indent(out, 2);
    out.push('}');
}

fn render_type(out: &mut String, value_type: &ValueType) {
    match value_type {
        ValueType::Bool => out.push_str("\"bool\""),
        ValueType::U64 => out.push_str("\"u64\""),
        ValueType::I64 => out.push_str("\"i64\""),
        ValueType::F64 => out.push_str("\"f64\""),
        ValueType::String => out.push_str("\"string\""),
        ValueType::Id128 => out.push_str("\"id128\""),
        ValueType::FixedBytes { len } => {
            out.push_str("{\"fixedBytes\":");
            out.push_str(&len.to_string());
            out.push('}');
        }
        ValueType::Interval { element } => {
            out.push_str("{\"interval\":");
            out.push_str(match element {
                IntervalElement::U64 => "\"u64\"",
                IntervalElement::I64 => "\"i64\"",
                IntervalElement::F64 => "\"f64\"",
            });
            out.push('}');
        }
        ValueType::FixedInterval { element, width } => {
            out.push_str("{\"fixedInterval\":{\"element\":");
            out.push_str(match element {
                FixedIntervalElement::U64 => "\"u64\"",
                FixedIntervalElement::I64 => "\"i64\"",
            });
            out.push_str(",\"width\":\"");
            out.push_str(&width.to_string());
            out.push_str("\"}}");
        }
    }
}

fn render_statement(out: &mut String, statement: &StatementDescriptor) {
    match statement {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => {
            out.push_str("{\"functionality\":{\"relation\":");
            out.push_str(&relation.0.to_string());
            out.push_str(",\"projection\":");
            render_projection(out, projection);
            out.push_str("}}");
        }
        StatementDescriptor::Containment { source, target } => {
            out.push_str("{\"containment\":{\"source\":");
            render_side(out, source);
            out.push_str(",\"target\":");
            render_side(out, target);
            out.push_str("}}");
        }
        StatementDescriptor::Capacity {
            target,
            weight,
            lo,
            hi,
            source,
        } => {
            out.push_str("{\"capacity\":{\"target\":");
            render_side(out, target);
            out.push_str(",\"weight\":");
            match weight {
                Weight::Unit => out.push_str("\"unit\""),
                Weight::Field(field) => {
                    out.push_str("{\"field\":");
                    out.push_str(&field.0.to_string());
                    out.push('}');
                }
                Weight::DurationOf(field) => {
                    out.push_str("{\"durationOf\":");
                    out.push_str(&field.0.to_string());
                    out.push('}');
                }
            }
            out.push_str(",\"lo\":\"");
            out.push_str(&lo.to_string());
            out.push_str("\",\"hi\":");
            match hi {
                None => out.push_str("null"),
                Some(Bound::Lit(value)) => {
                    out.push_str("{\"lit\":\"");
                    out.push_str(&value.to_string());
                    out.push_str("\"}");
                }
                Some(Bound::TargetField(field)) => {
                    out.push_str("{\"targetField\":");
                    out.push_str(&field.0.to_string());
                    out.push('}');
                }
                Some(Bound::TargetDuration(field)) => {
                    out.push_str("{\"targetDuration\":");
                    out.push_str(&field.0.to_string());
                    out.push('}');
                }
            }
            out.push_str(",\"source\":");
            render_side(out, source);
            out.push_str("}}");
        }
    }
}

fn render_projection(out: &mut String, projection: &[FieldId]) {
    out.push('[');
    for (index, field) in projection.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&field.0.to_string());
    }
    out.push(']');
}

fn render_side(out: &mut String, side: &Side) {
    out.push_str("{\"relation\":");
    out.push_str(&side.relation.0.to_string());
    out.push_str(",\"projection\":");
    render_projection(out, &side.projection);
    if !side.selection.is_empty() {
        out.push_str(",\"selection\":[");
        for (index, (field, literals)) in side.selection.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('[');
            out.push_str(&field.0.to_string());
            out.push_str(",[");
            for (literal_index, literal) in literals.literals().iter().enumerate() {
                if literal_index > 0 {
                    out.push(',');
                }
                render_value(out, literal);
            }
            out.push_str("]]");
        }
        out.push(']');
    }
    out.push('}');
}

fn parse_schema(json: &Json) -> Result<SchemaDescriptor, TheoryFile> {
    let relations = arr(json, "relations")?
        .iter()
        .map(parse_relation)
        .collect::<Result<_, _>>()?;
    let statements = arr(json, "statements")?
        .iter()
        .map(parse_statement)
        .collect::<Result<_, _>>()?;
    Ok(SchemaDescriptor {
        relations,
        statements,
    })
}

fn parse_relation(json: &Json) -> Result<RelationDescriptor, TheoryFile> {
    let fields = arr(json, "fields")?
        .iter()
        .map(|field| {
            if field.get("generation").is_some() {
                // Fresh issuance is deleted with the successor; silently
                // ignoring the old spelling would silently change meaning.
                return Err(TheoryFile::Shape("fresh generation is deleted"));
            }
            Ok(FieldDescriptor {
                name: text(field, "name")?.into(),
                value_type: parse_type(&field["type"])?,
            })
        })
        .collect::<Result<_, _>>()?;
    let extension = match json.get("extension") {
        None | Some(Json::Null) => None,
        Some(rows) => Some(
            rows.as_array()
                .ok_or(TheoryFile::Shape("extension rows"))?
                .iter()
                .map(parse_row)
                .collect::<Result<_, _>>()?,
        ),
    };
    Ok(RelationDescriptor {
        name: text(json, "name")?.into(),
        fields,
        extension,
    })
}

fn parse_row(json: &Json) -> Result<Row, TheoryFile> {
    let values = arr(json, "values")?
        .iter()
        .map(|value| parse_value(value).map_err(TheoryFile::Shape))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Row {
        handle: text(json, "handle")?.into(),
        values: values.into_boxed_slice(),
    })
}

fn parse_type(json: &Json) -> Result<ValueType, TheoryFile> {
    if let Some(name) = json.as_str() {
        return match name {
            "bool" => Ok(ValueType::Bool),
            "u64" => Ok(ValueType::U64),
            "i64" => Ok(ValueType::I64),
            "f64" => Ok(ValueType::F64),
            "string" => Ok(ValueType::String),
            "id128" => Ok(ValueType::Id128),
            _ => Err(TheoryFile::Shape("unknown scalar type")),
        };
    }
    let object = json.as_object().ok_or(TheoryFile::Shape("type object"))?;
    if let Some(len) = object.get("fixedBytes") {
        let len = u16::try_from(as_u64(len, "fixedBytes len")?)
            .map_err(|_| TheoryFile::Shape("fixedBytes len"))?;
        return Ok(ValueType::FixedBytes { len });
    }
    if let Some(element) = object.get("interval") {
        return Ok(ValueType::Interval {
            element: parse_element(element)?,
        });
    }
    if let Some(fixed) = object.get("fixedInterval") {
        return Ok(ValueType::FixedInterval {
            element: parse_fixed_element(&fixed["element"])?,
            width: parse_u64(&fixed["width"]).map_err(TheoryFile::Shape)?,
        });
    }
    Err(TheoryFile::Shape("unknown type"))
}

fn parse_element(json: &Json) -> Result<IntervalElement, TheoryFile> {
    match json.as_str() {
        Some("u64") => Ok(IntervalElement::U64),
        Some("i64") => Ok(IntervalElement::I64),
        Some("f64") => Ok(IntervalElement::F64),
        _ => Err(TheoryFile::Shape("interval element")),
    }
}

fn parse_fixed_element(json: &Json) -> Result<FixedIntervalElement, TheoryFile> {
    match json.as_str() {
        Some("u64") => Ok(FixedIntervalElement::U64),
        Some("i64") => Ok(FixedIntervalElement::I64),
        // `fixedInterval<f64>` is unrepresentable by the descriptor family;
        // this grammar refuses the spelling rather than inventing a type.
        _ => Err(TheoryFile::Shape("fixed interval element")),
    }
}

fn parse_statement(json: &Json) -> Result<StatementDescriptor, TheoryFile> {
    let object = json.as_object().ok_or(TheoryFile::Shape("statement"))?;
    if let Some(body) = object.get("functionality") {
        return Ok(StatementDescriptor::Functionality {
            relation: RelationId(as_u32(&body["relation"], "relation")?),
            projection: parse_projection(&body["projection"])?,
        });
    }
    if let Some(body) = object.get("containment") {
        return Ok(StatementDescriptor::Containment {
            source: parse_side(&body["source"])?,
            target: parse_side(&body["target"])?,
        });
    }
    if let Some(body) = object.get("capacity") {
        return Ok(StatementDescriptor::Capacity {
            target: parse_side(&body["target"])?,
            weight: parse_weight(&body["weight"])?,
            lo: parse_u64(&body["lo"]).map_err(TheoryFile::Shape)?,
            hi: parse_bound(&body["hi"])?,
            source: parse_side(&body["source"])?,
        });
    }
    Err(TheoryFile::Shape("unknown statement"))
}

fn parse_projection(json: &Json) -> Result<Box<[FieldId]>, TheoryFile> {
    json.as_array()
        .ok_or(TheoryFile::Shape("projection"))?
        .iter()
        .map(|field| Ok(FieldId(as_u16(field, "field")?)))
        .collect()
}

fn parse_side(json: &Json) -> Result<Side, TheoryFile> {
    let selection = match json.get("selection") {
        None | Some(Json::Null) => Box::from([]),
        Some(bindings) => bindings
            .as_array()
            .ok_or(TheoryFile::Shape("selection"))?
            .iter()
            .map(|binding| {
                let pair = pair2(binding).map_err(TheoryFile::Shape)?;
                let field = FieldId(as_u16(&pair[0], "field")?);
                let literals: Vec<Value> = pair[1]
                    .as_array()
                    .ok_or(TheoryFile::Shape("literals"))?
                    .iter()
                    .map(|value| parse_value(value).map_err(TheoryFile::Shape))
                    .collect::<Result<_, _>>()?;
                let set = if literals.len() == 1 {
                    LiteralSet::One(literals.into_iter().next().expect("one literal"))
                } else {
                    LiteralSet::Many(literals.into_boxed_slice())
                };
                Ok((field, set))
            })
            .collect::<Result<_, _>>()?,
    };
    Ok(Side {
        relation: RelationId(as_u32(&json["relation"], "relation")?),
        projection: parse_projection(&json["projection"])?,
        selection,
    })
}

fn parse_weight(json: &Json) -> Result<Weight, TheoryFile> {
    if json.as_str() == Some("unit") {
        return Ok(Weight::Unit);
    }
    let object = json.as_object().ok_or(TheoryFile::Shape("weight"))?;
    if let Some(field) = object.get("field") {
        return Ok(Weight::Field(FieldId(as_u16(field, "field")?)));
    }
    if let Some(field) = object.get("durationOf") {
        return Ok(Weight::DurationOf(FieldId(as_u16(field, "field")?)));
    }
    Err(TheoryFile::Shape("unknown weight"))
}

fn parse_bound(json: &Json) -> Result<Option<Bound>, TheoryFile> {
    if json.is_null() {
        return Ok(None);
    }
    let object = json.as_object().ok_or(TheoryFile::Shape("bound"))?;
    if let Some(lit) = object.get("lit") {
        return Ok(Some(Bound::Lit(parse_u64(lit).map_err(TheoryFile::Shape)?)));
    }
    if let Some(field) = object.get("targetField") {
        return Ok(Some(Bound::TargetField(FieldId(as_u16(field, "field")?))));
    }
    if let Some(field) = object.get("targetDuration") {
        return Ok(Some(Bound::TargetDuration(FieldId(as_u16(
            field, "field",
        )?))));
    }
    Err(TheoryFile::Shape("unknown bound"))
}

fn arr<'a>(json: &'a Json, field: &'static str) -> Result<&'a Vec<Json>, TheoryFile> {
    json.get(field)
        .and_then(Json::as_array)
        .ok_or(TheoryFile::Shape(field))
}

fn text<'a>(json: &'a Json, field: &'static str) -> Result<&'a str, TheoryFile> {
    json.get(field)
        .and_then(Json::as_str)
        .ok_or(TheoryFile::Shape(field))
}

fn as_u64(json: &Json, field: &'static str) -> Result<u64, TheoryFile> {
    json.as_u64().ok_or(TheoryFile::Shape(field))
}

fn as_u32(json: &Json, field: &'static str) -> Result<u32, TheoryFile> {
    u32::try_from(as_u64(json, field)?).map_err(|_| TheoryFile::Shape(field))
}

fn as_u16(json: &Json, field: &'static str) -> Result<u16, TheoryFile> {
    u16::try_from(as_u64(json, field)?).map_err(|_| TheoryFile::Shape(field))
}

/// A stable hex spelling for schema identities in generated artifacts.
#[must_use]
pub fn fingerprint_hex(fingerprint: &SchemaFingerprint) -> String {
    let mut out = String::with_capacity(64);
    push_hex(&mut out, &fingerprint.0);
    out
}

#[cfg(test)]
mod tests {
    use super::{TheoryFile, parse, render, schema_id};

    #[test]
    fn a_multi_arm_value_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[],"extension":[{"handle":"h","values":[{"bool":true,"u64":"1"}]}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape("value arm"))));
    }

    #[test]
    fn unhex_refuses_a_mid_char_slice_without_panic() {
        let raw = r#"{"relations":[{"name":"n","fields":[],"extension":[{"handle":"h","values":[{"fixedBytes":"€a"}]}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_short_binding_pair_is_shape() {
        let raw = r#"{"relations":[{"name":"a","fields":[{"name":"id","type":"u64"}]},{"name":"b","fields":[{"name":"id","type":"u64"}]}],"statements":[{"containment":{"source":{"relation":0,"projection":[0],"selection":[[0]]},"target":{"relation":1,"projection":[0]}}}]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn note_theory_is_a_descriptor() {
        let raw = r#"{"relations":[{"name":"note","fields":[{"name":"id","type":"id128"},{"name":"body","type":"string"}]}],"statements":[{"functionality":{"relation":0,"projection":[0]}}]}"#;
        let schema = parse(raw).expect("note theory");
        assert_eq!(schema.relations.len(), 1);
        assert_eq!(schema.statements.len(), 1);
    }

    #[test]
    fn the_fresh_generation_spelling_refuses_instead_of_being_ignored() {
        let raw = r#"{"relations":[{"name":"n","fields":[{"name":"id","type":"u64","generation":"fresh"}]}],"statements":[]}"#;
        assert!(matches!(
            parse(raw),
            Err(TheoryFile::Shape("fresh generation is deleted"))
        ));
    }

    #[test]
    fn successor_types_parse_and_fixed_f64_refuses() {
        let raw = r#"{"relations":[{"name":"n","fields":[{"name":"a","type":"id128"},{"name":"b","type":{"interval":"f64"}},{"name":"c","type":{"fixedInterval":{"element":"i64","width":"4"}}}]}],"statements":[]}"#;
        let schema = parse(raw).expect("successor types");
        assert_eq!(schema.relations[0].fields.len(), 3);
        let bad = r#"{"relations":[{"name":"n","fields":[{"name":"c","type":{"fixedInterval":{"element":"f64","width":"4"}}}]}],"statements":[]}"#;
        assert!(matches!(
            parse(bad),
            Err(TheoryFile::Shape("fixed interval element"))
        ));
    }

    #[test]
    fn render_is_a_deterministic_left_inverse_of_parse() {
        let raw = r#"{"relations":[{"name":"kind","fields":[{"name":"label","type":"string"}],"extension":[{"handle":"a","values":[{"string":"alpha"}]},{"handle":"b","values":[{"string":"beta"}]}]},{"name":"note","fields":[{"name":"id","type":"id128"},{"name":"kind","type":"u64"},{"name":"score","type":"f64"},{"name":"span","type":{"interval":"u64"}}]}],"statements":[{"functionality":{"relation":1,"projection":[0]}},{"containment":{"source":{"relation":1,"projection":[1]},"target":{"relation":0,"projection":[0]}}},{"capacity":{"target":{"relation":0,"projection":[0]},"weight":"unit","lo":"0","hi":{"lit":"5"},"source":{"relation":1,"projection":[1],"selection":[[2,[{"u64":"1"},{"u64":"2"}]]]}}}]}"#;
        let descriptor = parse(raw).expect("kitchen descriptor");
        let text = render(&descriptor);
        let reparsed = parse(&text).expect("rendered text parses");
        assert_eq!(reparsed, descriptor);
        assert_eq!(render(&reparsed), text, "byte-stable");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn schema_id_is_the_core_fingerprint_and_validation_refuses_junk() {
        let raw = r#"{"relations":[{"name":"note","fields":[{"name":"id","type":"u64"},{"name":"body","type":"string"}]}],"statements":[{"functionality":{"relation":0,"projection":[0]}}]}"#;
        let descriptor = parse(raw).expect("note theory");
        let id = schema_id(&descriptor).expect("valid schema");
        // Determinism through render/parse: identity never depends on text.
        let again = schema_id(&parse(&render(&descriptor)).unwrap()).unwrap();
        assert_eq!(id, again);
        // A statement citing a missing relation refuses with the core error.
        let bad =
            r#"{"relations":[],"statements":[{"functionality":{"relation":7,"projection":[0]}}]}"#;
        let bad = parse(bad).expect("well-formed shape");
        assert!(schema_id(&bad).is_err());
    }
}
