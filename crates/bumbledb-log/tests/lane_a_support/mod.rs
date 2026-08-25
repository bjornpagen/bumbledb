//! Shared fixtures for the lane's conformance tests: the corpus schema
//! loader (`conformance/v3/schemas.json` is the one cross-language
//! source of the fixture descriptors), the JSON value vocabulary the
//! sidecars speak, and the renderers that turn decoded batches back
//! into sidecar JSON for byte-exact comparison.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet, RelationDescriptor,
    RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight,
};
use bumbledb::{Interval, Value};
use bumbledb_log::codec::{Batch, BatchHeader, Op, OpKind};
use serde_json::Value as Json;

pub fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("v3")
}

pub fn bless() -> bool {
    std::env::var_os("BUMBLEDB_LOG_BLESS").is_some()
}

/// The corpus fingerprints are synthetic: the goldens pin the codec,
/// not the engine's schema hash, so the two cannot drift each other.
pub fn corpus_fingerprint(schema: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bumbledb-log corpus fingerprint: ");
    hasher.update(schema.as_bytes());
    *hasher.finalize().as_bytes()
}

pub fn load_schemas() -> BTreeMap<String, SchemaDescriptor> {
    let raw = std::fs::read_to_string(corpus_dir().join("schemas.json")).expect("schemas.json");
    let json: Json = serde_json::from_str(&raw).expect("schemas.json parses");
    json["schemas"]
        .as_object()
        .expect("schemas map")
        .iter()
        .map(|(name, schema)| (name.clone(), parse_schema(schema)))
        .collect()
}

pub fn schema(name: &str) -> SchemaDescriptor {
    load_schemas().remove(name).expect("named fixture schema")
}

fn parse_schema(json: &Json) -> SchemaDescriptor {
    let relations = json["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .map(parse_relation)
        .collect();
    let statements = json["statements"]
        .as_array()
        .expect("statements")
        .iter()
        .map(parse_statement)
        .collect();
    SchemaDescriptor {
        relations,
        statements,
    }
}

fn parse_relation(json: &Json) -> RelationDescriptor {
    let fields = json["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| FieldDescriptor {
            name: field["name"].as_str().expect("field name").into(),
            value_type: parse_type(&field["type"]),
            generation: match field.get("generation").and_then(Json::as_str) {
                Some("fresh") => Generation::Fresh,
                Some(other) => panic!("unknown generation {other}"),
                None => Generation::None,
            },
        })
        .collect();
    let extension = json.get("extension").map(|rows| {
        rows.as_array()
            .expect("extension rows")
            .iter()
            .map(|row| Row {
                handle: row["handle"].as_str().expect("handle").into(),
                values: row["values"]
                    .as_array()
                    .expect("row values")
                    .iter()
                    .map(parse_value)
                    .collect(),
            })
            .collect()
    });
    RelationDescriptor {
        name: json["name"].as_str().expect("relation name").into(),
        fields,
        extension,
    }
}

fn parse_type(json: &Json) -> ValueType {
    if let Some(name) = json.as_str() {
        return match name {
            "bool" => ValueType::Bool,
            "u64" => ValueType::U64,
            "i64" => ValueType::I64,
            "string" => ValueType::String,
            other => panic!("unknown scalar type {other}"),
        };
    }
    let object = json.as_object().expect("type object");
    if let Some(len) = object.get("fixedBytes") {
        return ValueType::FixedBytes {
            len: u16::try_from(len.as_u64().expect("len")).expect("len fits u16"),
        };
    }
    if let Some(element) = object.get("interval") {
        return ValueType::Interval {
            element: parse_element(element),
        };
    }
    if let Some(fixed) = object.get("fixedInterval") {
        return ValueType::FixedInterval {
            element: parse_element(&fixed["element"]),
            width: parse_u64(&fixed["width"]),
        };
    }
    panic!("unknown type {json}");
}

fn parse_element(json: &Json) -> IntervalElement {
    match json.as_str().expect("element") {
        "u64" => IntervalElement::U64,
        "i64" => IntervalElement::I64,
        other => panic!("unknown element {other}"),
    }
}

fn parse_statement(json: &Json) -> StatementDescriptor {
    let object = json.as_object().expect("statement object");
    if let Some(body) = object.get("functionality") {
        return StatementDescriptor::Functionality {
            relation: RelationId(
                u32::try_from(body["relation"].as_u64().expect("relation")).unwrap(),
            ),
            projection: parse_projection(&body["projection"]),
        };
    }
    if let Some(body) = object.get("containment") {
        return StatementDescriptor::Containment {
            source: parse_side(&body["source"]),
            target: parse_side(&body["target"]),
        };
    }
    if let Some(body) = object.get("capacity") {
        return StatementDescriptor::Capacity {
            target: parse_side(&body["target"]),
            weight: parse_weight(&body["weight"]),
            lo: parse_u64(&body["lo"]),
            hi: parse_bound(&body["hi"]),
            source: parse_side(&body["source"]),
        };
    }
    panic!("unknown statement {json}");
}

fn parse_projection(json: &Json) -> Box<[FieldId]> {
    json.as_array()
        .expect("projection")
        .iter()
        .map(|field| FieldId(u16::try_from(field.as_u64().expect("field")).unwrap()))
        .collect()
}

fn parse_side(json: &Json) -> Side {
    let selection = json
        .get("selection")
        .map(|bindings| {
            bindings
                .as_array()
                .expect("selection")
                .iter()
                .map(|binding| {
                    let pair = binding.as_array().expect("binding pair");
                    let field = FieldId(u16::try_from(pair[0].as_u64().expect("field")).unwrap());
                    let literals: Vec<Value> = pair[1]
                        .as_array()
                        .expect("literals")
                        .iter()
                        .map(parse_value)
                        .collect();
                    let set = if literals.len() == 1 {
                        LiteralSet::One(literals.into_iter().next().expect("one literal"))
                    } else {
                        LiteralSet::Many(literals.into_boxed_slice())
                    };
                    (field, set)
                })
                .collect()
        })
        .unwrap_or_default();
    Side {
        relation: RelationId(u32::try_from(json["relation"].as_u64().expect("relation")).unwrap()),
        projection: parse_projection(&json["projection"]),
        selection,
    }
}

fn parse_weight(json: &Json) -> Weight {
    if json.as_str() == Some("unit") {
        return Weight::Unit;
    }
    let object = json.as_object().expect("weight object");
    if let Some(field) = object.get("field") {
        return Weight::Field(FieldId(
            u16::try_from(field.as_u64().expect("field")).unwrap(),
        ));
    }
    if let Some(field) = object.get("durationOf") {
        return Weight::DurationOf(FieldId(
            u16::try_from(field.as_u64().expect("field")).unwrap(),
        ));
    }
    panic!("unknown weight {json}");
}

fn parse_bound(json: &Json) -> Option<Bound> {
    if json.is_null() {
        return None;
    }
    let object = json.as_object().expect("bound object");
    if let Some(lit) = object.get("lit") {
        return Some(Bound::Lit(parse_u64(lit)));
    }
    if let Some(field) = object.get("targetField") {
        return Some(Bound::TargetField(FieldId(
            u16::try_from(field.as_u64().expect("field")).unwrap(),
        )));
    }
    if let Some(field) = object.get("targetDuration") {
        return Some(Bound::TargetDuration(FieldId(
            u16::try_from(field.as_u64().expect("field")).unwrap(),
        )));
    }
    panic!("unknown bound {json}");
}

fn parse_u64(json: &Json) -> u64 {
    json.as_str().expect("decimal string").parse().expect("u64")
}

fn parse_i64(json: &Json) -> i64 {
    json.as_str().expect("decimal string").parse().expect("i64")
}

pub fn parse_value(json: &Json) -> Value {
    let object = json.as_object().expect("value object");
    let (kind, body) = object.iter().next().expect("one arm");
    match kind.as_str() {
        "bool" => Value::Bool(body.as_bool().expect("bool")),
        "u64" => Value::U64(parse_u64(body)),
        "i64" => Value::I64(parse_i64(body)),
        "string" => Value::String(body.as_str().expect("string").into()),
        "fixedBytes" => Value::FixedBytes(unhex(body.as_str().expect("hex")).into_boxed_slice()),
        "intervalU64" => {
            let pair = body.as_array().expect("bounds");
            Value::IntervalU64(
                Interval::new(parse_u64(&pair[0]), parse_u64(&pair[1])).expect("interval"),
            )
        }
        "intervalI64" => {
            let pair = body.as_array().expect("bounds");
            Value::IntervalI64(
                Interval::new(parse_i64(&pair[0]), parse_i64(&pair[1])).expect("interval"),
            )
        }
        other => panic!("unknown value arm {other}"),
    }
}

pub fn render_value(value: &Value) -> Json {
    match value {
        Value::Bool(b) => serde_json::json!({ "bool": b }),
        Value::U64(v) => serde_json::json!({ "u64": v.to_string() }),
        Value::I64(v) => serde_json::json!({ "i64": v.to_string() }),
        Value::String(s) => serde_json::json!({ "string": s }),
        Value::FixedBytes(raw) => serde_json::json!({ "fixedBytes": hex(raw) }),
        Value::IntervalU64(interval) => serde_json::json!({
            "intervalU64": [interval.start().to_string(), interval.end().to_string()]
        }),
        Value::IntervalI64(interval) => serde_json::json!({
            "intervalI64": [interval.start().to_string(), interval.end().to_string()]
        }),
    }
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("write to string");
        out
    })
}

pub fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "even hex length");
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).expect("hex byte"))
        .collect()
}

pub fn render_header(header: &BatchHeader) -> Json {
    serde_json::json!({
        "braid": header.braid.to_string(),
        "braidGen": header.braid_gen.to_string(),
        "prev": hex(&header.prev),
        "writer": header.writer.to_string(),
        "timestamp": header.timestamp.to_string(),
    })
}

pub fn render_ops(ops: &[Op]) -> Json {
    Json::Array(
        ops.iter()
            .map(|op| {
                serde_json::json!({
                    "kind": match op.kind {
                        OpKind::Insert => "insert",
                        OpKind::Delete => "delete",
                    },
                    "relation": op.relation.0,
                    "rows": op.rows.iter().map(|row| {
                        Json::Array(row.iter().map(render_value).collect())
                    }).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub fn render_batch(batch: &Batch) -> Json {
    serde_json::json!({
        "header": render_header(&batch.header),
        "ops": render_ops(&batch.ops),
    })
}
