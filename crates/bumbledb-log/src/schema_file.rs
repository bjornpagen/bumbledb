//! Parse a theory file at the duty boundary. The spelling is the
//! crate's corpus schema object — `{relations, statements}` — so a
//! second descriptor grammar cannot exist.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet, RelationDescriptor,
    RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight,
};
use bumbledb::{Interval, Value};
use serde_json::Value as Json;

/// Why a theory file refused to become a descriptor.
#[derive(Debug)]
pub enum TheoryFile {
    Io(io::Error),
    Json(serde_json::Error),
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
pub fn load(path: &Path) -> Result<SchemaDescriptor, TheoryFile> {
    let raw = fs::read_to_string(path).map_err(TheoryFile::Io)?;
    parse(&raw)
}

/// Parse the theory-file grammar from bytes already in memory.
pub fn parse(raw: &str) -> Result<SchemaDescriptor, TheoryFile> {
    let json: Json = serde_json::from_str(raw).map_err(TheoryFile::Json)?;
    parse_schema(&json)
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
            Ok(FieldDescriptor {
                name: text(field, "name")?.into(),
                value_type: parse_type(&field["type"])?,
                generation: match field.get("generation").and_then(Json::as_str) {
                    Some("fresh") => Generation::Fresh,
                    Some(_) => return Err(TheoryFile::Shape("unknown generation")),
                    None => Generation::None,
                },
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
        .map(parse_value)
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
            "string" => Ok(ValueType::String),
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
            element: parse_element(&fixed["element"])?,
            width: parse_u64(&fixed["width"])?,
        });
    }
    Err(TheoryFile::Shape("unknown type"))
}

fn parse_element(json: &Json) -> Result<IntervalElement, TheoryFile> {
    match json.as_str() {
        Some("u64") => Ok(IntervalElement::U64),
        Some("i64") => Ok(IntervalElement::I64),
        _ => Err(TheoryFile::Shape("interval element")),
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
            lo: parse_u64(&body["lo"])?,
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
                let pair = pair2(binding)?;
                let field = FieldId(as_u16(&pair[0], "field")?);
                let literals: Vec<Value> = pair[1]
                    .as_array()
                    .ok_or(TheoryFile::Shape("literals"))?
                    .iter()
                    .map(parse_value)
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
        return Ok(Some(Bound::Lit(parse_u64(lit)?)));
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

fn parse_value(json: &Json) -> Result<Value, TheoryFile> {
    let object = json.as_object().ok_or(TheoryFile::Shape("value"))?;
    if object.len() != 1 {
        return Err(TheoryFile::Shape("value arm"));
    }
    let (kind, body) = object.iter().next().ok_or(TheoryFile::Shape("value arm"))?;
    match kind.as_str() {
        "bool" => Ok(Value::Bool(
            body.as_bool().ok_or(TheoryFile::Shape("bool"))?,
        )),
        "u64" => Ok(Value::U64(parse_u64(body)?)),
        "i64" => Ok(Value::I64(parse_i64(body)?)),
        "string" => Ok(Value::String(
            body.as_str().ok_or(TheoryFile::Shape("string"))?.into(),
        )),
        "fixedBytes" => Ok(Value::FixedBytes(
            unhex(body.as_str().ok_or(TheoryFile::Shape("hex"))?)?.into_boxed_slice(),
        )),
        "intervalU64" => {
            let pair = pair2(body)?;
            Interval::new(parse_u64(&pair[0])?, parse_u64(&pair[1])?)
                .map(Value::IntervalU64)
                .ok_or(TheoryFile::Shape("interval"))
        }
        "intervalI64" => {
            let pair = pair2(body)?;
            Interval::new(parse_i64(&pair[0])?, parse_i64(&pair[1])?)
                .map(Value::IntervalI64)
                .ok_or(TheoryFile::Shape("interval"))
        }
        _ => Err(TheoryFile::Shape("unknown value arm")),
    }
}

fn parse_u64(json: &Json) -> Result<u64, TheoryFile> {
    json.as_str()
        .ok_or(TheoryFile::Shape("decimal string"))?
        .parse()
        .map_err(|_| TheoryFile::Shape("u64"))
}

fn parse_i64(json: &Json) -> Result<i64, TheoryFile> {
    json.as_str()
        .ok_or(TheoryFile::Shape("decimal string"))?
        .parse()
        .map_err(|_| TheoryFile::Shape("i64"))
}

fn unhex(text: &str) -> Result<Vec<u8>, TheoryFile> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(TheoryFile::Shape("even hex length"));
    }
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hi = hex_nibble(pair[0])?;
            let lo = hex_nibble(pair[1])?;
            Ok((hi << 4) | lo)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Result<u8, TheoryFile> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(TheoryFile::Shape("hex byte")),
    }
}

fn pair2(json: &Json) -> Result<&[Json], TheoryFile> {
    let pair = json.as_array().ok_or(TheoryFile::Shape("pair"))?;
    if pair.len() == 2 {
        Ok(pair)
    } else {
        Err(TheoryFile::Shape("pair"))
    }
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

#[cfg(test)]
mod tests {
    use super::{TheoryFile, parse};

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
}
