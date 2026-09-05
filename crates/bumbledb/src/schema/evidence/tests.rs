//! Authored acceptance tests for the canonical rejection-evidence codec
//! (C01/C03 → C06 receipts): deterministic complete-or-refused encoding,
//! strict decode, schema interpretation, and the exact judge round-trip.
//! Mapped to ENG-005/ENG-007, E-ADMIT (evidence half), PROTO-02/-03
//! (recorded-outcome byte determinism half) and OPS-006 (bounded
//! diagnostics). Executed in F3, never before.

use super::{
    EvidenceDecodeError, EvidenceError, EvidenceInterpretError, FAMILY, LAYOUT, decode,
    encode_judged, encode_violations,
};
use crate::error::{CitedFact, Conflict, Direction, Violation, Violations};
use crate::schema::judge::{JudgeBudget, JudgedViolation, Judgment, MapState, judge_final_state};
use crate::schema::tests::{capacity_weighted, containment, fd, field, id_field, side};
use crate::schema::{
    FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor, StatementId, StatementKind,
    StatementRef, ValidateDescriptor as _, ValueType, Weight,
};
use crate::work::ExecutionPolicy;
use crate::{Value, WorkContext};
use std::time::Duration;

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 100_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap()
}

/// `Student { id, budget }` / `Attempt { id, student, units }` with a key
/// on each id, `Attempt.student ⊆ Student.id`, and a unit capacity
/// `Student(id) {0..1} Attempt(student)` — one schema whose one candidate
/// state violates all three statement kinds at once.
fn theory() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Student".into(),
                fields: vec![id_field("id"), field("budget", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Attempt".into(),
                fields: vec![
                    id_field("id"),
                    field("student", ValueType::U64),
                    field("units", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Unit,
                0,
                Some(crate::schema::Bound::Lit(1)),
                side(RelationId(1), &[FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid theory")
}

fn attempt(id: u64, student: u64, units: u64) -> Vec<Value> {
    vec![Value::U64(id), Value::U64(student), Value::U64(units)]
}

/// Key conflict on Attempt.id (two competing rows), an unwitnessed
/// containment source, and a capacity group of two over ceiling one.
fn violating_state() -> MapState {
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(5)]);
    state.insert(RelationId(1), attempt(1, 9, 3));
    state.insert(RelationId(1), attempt(1, 7, 3));
    state.insert(RelationId(1), attempt(2, 7, 3));
    state
}

fn rejected(schema: &Schema, state: &MapState) -> Box<[JudgedViolation]> {
    match judge_final_state(schema, state, &work(), JudgeBudget::default())
        .expect("judgment completes")
    {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("the fixture violates three statements"),
    }
}

/// Exactly what the live admission bridge does with judge output — the
/// (Violation, cited facts) pairs of the public rejection value. Mirrored
/// here because the bridge itself is module-private to `api::db`.
fn public_violations(schema: &Schema, judged: &[JudgedViolation]) -> Violations {
    let truncated: Vec<bool> = judged
        .iter()
        .map(|violation| violation.examples_truncated)
        .collect();
    let citations: Vec<(Violation, Box<[CitedFact]>)> = judged
        .iter()
        .map(|violation| {
            let reference = match schema.statement(violation.statement) {
                crate::schema::StatementView::Key(id, _) => StatementRef::Key(id),
                crate::schema::StatementView::Containment(id, _) => StatementRef::Containment(id),
                crate::schema::StatementView::Capacity(id, _) => StatementRef::Capacity(id),
            };
            let fact: Box<[u8]> = violation.examples.first().map_or_else(
                || Box::<[u8]>::from([]),
                |example| {
                    let fields = schema.relation(example.relation).fields();
                    Box::from(
                        crate::canonical::CanonicalRow::encode(fields, &example.values, &work())
                            .expect("fixture rows encode")
                            .as_bytes(),
                    )
                },
            );
            let typed = match violation.kind {
                StatementKind::Functionality => {
                    Violation::functionality(reference, fact, Conflict::Scalar)
                }
                StatementKind::Containment => {
                    Violation::containment(reference, Direction::SourceUnsatisfied, fact)
                }
                StatementKind::Capacity => {
                    Violation::capacity(reference, fact, violation.measure.unwrap_or(0))
                }
            };
            let cited: Box<[CitedFact]> = violation
                .examples
                .iter()
                .map(|example| {
                    CitedFact::new(
                        example.relation,
                        schema.relation(example.relation).fields().len(),
                        example.values.clone(),
                    )
                })
                .collect();
            (typed, cited)
        })
        .collect();
    Violations::from_pairs_with_truncation(
        citations.into_boxed_slice(),
        truncated.into_boxed_slice(),
    )
}

const BUDGET: usize = 64 * 1024;

/// C03 round-trip: `encode_judged` → decode → `to_judged` reproduces the
/// judge's complete verdict exactly — statement set, kinds, direction,
/// exact widened measure, every labeled example and the truncation flags.
#[test]
fn judge_output_round_trips_through_the_canonical_evidence_codec() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    assert_eq!(judged.len(), 3, "all three statement kinds violated");

    let bytes = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    let evidence = decode(&bytes, BUDGET).expect("strict decode");
    let round = evidence.to_judged(&schema, &work()).expect("interprets");
    assert_eq!(round, judged);

    // The decoded structure exposes the same complete ordered verdict.
    let ids: Vec<StatementId> = evidence
        .violations()
        .iter()
        .map(|violation| violation.statement)
        .collect();
    assert_eq!(ids, vec![StatementId(1), StatementId(2), StatementId(3)]);
    assert_eq!(evidence.violations()[2].measure, Some(2));
    assert_eq!(
        evidence.violations()[1].direction,
        Some(Direction::SourceUnsatisfied)
    );
}

/// Replay determinism (the log compares recorded evidence byte for byte):
/// the live path (public `Violations`) and the replay path (re-judged
/// output) produce IDENTICAL bytes, and the public value round-trips.
#[test]
fn live_rejection_bytes_equal_replayed_judge_bytes() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let violations = public_violations(&schema, &judged);

    let live = encode_violations(&schema, &violations, BUDGET, &work()).expect("encodes");
    let replayed = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    assert_eq!(live, replayed, "one deterministic byte spelling");

    let round = decode(&live, BUDGET)
        .expect("decodes")
        .to_violations(&schema, &work())
        .expect("interprets");
    assert_eq!(round, violations);
}

/// The bytes are a pure function of the input, and the frame header is the
/// pinned domain-separated family — golden-checked so no other frame
/// family can alias it.
#[test]
fn deterministic_bytes_and_pinned_header() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let first = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    let second = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    assert_eq!(first, second);

    assert_eq!(&first[..21], FAMILY);
    assert_eq!(first[21..23], LAYOUT.to_be_bytes());
    assert_eq!(first[23], 1, "the evidence frame kind");
    assert_eq!(first[24..28], 3u32.to_be_bytes(), "three violations");
    assert_eq!(FAMILY, b"bumbledb.evidence.v1\0");
}

/// Under a byte budget the codec drops EXAMPLES deterministically (uniform
/// rank across violations), labels the truncation per violation, and never
/// drops a violated statement.
#[test]
fn byte_budget_drops_examples_deterministically_and_labels_truncation() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let full = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");

    // Find a budget that forces truncation but fits the skeleton.
    let tight = full.len() - 1;
    let bytes = encode_judged(&schema, &judged, tight, &work()).expect("still encodes");
    assert!(bytes.len() <= tight);
    let evidence = decode(&bytes, BUDGET).expect("decodes");
    assert_eq!(evidence.violations().len(), 3, "the statement set is whole");
    assert!(
        evidence
            .violations()
            .iter()
            .any(|violation| violation.examples_truncated),
        "dropped examples are labeled"
    );
    let kept: usize = evidence
        .violations()
        .iter()
        .map(|violation| violation.examples.len())
        .sum();
    let total: usize = judged
        .iter()
        .map(|violation| violation.examples.len())
        .sum();
    assert!(kept < total, "something was actually dropped");

    // Deterministic: the same tight budget yields the same bytes.
    assert_eq!(
        bytes,
        encode_judged(&schema, &judged, tight, &work()).expect("encodes")
    );

    // The judge's own truncation label survives encoding even when the
    // byte budget drops nothing.
    let mut labeled = judged.clone();
    labeled[0].examples_truncated = true;
    let bytes = encode_judged(&schema, &labeled, BUDGET, &work()).expect("encodes");
    let round = decode(&bytes, BUDGET)
        .expect("decodes")
        .to_judged(&schema, &work())
        .expect("interprets");
    assert!(round[0].examples_truncated);
}

/// The judge's own per-statement truncation label crosses the PUBLIC
/// boundary ([`Violations::examples_truncated`], threaded by the rejection
/// bridge) and round-trips through evidence encode/decode: live bytes equal
/// replay bytes under a genuine judge-level example drop, decoded evidence
/// carries the label, and re-encoding the decoded public value is a byte
/// fixed point.
#[test]
fn judge_truncation_label_round_trips_through_the_public_boundary() {
    let schema = theory();
    // A genuine judge-level drop: the key violation cites two competing
    // rows, so an example budget of one truncates it at the JUDGE, before
    // any byte budget exists.
    let judged = match judge_final_state(
        &schema,
        &violating_state(),
        &work(),
        JudgeBudget {
            examples_per_statement: 1,
        },
    )
    .expect("judgment completes")
    {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("the fixture violates three statements"),
    };
    assert!(
        judged.iter().any(|violation| violation.examples_truncated),
        "the tightened judge budget truncates at least one statement"
    );
    assert!(
        judged.iter().any(|violation| !violation.examples_truncated),
        "and leaves at least one statement complete (the label is per statement)"
    );

    // The public value carries the label per citation, in order.
    let violations = public_violations(&schema, &judged);
    for (index, violation) in judged.iter().enumerate() {
        assert_eq!(
            violations.examples_truncated(index),
            violation.examples_truncated,
            "citation {index} carries the judge's label"
        );
    }
    assert!(
        !violations.examples_truncated(violations.len()),
        "out-of-range reads as false, never a panic"
    );

    // Live (public) and replay (judge) bytes agree under the label.
    let live = encode_violations(&schema, &violations, BUDGET, &work()).expect("encodes");
    let replayed = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    assert_eq!(live, replayed, "one deterministic byte spelling");

    // Decode exposes the label; interpretation preserves it on the public
    // value; re-encoding the decoded value reproduces the same bytes.
    let evidence = decode(&live, BUDGET).expect("decodes");
    for (index, violation) in judged.iter().enumerate() {
        assert_eq!(
            evidence.violations()[index].examples_truncated,
            violation.examples_truncated
        );
    }
    let round = evidence
        .to_violations(&schema, &work())
        .expect("interprets");
    assert_eq!(round, violations, "labels participate in the round-trip");
    assert_eq!(
        encode_violations(&schema, &round, BUDGET, &work()).expect("encodes"),
        live,
        "decode → interpret → encode is a byte fixed point"
    );
}

/// A budget the example-free statement skeleton cannot fit refuses whole:
/// the caller refuses before deciding instead of recording a rejection
/// with a silently narrowed statement set.
#[test]
fn skeleton_overflow_refuses_before_deciding() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let result = encode_judged(&schema, &judged, 40, &work());
    match result {
        Err(EvidenceError::Budget { needed, budget }) => {
            assert_eq!(budget, 40);
            assert!(needed > 40);
        }
        other => panic!("expected the budget refusal, got {other:?}"),
    }

    // With exactly the skeleton budget, encoding succeeds with zero
    // examples, every flag labeled truncated where facts existed.
    let skeleton = 28 + (2 + 1 + 1 + 4) /* key */ + (2 + 1 + 1 + 1 + 4) /* containment */
        + (2 + 1 + 16 + 1 + 4) /* capacity */;
    let bytes = encode_judged(&schema, &judged, skeleton, &work()).expect("skeleton fits");
    assert_eq!(bytes.len(), skeleton);
    let evidence = decode(&bytes, BUDGET).expect("decodes");
    assert!(
        evidence
            .violations()
            .iter()
            .all(|violation| { violation.examples.is_empty() && violation.examples_truncated })
    );
}

/// Strict decode: foreign families/layouts/kinds, truncation, trailing
/// bytes, unsorted statements, zero counts, oversized claims and the
/// caller's byte cap all refuse with typed errors — never partial data.
#[test]
fn strict_decode_refuses_foreign_and_malformed_frames() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let bytes = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");

    // Caller cap.
    assert_eq!(
        decode(&bytes, bytes.len() - 1),
        Err(EvidenceDecodeError::LimitExceeded)
    );

    // Foreign family (one flipped magic byte).
    let mut forged = bytes.clone();
    forged[0] ^= 1;
    assert_eq!(decode(&forged, BUDGET), Err(EvidenceDecodeError::Family));

    // Future layout.
    let mut forged = bytes.clone();
    forged[22] = 2;
    assert_eq!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::Layout { got: 2 })
    );

    // Wrong frame kind.
    let mut forged = bytes.clone();
    forged[23] = 9;
    assert_eq!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::Kind { got: 9 })
    );

    // Truncated buffer refuses at every prefix length.
    for cut in [10, 27, 30, bytes.len() - 1] {
        assert!(
            matches!(
                decode(&bytes[..cut], BUDGET),
                Err(EvidenceDecodeError::Truncated { .. } | EvidenceDecodeError::InvalidCount)
            ),
            "prefix {cut} must refuse"
        );
    }

    // Trailing garbage refuses.
    let mut forged = bytes.clone();
    forged.push(0);
    assert!(matches!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::TrailingBytes { .. })
    ));

    // Zero violations are unrepresentable.
    let mut forged = bytes.clone();
    forged[24..28].copy_from_slice(&0u32.to_be_bytes());
    assert_eq!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::InvalidCount)
    );

    // A count claiming more violations than the bytes could hold refuses
    // before any allocation of that size.
    let mut forged = bytes.clone();
    forged[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::InvalidCount)
    );

    // Unsorted statement ids: duplicate the first violation's id into the
    // second slot by rewriting the second statement id to the first's.
    // Layout: header(28) + violation 0 (statement u16 at 28).
    let first_id = [bytes[28], bytes[29]];
    // Find the second violation's statement offset by re-walking: the
    // first violation is a key with 4 examples? Walk structurally instead:
    // decode and re-encode a hand-built unsorted frame.
    let mut unsorted = Vec::new();
    unsorted.extend_from_slice(FAMILY);
    unsorted.extend_from_slice(&LAYOUT.to_be_bytes());
    unsorted.push(1);
    unsorted.extend_from_slice(&2u32.to_be_bytes());
    for _ in 0..2 {
        unsorted.extend_from_slice(&first_id);
        unsorted.push(0); // functionality
        unsorted.push(0); // not truncated
        unsorted.extend_from_slice(&0u32.to_be_bytes()); // no examples
    }
    assert_eq!(
        decode(&unsorted, BUDGET),
        Err(EvidenceDecodeError::Unordered)
    );

    // A bad statement-kind tag refuses.
    let mut forged = bytes.clone();
    forged[30] = 7; // the first violation's kind tag
    assert!(matches!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::Tag { got: 7, .. })
    ));

    // A non-boolean truncation flag refuses.
    let mut forged = bytes.clone();
    forged[31] = 2; // first violation: key → truncated flag follows the kind
    assert!(matches!(
        decode(&forged, BUDGET),
        Err(EvidenceDecodeError::Tag { got: 2, .. })
    ));
}

/// Grammatical evidence that does not belong to the presented schema
/// refuses at interpretation with typed errors — foreign statement ids,
/// kind mismatches, unknown relations and corrupt canonical rows.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one refusal scenario per foreign-evidence shape"
)]
fn interpretation_refuses_foreign_schema_data() {
    let schema = theory();
    let judged = rejected(&schema, &violating_state());
    let bytes = encode_judged(&schema, &judged, BUDGET, &work()).expect("encodes");
    let evidence = decode(&bytes, BUDGET).expect("decodes");

    // A schema with fewer statements: id 3 is foreign.
    let smaller = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Student".into(),
            fields: vec![id_field("id"), field("budget", ValueType::U64)],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(0)])],
    }
    .validate()
    .expect("valid");
    assert!(matches!(
        evidence.to_judged(&smaller, &work()),
        Err(EvidenceInterpretError::ForeignStatement { .. })
    ));

    // Same statement count, different kinds at those ids.
    let reshaped = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Student".into(),
                fields: vec![id_field("id"), field("budget", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Attempt".into(),
                fields: vec![
                    id_field("id"),
                    field("student", ValueType::U64),
                    field("units", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(1)]), // id 2 is a KEY here, not containment
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid");
    assert!(matches!(
        evidence.to_judged(&reshaped, &work()),
        Err(EvidenceInterpretError::KindMismatch {
            statement: StatementId(2)
        })
    ));

    // Corrupt an example row's payload: strict canonical decode refuses.
    let mut forged = decode(&bytes, BUDGET).expect("decodes");
    let mut violations: Vec<_> = forged.violations().to_vec();
    let example = &mut violations[0].examples[0];
    let mut fact = example.fact.to_vec();
    let last = fact.len() - 1;
    fact[last] ^= 0xff;
    fact.pop(); // truncate one byte: arity/length now disagree
    example.fact = fact.into_boxed_slice();
    forged = super::ViolationEvidence {
        violations: violations.into_boxed_slice(),
    };
    assert!(matches!(
        forged.to_judged(&schema, &work()),
        Err(EvidenceInterpretError::Row(_))
    ));

    // An example naming a relation this schema does not have refuses.
    let evidence = decode(&bytes, BUDGET).expect("decodes");
    let mut violations: Vec<_> = evidence.violations().to_vec();
    violations[0].examples[0].relation = RelationId(99);
    let foreign = super::ViolationEvidence {
        violations: violations.into_boxed_slice(),
    };
    assert!(matches!(
        foreign.to_judged(&schema, &work()),
        Err(EvidenceInterpretError::ForeignRelation {
            relation: RelationId(99)
        })
    ));

    // A target-required direction is representable evidence (the public
    // Direction has the arm) but is never judge output: to_judged refuses,
    // to_violations carries it faithfully.
    let target_required = Violations::from_pairs(Box::from([(
        Violation::containment(
            schema.cite(StatementId(2)),
            Direction::TargetRequired,
            Box::<[u8]>::from([]),
        ),
        Box::<[CitedFact]>::from([]),
    )]));
    let bytes = encode_violations(&schema, &target_required, BUDGET, &work()).expect("encodes");
    let evidence = decode(&bytes, BUDGET).expect("decodes");
    assert!(matches!(
        evidence.to_judged(&schema, &work()),
        Err(EvidenceInterpretError::ForeignDirection {
            statement: StatementId(2)
        })
    ));
    assert_eq!(
        evidence
            .to_violations(&schema, &work())
            .expect("interprets"),
        target_required
    );
}

/// Total input refusals: an empty verdict, unsorted statements and the
/// old engine's pointwise conflict detail are typed encode errors.
#[test]
fn empty_unordered_and_pointwise_inputs_refuse() {
    let schema = theory();
    assert_eq!(
        encode_judged(&schema, &[], BUDGET, &work()),
        Err(EvidenceError::Empty)
    );

    let judged = rejected(&schema, &violating_state());
    let mut unsorted: Vec<JudgedViolation> = judged.to_vec();
    unsorted.swap(0, 1);
    assert_eq!(
        encode_judged(&schema, &unsorted, BUDGET, &work()),
        Err(EvidenceError::Unordered)
    );

    let pointwise = Violations::from_pairs(Box::from([(
        Violation::functionality(
            schema.cite(StatementId(0)),
            Box::<[u8]>::from([]),
            Conflict::Pointwise {
                incumbent: Box::from([1, 2, 3]),
            },
        ),
        Box::<[CitedFact]>::from([]),
    )]));
    assert_eq!(
        encode_violations(&schema, &pointwise, BUDGET, &work()),
        Err(EvidenceError::PointwiseConflict)
    );
}

/// D05 — evidence bytes are a function of logical facts, not insertion or
/// remint order. Sorting already-selected row-id examples would fail this.
#[test]
fn d05_evidence_bytes_survive_opposite_insertion_and_remint() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "User".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("email", ValueType::String),
            ],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(0)]), fd(RelationId(0), &[FieldId(1)])],
    }
    .validate()
    .expect("valid");
    let mut forward = MapState::new();
    let mut reverse = MapState::new();
    for id in 0..6u64 {
        let row = vec![Value::U64(id), Value::String("shared@ex".into())];
        forward.insert(RelationId(0), row.clone());
        reverse.insert(
            RelationId(0),
            vec![Value::U64(5 - id), Value::String("shared@ex".into())],
        );
    }
    let budget = JudgeBudget {
        examples_per_statement: 2,
    };
    let left = match judge_final_state(&schema, &forward, &work(), budget).expect("forward") {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("must reject"),
    };
    let right = match judge_final_state(&schema, &reverse, &work(), budget).expect("reverse") {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("must reject"),
    };
    assert_eq!(
        encode_judged(&schema, &left, BUDGET, &work()).expect("left"),
        encode_judged(&schema, &right, BUDGET, &work()).expect("right"),
        "receipt bytes must agree across remint order"
    );
    assert!(left[0].examples_truncated);
}
