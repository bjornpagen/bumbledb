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
                value_type: parse_type(req(field, "type")?)?,
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
    let (kind, body) = arm(json, "type object")?;
    match kind {
        "fixedBytes" => {
            let len = u16::try_from(as_u64(body, "fixedBytes len")?)
                .map_err(|_| TheoryFile::Shape("fixedBytes len"))?;
            Ok(ValueType::FixedBytes { len })
        }
        "interval" => Ok(ValueType::Interval {
            element: parse_element(body)?,
        }),
        "fixedInterval" => Ok(ValueType::FixedInterval {
            element: parse_element(req(body, "element")?)?,
            width: parse_u64(req(body, "width")?)?,
        }),
        _ => Err(TheoryFile::Shape("unknown type")),
    }
}

fn parse_element(json: &Json) -> Result<IntervalElement, TheoryFile> {
    match json.as_str() {
        Some("u64") => Ok(IntervalElement::U64),
        Some("i64") => Ok(IntervalElement::I64),
        _ => Err(TheoryFile::Shape("interval element")),
    }
}

fn parse_statement(json: &Json) -> Result<StatementDescriptor, TheoryFile> {
    let (kind, body) = arm(json, "statement")?;
    match kind {
        "functionality" => Ok(StatementDescriptor::Functionality {
            relation: RelationId(as_u32(req(body, "relation")?, "relation")?),
            projection: parse_projection(req(body, "projection")?)?,
        }),
        "containment" => Ok(StatementDescriptor::Containment {
            source: parse_side(req(body, "source")?)?,
            target: parse_side(req(body, "target")?)?,
        }),
        "capacity" => Ok(StatementDescriptor::Capacity {
            target: parse_side(req(body, "target")?)?,
            weight: parse_weight(req(body, "weight")?)?,
            lo: parse_u64(req(body, "lo")?)?,
            hi: parse_bound(req(body, "hi")?)?,
            source: parse_side(req(body, "source")?)?,
        }),
        _ => Err(TheoryFile::Shape("unknown statement")),
    }
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
        relation: RelationId(as_u32(req(json, "relation")?, "relation")?),
        projection: parse_projection(req(json, "projection")?)?,
        selection,
    })
}

fn parse_weight(json: &Json) -> Result<Weight, TheoryFile> {
    if json.as_str() == Some("unit") {
        return Ok(Weight::Unit);
    }
    let (kind, body) = arm(json, "weight")?;
    match kind {
        "field" => Ok(Weight::Field(FieldId(as_u16(body, "field")?))),
        "durationOf" => Ok(Weight::DurationOf(FieldId(as_u16(body, "field")?))),
        _ => Err(TheoryFile::Shape("unknown weight")),
    }
}

fn parse_bound(json: &Json) -> Result<Option<Bound>, TheoryFile> {
    if json.is_null() {
        return Ok(None);
    }
    let (kind, body) = arm(json, "bound")?;
    match kind {
        "lit" => Ok(Some(Bound::Lit(parse_u64(body)?))),
        "targetField" => Ok(Some(Bound::TargetField(FieldId(as_u16(body, "field")?)))),
        "targetDuration" => Ok(Some(Bound::TargetDuration(FieldId(as_u16(body, "field")?)))),
        _ => Err(TheoryFile::Shape("unknown bound")),
    }
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

fn arm<'a>(json: &'a Json, what: &'static str) -> Result<(&'a str, &'a Json), TheoryFile> {
    let object = json.as_object().ok_or(TheoryFile::Shape(what))?;
    if object.len() != 1 {
        return Err(TheoryFile::Shape(what));
    }
    let (kind, body) = object.iter().next().ok_or(TheoryFile::Shape(what))?;
    Ok((kind.as_str(), body))
}

fn req<'a>(json: &'a Json, field: &'static str) -> Result<&'a Json, TheoryFile> {
    json.get(field).ok_or(TheoryFile::Shape(field))
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
    use super::{parse, TheoryFile};

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
    fn a_missing_field_type_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[{"name":"id"}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_missing_functionality_relation_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[]}],"statements":[{"functionality":{"projection":[]}}]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_missing_side_relation_is_shape() {
        let raw = r#"{"relations":[{"name":"a","fields":[]},{"name":"b","fields":[]}],"statements":[{"containment":{"source":{"projection":[]},"target":{"relation":1,"projection":[]}}}]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_missing_fixed_interval_element_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[{"name":"w","type":{"fixedInterval":{"width":"1"}}}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_multi_arm_statement_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[]}],"statements":[{"functionality":{"relation":0,"projection":[]},"containment":{"source":{"relation":0,"projection":[]},"target":{"relation":0,"projection":[]}}}]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_multi_arm_type_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[{"name":"x","type":{"fixedBytes":4,"interval":"u64"}}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape(_))));
    }

    #[test]
    fn a_short_interval_pair_is_shape() {
        let raw = r#"{"relations":[{"name":"n","fields":[],"extension":[{"handle":"h","values":[{"intervalU64":["1"]"}]}]}],"statements":[]}"#;
        assert!(matches!(parse(raw), Err(TheoryFile::Shape("pair"))));
    }

    #[test]
    fn a_corpus_schema_object_parses() {
        let raw = r#"{"relations":[{"name":"note","fields":[{"name":"id","type":"u64"},{"name":"body","type":"string"}]}],"statements":[{"functionality":{"relation":0,"projection":[0]}}]}"#;
        let schema = parse(raw).expect("valid theory");
        assert_eq!(schema.relations.len(), 1);
        assert_eq!(schema.statements.len(), 1);
    }
}
