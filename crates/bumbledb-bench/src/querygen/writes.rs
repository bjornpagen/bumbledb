//! The closed-relation judgment write scenarios over the target theory
//! (PRD 06, the fourth pattern class): seeded random single-fact writes
//! whose verdicts the differential runner compares typed — attempted
//! closed-relation writes (`ClosedRelationWrite`), subset-violating
//! inserts (in-range-but-ψ-excluded AND out-of-range ids, both below
//! and beyond the 256-row roster cap), and plain-reference dangling
//! handles. Every case carries its hand-derived expected violation so
//! the consumer asserts the typed identity, never just abort-vs-commit.
//!
//! The facts are constructed to violate **exactly one** statement over
//! the seeded unit world (`JournalEntry` rows use non-`Import` sources
//! so the DU pair stays silent; fresh ids sit beyond every seeded row).

use bumbledb::{Direction, RelationId, Value};

use crate::gen::Rng;
use crate::naive::Violation;
use crate::querygen::target::{self, ids};

/// Which judgment scenario a generated write is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedWriteKind {
    /// An insert naming a closed relation.
    ClosedInsert,
    /// A delete naming a closed relation.
    ClosedDelete,
    /// A plain closed reference dangling inside the word (id in
    /// `3..256`): the compiled-subset miss below the roster cap.
    DanglingHandle,
    /// A plain closed reference beyond the roster cap (id ≥ 256): the
    /// bit position falls outside the 4×u64 member set entirely — the
    /// same violation, no special error.
    BeyondRosterCap,
    /// A ψ-subset reference that is a real row OUTSIDE ψ (`Usd`/`Eur`
    /// under `minor_units == 0`): in range, excluded by selection.
    PsiExcluded,
    /// A ψ-subset reference out of range entirely.
    PsiOutOfRange,
}

/// One generated judgment write: the fact, whether it is a delete, and
/// the violation both oracles must report, typed whole.
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

/// `n` seeded cases cycling the six kinds (so any `n ≥ 6` covers all of
/// them — asserted by the closed-class self-test).
#[must_use]
pub fn closed_write_cases(rng: &mut Rng, n: usize) -> Vec<ClosedWriteCase> {
    (0..n)
        .map(|i| case(KINDS[i % KINDS.len()], rng, i))
        .collect()
}

fn case(kind: ClosedWriteKind, rng: &mut Rng, index: usize) -> ClosedWriteCase {
    // Fresh ids beyond any seeded unit-world row (the consumer seeds
    // single-digit ids), so the only violated statement is the case's.
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
                3 + rng.range(253) // in the word, outside the extension
            } else {
                256 + rng.range(1 << 20) // beyond the 4×u64 member set
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
                rng.range(target::ZERO_DECIMAL_CURRENCY) // Usd/Eur: real rows outside ψ
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
