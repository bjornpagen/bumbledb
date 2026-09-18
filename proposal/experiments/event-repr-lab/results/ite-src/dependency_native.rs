//! Denotational bridge, using the actual macro, validator and interval judge.
//! This does not add Event storage or admission to the production engine.
use super::dependencies::{Fact, cases};
use crate::schema::judge::{JudgeBudget, Judgment, MapState, judge_complete};
use crate::schema::{FieldId, RelationId, StatementDescriptor, ValidateDescriptor};
use crate::{Interval, Theory, Value, WorkContext};

crate::schema! {
    pub DependencyBridge;
    relation Source { group: u64, origin: u64, span: interval<u64> }
    relation Target { group: u64, origin: u64, span: interval<u64> }
    Source(group, span) -> Source;
    Target(group, span) -> Target;
    Source(group, span) == Target(group, span);
}

pub fn verify() {
    let schema = DependencyBridge
        .descriptor()
        .validate()
        .expect("native pointwise schema");
    // A scalar key is not the explicitly projected pointwise target key.
    let mut missing = DependencyBridge.descriptor();
    missing.statements[1] = StatementDescriptor::Functionality {
        relation: RelationId(1),
        projection: vec![FieldId(0)].into_boxed_slice(),
    };
    assert!(missing.validate().is_err());
    let cases = cases();
    let work = WorkContext::new();
    let mut citations = 0;
    for case in &cases {
        let originals = case.normalized();
        let expected = case.oracle();
        let expected_statements: Vec<_> = (0..4).filter(|&i| !expected[i].is_empty()).collect();
        let mut state = MapState::new();
        for (side, facts) in originals.iter().enumerate() {
            for (origin, fact) in facts.iter().enumerate() {
                // Each world is [w,w+1). Coalesce only within the same original fact.
                let mut w = 0;
                while w < 64 {
                    if fact.region >> w & 1 == 0 {
                        w += 1;
                        continue;
                    }
                    let start = w;
                    while w < 64 && fact.region >> w & 1 != 0 {
                        w += 1;
                    }
                    state.insert(
                        RelationId(side as u32),
                        vec![
                            Value::U64(fact.group),
                            Value::U64(origin as u64),
                            Value::IntervalU64(Interval::<u64>::new(start, w).unwrap()),
                        ],
                    );
                }
            }
        }
        let judgment = judge_complete(
            &schema,
            &state,
            &work,
            JudgeBudget {
                examples_per_statement: 128,
            },
        )
        .unwrap();
        let violations = match judgment {
            Judgment::Admitted => vec![],
            Judgment::Rejected(v) => v.into_vec(),
        };
        assert_eq!(
            violations
                .iter()
                .map(|v| v.statement.0 as usize)
                .collect::<Vec<_>>(),
            expected_statements
        );
        for v in violations {
            // Native citations can be a witness subset. Validate their source identity
            // and soundness, not equality to the set of every overlapping contributor.
            for cited in v.examples {
                let Value::U64(origin) = cited.values[1] else {
                    panic!("origin field")
                };
                let fact: Fact = originals[cited.relation.0 as usize][origin as usize];
                assert!(expected[v.statement.0 as usize].contains(&fact));
                citations += 1;
            }
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"dependency_native_bridge\",\"cases\":{},\"sound_native_citations\":{},\"passed\":true}}",
        cases.len(),
        citations
    );
}
