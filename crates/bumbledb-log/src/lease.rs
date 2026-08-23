//! Fresh-id leases: `ids/{relation}/{field}` holds a canonical u64
//! (decimal ASCII), the next unleased id. Birth is `put_create` with
//! body `4096` — the creator thereby claims `[0, 4096)`; a writer
//! leases `[n, n+4096)` by CAS-incrementing, retrying `Moved`
//! unbounded. Commands carry concrete ids, so replay determinism never
//! depends on the counter; the counter object is also the failover
//! floor an adopting writer reads. Leased ids are unique, never dense,
//! deliberately — abandoned tails of a block are the price of
//! coordination-free draws, and sequences are refused by design.

use std::collections::BTreeMap;
use std::ops::Range;

use bumbledb::schema::{FieldId, RelationId};

use crate::manifest::Text;
use crate::store::{Create, ObjectStore, Result as StoreResult, Swap};

/// The lease width: one CAS increment claims this many ids.
pub const LEASE_WIDTH: u64 = 4096;

/// The counter object key: `ids/{relation:08x}/{field:04x}` under the
/// store prefix.
#[must_use]
pub fn ids_key(prefix: &str, relation: RelationId, field: FieldId) -> String {
    let rest = format!("ids/{:08x}/{:04x}", relation.0, field.0);
    if prefix.is_empty() {
        rest
    } else {
        format!("{prefix}/{rest}")
    }
}

/// Typed lease refusals. None of these retry: each names a disagreement
/// no repetition mends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseRefusal {
    /// The counter body is not a canonical decimal u64.
    Counter {
        relation: RelationId,
        field: FieldId,
    },
    /// The next lease would leave u64 — the id space is spent.
    Exhausted {
        relation: RelationId,
        field: FieldId,
    },
    /// A single draw larger than one lease width; the width is the
    /// protocol's one block size.
    OverWidth { requested: u64 },
}

/// Outcome of a lease or draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leased {
    Range(Range<u64>),
    Refused(LeaseRefusal),
}

/// Leases one block `[n, n+4096)` for `(relation, field)`: birth via
/// create-only PUT claiming `[0, 4096)`, otherwise GET + CAS with
/// unbounded `Moved` retry — every retry is someone else's successful
/// lease, so the loop always advances globally.
pub fn lease_block<S: ObjectStore>(
    store: &S,
    prefix: &str,
    relation: RelationId,
    field: FieldId,
) -> StoreResult<Leased> {
    let key = ids_key(prefix, relation, field);
    loop {
        let Some(fetched) = store.get(&key)? else {
            match store.put_create(&key, format!("{LEASE_WIDTH}").as_bytes())? {
                Create::Created(_) => return Ok(Leased::Range(0..LEASE_WIDTH)),
                Create::Exists => continue,
            }
        };
        let Some(next) = parse_counter(&fetched.bytes) else {
            return Ok(Leased::Refused(LeaseRefusal::Counter { relation, field }));
        };
        let Some(end) = next.checked_add(LEASE_WIDTH) else {
            return Ok(Leased::Refused(LeaseRefusal::Exhausted { relation, field }));
        };
        match store.put_swap(&key, format!("{end}").as_bytes(), &fetched.etag)? {
            Swap::Swapped(_) => return Ok(Leased::Range(next..end)),
            Swap::Moved => {}
        }
    }
}

fn parse_counter(bytes: &[u8]) -> Option<u64> {
    let mut text = Text::new(bytes);
    let value = text.u64().ok()?;
    text.end().ok()?;
    Some(value)
}

/// The writer's local lease cache: per `(relation, field)`, the tail of
/// the current block. A draw that outgrows the tail abandons it and
/// leases fresh — unique, never dense.
#[derive(Debug, Default)]
pub struct Leases {
    ranges: BTreeMap<(RelationId, FieldId), Range<u64>>,
}

impl Leases {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws `count` contiguous ids, from the cached block when it
    /// holds enough, otherwise from one fresh CAS lease.
    pub fn draw<S: ObjectStore>(
        &mut self,
        store: &S,
        prefix: &str,
        relation: RelationId,
        field: FieldId,
        count: u64,
    ) -> StoreResult<Leased> {
        if count > LEASE_WIDTH {
            return Ok(Leased::Refused(LeaseRefusal::OverWidth {
                requested: count,
            }));
        }
        if let Some(cached) = self.ranges.get_mut(&(relation, field))
            && cached.end - cached.start >= count
        {
            let drawn = cached.start..cached.start + count;
            cached.start += count;
            return Ok(Leased::Range(drawn));
        }
        match lease_block(store, prefix, relation, field)? {
            Leased::Range(block) => {
                let drawn = block.start..block.start + count;
                self.ranges
                    .insert((relation, field), block.start + count..block.end);
                Ok(Leased::Range(drawn))
            }
            refused @ Leased::Refused(_) => Ok(refused),
        }
    }
}
