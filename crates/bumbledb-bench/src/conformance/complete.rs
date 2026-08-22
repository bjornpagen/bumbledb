//! Each document is written only after Format in `lean/conformance/README.md` §
//! complete-admission cases.

use bumbledb::Value;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};

use crate::differential::{self, Verdict};
use crate::naive::NaiveDb;

use super::judgment::{lane_verdict, push_blocks, push_relations, push_statements, push_verdict};

type Facts = Vec<(RelationId, Vec<Value>)>;

struct CompleteFixture {
    name: &'static str,
    schema: SchemaDescriptor,
    facts: Facts,
}

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn u64_relation(name: &str, fields: &[&str]) -> RelationDescriptor {
    RelationDescriptor {
        extension: None,
        name: name.into(),
        fields: fields.iter().map(|f| field(f, ValueType::U64)).collect(),
    }
}

fn closed(name: &str, handles: &[&str]) -> RelationDescriptor {
    RelationDescriptor {
        extension: Some(
            handles
                .iter()
                .map(|handle| Row {
                    handle: (*handle).into(),
                    values: Box::new([]),
                })
                .collect(),
        ),
        name: name.into(),
        fields: vec![],
    }
}

fn side(relation: RelationId, projection: &[u16]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    }
}

fn must_validate(schema: SchemaDescriptor, name: &str) -> SchemaDescriptor {
    schema
        .clone()
        .validate()
        .unwrap_or_else(|err| panic!("complete fixture {name}: schema refused: {err}"));
    schema
}

fn closed_source_schema() -> SchemaDescriptor {
    must_validate(
        SchemaDescriptor {
            relations: vec![
                closed("Severity", &["Low", "High"]),
                u64_relation("Handler", &["severity", "who"]),
                u64_relation("Note", &["id"]),
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Containment {
                    source: side(RelationId(0), &[0]),
                    target: side(RelationId(1), &[0]),
                },
            ],
        },
        "closed-source",
    )
}

fn closed_capacity_schema() -> SchemaDescriptor {
    must_validate(
        SchemaDescriptor {
            relations: vec![
                closed("Kind", &["Soft", "Hard"]),
                u64_relation("Bucket", &["kind"]),
            ],
            statements: vec![StatementDescriptor::Capacity {
                target: side(RelationId(0), &[0]),
                weight: Weight::Unit,
                lo: 1,
                hi: Some(Bound::Lit(2)),
                source: side(RelationId(1), &[0]),
            }],
        },
        "closed-capacity",
    )
}

fn ordinary_source_schema() -> SchemaDescriptor {
    must_validate(
        SchemaDescriptor {
            relations: vec![
                u64_relation("Holder", &["id", "tag"]),
                u64_relation("Account", &["holder", "kind", "num"]),
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Containment {
                    source: side(RelationId(1), &[0]),
                    target: side(RelationId(0), &[0]),
                },
            ],
        },
        "ordinary-source",
    )
}

fn ordinary_capacity_schema() -> SchemaDescriptor {
    must_validate(
        SchemaDescriptor {
            relations: vec![
                u64_relation("Holder", &["id", "tag"]),
                u64_relation("Account", &["holder", "kind", "num"]),
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Capacity {
                    target: side(RelationId(0), &[0]),
                    weight: Weight::Unit,
                    lo: 1,
                    hi: Some(Bound::Lit(2)),
                    source: side(RelationId(1), &[0]),
                },
            ],
        },
        "ordinary-capacity",
    )
}

fn key_only_schema() -> SchemaDescriptor {
    must_validate(
        SchemaDescriptor {
            relations: vec![u64_relation("Holder", &["id", "tag"])],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            }],
        },
        "key-only",
    )
}

fn handler(severity: u64, who: u64) -> (RelationId, Vec<Value>) {
    (RelationId(1), vec![Value::U64(severity), Value::U64(who)])
}

fn note(id: u64) -> (RelationId, Vec<Value>) {
    (RelationId(2), vec![Value::U64(id)])
}

fn bucket(kind: u64) -> (RelationId, Vec<Value>) {
    (RelationId(1), vec![Value::U64(kind)])
}

fn holder(id: u64) -> (RelationId, Vec<Value>) {
    (RelationId(0), vec![Value::U64(id), Value::U64(0)])
}

fn holder_tagged(id: u64, tag: u64) -> (RelationId, Vec<Value>) {
    (RelationId(0), vec![Value::U64(id), Value::U64(tag)])
}

fn account(holder: u64, kind: u64, num: u64) -> (RelationId, Vec<Value>) {
    (
        RelationId(1),
        vec![Value::U64(holder), Value::U64(kind), Value::U64(num)],
    )
}

fn fixtures() -> Vec<CompleteFixture> {
    vec![
        CompleteFixture {
            name: "complete-empty-green",
            schema: key_only_schema(),
            facts: vec![],
        },
        CompleteFixture {
            name: "complete-closed-source-missing-target",
            schema: closed_source_schema(),
            facts: vec![],
        },
        CompleteFixture {
            name: "complete-closed-source-satisfied",
            schema: closed_source_schema(),
            facts: vec![handler(0, 1), handler(1, 2)],
        },
        CompleteFixture {
            name: "complete-closed-source-unrelated-note",
            schema: closed_source_schema(),
            facts: vec![note(1)],
        },
        CompleteFixture {
            name: "complete-closed-capacity-childless",
            schema: closed_capacity_schema(),
            facts: vec![],
        },
        CompleteFixture {
            name: "complete-closed-capacity-satisfied",
            schema: closed_capacity_schema(),
            facts: vec![bucket(0), bucket(1)],
        },
        CompleteFixture {
            name: "complete-ordinary-source-missing-target",
            schema: ordinary_source_schema(),
            facts: vec![account(7, 1, 0)],
        },
        CompleteFixture {
            name: "complete-ordinary-capacity-childless",
            schema: ordinary_capacity_schema(),
            facts: vec![holder(8)],
        },
        CompleteFixture {
            name: "complete-key-collision",
            schema: key_only_schema(),
            facts: vec![holder_tagged(9, 0), holder_tagged(9, 1)],
        },
    ]
}

fn render_fixture(fixture: &CompleteFixture) -> String {
    let engine = differential::engine_admit(fixture.schema.clone(), &fixture.facts);
    let mut naive = NaiveDb::new(&fixture.schema);
    naive.load_candidate(&fixture.facts);
    let violations = naive.judge_complete();
    let model = if violations.is_empty() {
        Verdict::Committed
    } else {
        Verdict::Aborted(violations)
    };
    assert_eq!(
        engine, model,
        "TROPHY (engine vs naive) on complete-admission case {}: triage per the fuzzing charter",
        fixture.name
    );
    let verdict = lane_verdict(fixture.name, &engine);

    let axioms: Vec<(RelationId, Vec<Vec<Value>>)> = fixture
        .schema
        .relations
        .iter()
        .enumerate()
        .filter_map(|(index, relation)| {
            let extension = relation.extension.as_ref()?;
            let relation = RelationId(u32::try_from(index).expect("relation count fits u32"));
            Some((
                relation,
                extension
                    .iter()
                    .enumerate()
                    .map(|(row, axiom)| {
                        let mut fact = vec![Value::U64(
                            u64::try_from(row).expect("extension rows fit u64"),
                        )];
                        fact.extend(axiom.values.iter().cloned());
                        fact
                    })
                    .collect(),
            ))
        })
        .collect();
    let instance: Vec<(RelationId, Vec<Vec<Value>>)> = fixture
        .schema
        .relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.extension.is_none())
        .map(|(index, _)| {
            let relation = RelationId(u32::try_from(index).expect("relation count fits u32"));
            (
                relation,
                fixture
                    .facts
                    .iter()
                    .filter(|(r, _)| *r == relation)
                    .map(|(_, fact)| fact.clone())
                    .collect(),
            )
        })
        .collect();

    let mut relations_block = String::new();
    push_relations(&mut relations_block, &fixture.schema);
    let mut statements_block = String::new();
    push_statements(&mut statements_block, &fixture.schema);
    let mut axioms_block = String::new();
    push_blocks(&mut axioms_block, &axioms, &fixture.schema, true);
    let mut instance_block = String::new();
    push_blocks(&mut instance_block, &instance, &fixture.schema, false);
    let mut verdict_block = String::new();
    push_verdict(&mut verdict_block, &verdict);

    format!(
        "{{\n\"case\":\"{name}\",\n\"kind\":\"complete\",\n\
         \"provenance\":{{\"hand\":\"{name}\"}},\n\
         \"theory\":{{\"relations\":{relations_block},\n\
         \"ground_axioms\":{axioms_block},\n\
         \"statements\":{statements_block}}},\n\
         \"instance\":{instance_block},\n\
         \"delta\":{{\"deletes\":[],\n\"inserts\":[]}},\n\
         \"verdict\":{verdict_block}\n}}\n",
        name = fixture.name
    )
}

#[must_use]
pub fn generate_complete_corpus() -> Vec<(String, String)> {
    fixtures()
        .iter()
        .map(|fixture| (format!("{}.json", fixture.name), render_fixture(fixture)))
        .collect()
}

/// # Panics
#[must_use]
pub fn replay_complete_case(name: &str) -> String {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture.name == name)
        .unwrap_or_else(|| panic!("unknown complete-admission fixture {name}: stale corpus"));
    render_fixture(&fixture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_admission_engine_admit_agrees_with_naive() {
        for fixture in fixtures() {
            let _ = render_fixture(&fixture);
        }
    }

    #[test]
    fn complete_admission_includes_closed_source_containments() {
        let document = replay_complete_case("complete-closed-source-missing-target");
        assert!(
            document.contains("\"kind\":\"complete\""),
            "the complete-admission kind mark"
        );
        assert!(document.contains("\"phase\":\"statement\""), "{document}");
        assert!(
            document.contains("\"violations\":[2]"),
            "materialized: closed auto-key, Handler key, containment — {document}"
        );
        let satisfied = replay_complete_case("complete-closed-source-satisfied");
        assert!(satisfied.contains("\"verdict\":\"accept\""), "{satisfied}");
    }

    /// Unrelated ordinary facts do not hide a closed-source miss — the
    #[test]
    fn unrelated_ordinary_facts_do_not_discharge_closed_source() {
        let document = replay_complete_case("complete-closed-source-unrelated-note");
        assert!(document.contains("\"phase\":\"statement\""), "{document}");
        assert!(document.contains("\"violations\":[2]"), "{document}");
    }
}
