//! The `U` pass: one cursor over `U | relation | statement | guard`.
//! Every entry's row id must resolve to a live fact that re-derives the
//! same guard bytes, and pointwise keys additionally re-verify per-group
//! disjointness: within one scalar-prefix group the cursor is ordered by
//! interval start, so one lookback checks `prev.end <= next.start` — the
//! invariant the neighbor probe assumes but never re-checks globally.

use crate::error::Result;
use crate::schema::{RelationId, StatementId, StatementView};
use crate::storage::keys;

use super::{StoreFinding, Sweep, namespace};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let schema = s.schema;
    let mut derived = Vec::new();
    // The previous pointwise guard key: consecutive keys of one
    // scalar-prefix group sit adjacent under the cursor, so a single
    // lookback walks every successive pair.
    let mut prev_pointwise: Option<&[u8]> = None;
    for entry in namespace(s.data, txn, keys::NS_GUARD)? {
        let (key, value) = entry?;
        // U | relation(4) | statement(2) | guard — the guard is nonempty
        // (projections are non-empty by validation).
        if key.len() <= 7 {
            s.malformed(key, "U key length");
            prev_pointwise = None;
            continue;
        }
        let rel = RelationId(u32::from_be_bytes(
            key[1..5].try_into().expect("fixed-width slice"),
        ));
        let sid = StatementId(u16::from_be_bytes(
            key[5..7].try_into().expect("fixed-width slice"),
        ));
        let Some(relation) = schema.relation_checked(rel) else {
            s.malformed(key, "U key relation");
            prev_pointwise = None;
            continue;
        };
        // Closed relations have no rows in the store: presence is the
        // finding (the F pass's exemption, mirrored).
        if relation.is_closed() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            prev_pointwise = None;
            continue;
        }
        let Some(StatementView::Key(key_id, statement)) = schema.statement_checked(sid) else {
            s.malformed(key, "U key statement");
            prev_pointwise = None;
            continue;
        };
        if statement.relation != rel || !relation.keys().contains(&key_id) {
            s.malformed(key, "U key statement");
            prev_pointwise = None;
            continue;
        }
        let guard = &key[7..];
        let Ok(row_bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "U row id");
            prev_pointwise = None;
            continue;
        };
        let row_id = u64::from_le_bytes(row_bytes);

        // U→F: the row id must resolve to a live fact re-deriving this
        // guard. A wrong-width fact was already convicted by the F pass
        // and cannot be sliced, so it passes here rather than double-
        // reporting.
        let backs = match s.fact(rel, row_id)? {
            None => false,
            Some(fact) if fact.len() != relation.layout().fact_width() => true,
            Some(fact) => {
                keys::guard_bytes(relation.layout(), &statement.projection, fact, &mut derived);
                derived == guard
            }
        };
        if !backs {
            s.push(StoreFinding::GuardWithoutFact {
                relation: rel,
                statement: sid,
                guard_key: key.into(),
            });
        }

        // Pointwise disjointness: the guard's 16-byte tail is
        // `start ‖ end` in order-preserving halves, so byte compare is
        // numeric compare. Half-open `[ps, pe)` and `[ns, ne)` with
        // `ps <= ns` by cursor order overlap iff `pe > ns`; equality is
        // adjacency, legal by construction.
        if !statement.pointwise || guard.len() < 16 {
            // A pointwise guard shorter than its interval is a width
            // desync the re-derivation above already convicted.
            prev_pointwise = None;
            continue;
        }
        if let Some(prev) = prev_pointwise {
            let same_group =
                prev.len() == key.len() && prev[..prev.len() - 16] == key[..key.len() - 16];
            if same_group && prev[prev.len() - 8..] > key[key.len() - 16..key.len() - 8] {
                s.push(StoreFinding::PointwiseOverlap {
                    relation: rel,
                    statement: sid,
                    first: prev.into(),
                    second: key.into(),
                });
            }
        }
        prev_pointwise = Some(key);
    }
    Ok(())
}
