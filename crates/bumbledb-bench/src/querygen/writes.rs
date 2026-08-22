use bumbledb::{Direction, RelationId, Value};

use crate::corpus_gen::Rng;
use crate::naive::Violation;
use crate::querygen::target::{self, ids};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedWriteKind {

    ClosedInsert,

    ClosedDelete,

    DanglingHandle,

    BeyondRosterCap,

    PsiExcluded,

    PsiOutOfRange,
}

#[derive(Debug, Clone)]
pub struct ClosedWriteCase {
    pub kind: ClosedWriteKind,
    pub relation: RelationId,
    pub fact: Vec<Value>,
    pub delete: bool,
    pub expected: Violation,
}

const KINDS: [ClosedWriteKind; 6] = [
    ClosedWriteKind::ClosedInsert,
    ClosedWriteKind::ClosedDelete,
    ClosedWriteKind::DanglingHandle,
    ClosedWriteKind::BeyondRosterCap,
    ClosedWriteKind::PsiExcluded,
    ClosedWriteKind::PsiOutOfRange,
];

#[must_use]
pub fn closed_write_cases(rng: &mut Rng, n: usize) -> Vec<ClosedWriteCase> {
    (0..n)
        .map(|i| case(KINDS[i % KINDS.len()], rng, i))
        .collect()
}

fn case(kind: ClosedWriteKind, rng: &mut Rng, index: usize) -> ClosedWriteCase {

    let fresh = 1_000 + index as u64;
    match kind {
        ClosedWriteKind::ClosedInsert => ClosedWriteCase {
            kind,
            relation: ids::CURRENCY,
            fact: vec![Value::U64(3 + rng.range(8)), Value::U64(2)],
            delete: false,
            expected: Violation::ClosedRelationWrite {
                relation: ids::CURRENCY,
            },
        },
        ClosedWriteKind::ClosedDelete => {
            let relation = if rng.chance(1, 2) {
                ids::SOURCE
            } else {
                ids::TAG
            };
            ClosedWriteCase {
                kind,
                relation,
                fact: vec![Value::U64(rng.range(3))],
                delete: true,
                expected: Violation::ClosedRelationWrite { relation },
            }
        }
        ClosedWriteKind::DanglingHandle | ClosedWriteKind::BeyondRosterCap => {
            let source = if kind == ClosedWriteKind::DanglingHandle {
                3 + rng.range(253) 
            } else {
                256 + rng.range(1 << 20) 
            };
            ClosedWriteCase {
                kind,
                relation: ids::JOURNAL_ENTRY,
                fact: vec![
                    Value::U64(fresh),
                    Value::U64(source),
                    Value::I64(target::posting_at(fresh)),
                ],
                delete: false,
                expected: Violation::Containment {
                    statement: target::VOCAB_SOURCE,
                    direction: Direction::SourceUnsatisfied,
                },
            }
        }
        ClosedWriteKind::PsiExcluded | ClosedWriteKind::PsiOutOfRange => {
            let currency = if kind == ClosedWriteKind::PsiExcluded {
                rng.range(target::ZERO_DECIMAL_CURRENCY) 
            } else if rng.chance(1, 2) {
                3 + rng.range(253)
            } else {
                256 + rng.range(1 << 20)
            };
            ClosedWriteCase {
                kind,
                relation: ids::CASH_ROUNDING,
                fact: vec![Value::U64(currency)],
                delete: false,
                expected: Violation::Containment {
                    statement: target::CASH_ROUNDING_SUBSET,
                    direction: Direction::SourceUnsatisfied,
                },
            }
        }
    }
}
