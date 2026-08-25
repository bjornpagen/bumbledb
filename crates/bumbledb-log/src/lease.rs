//! Fresh-id leases: `ids/{relation}/{field}` holds a canonical u64
//! (decimal ASCII), the next unleased id. Birth is `put_create` with
//! body `4096` — the creator thereby claims `[0, 4096)`; a writer
//! leases `[n, n+4096)` by CAS-incrementing, retrying `Moved`
//! unbounded. Commands carry concrete ids, so replay determinism never
//! depends on the counter; the counter object is also the failover
//! floor an adopting writer reads. Leased ids are unique, never dense,
//! deliberately — abandoned tails of a block are the price of
//! coordination-free draws, and sequences are refused by design.
//!
//! `draw` is one algebra (10 §3):
//!
//! ```text
//! Lease.draw(count) =
//!   | Refused(OverWidth)     when count > LEASE_WIDTH
//!   | Refused(Exhausted)     when next + count would exceed u64
//!   | Drawn(range)           otherwise, contiguous, body runs once
//! ```
//!
//! `count` is unsigned, so a negative demand is unconstructible.
//! Counter mutations carry the writer's fencing token (20): a stale
//! holder's write is the token the store CAS no longer wins.

use std::collections::BTreeMap;
use std::ops::Range;

use bumbledb::schema::{FieldId, RelationId};

use crate::manifest::Text;
use crate::store::{
    prove_create, prove_swap, Create, ObjectStore, Result as StoreResult, StoreKey, Swap,
};

/// The lease width: one CAS increment claims this many ids.
pub const LEASE_WIDTH: u64 = 4096;

/// The counter object key: `ids/{relation:08x}/{field:04x}` under the
/// store prefix.
#[must_use]
pub fn ids_key(prefix: &str, relation: RelationId, field: FieldId) -> StoreKey {
    let rest = format!("ids/{:08x}/{:04x}", relation.0, field.0);
    let raw = if prefix.is_empty() {
        rest
    } else {
        format!("{prefix}/{rest}")
    };
    StoreKey::of(&raw)
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
    /// `next + count` would leave u64 — the id space is spent.
    Exhausted {
        relation: RelationId,
        field: FieldId,
    },
    /// A single draw larger than one lease width; the width is the
    /// protocol's one block size.
    OverWidth { requested: u64 },
}

/// `Lease.draw(count)`: `OverWidth | Exhausted | Drawn`, plus `Counter`
/// when the object is not a decimal. `Drawn` carries the fencing token
/// the counter write rode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leased {
    Drawn { range: Range<u64>, token: u64 },
    Refused(LeaseRefusal),
}

/// Leases one block `[n, n+4096)` for `(relation, field)`: birth via
/// create-only PUT claiming `[0, 4096)`, otherwise GET + CAS with
/// unbounded `Moved` retry — every retry is someone else's successful
/// lease, so the loop always advances globally. `token` is the fencing
/// token this write carries (20).
pub fn lease_block<S: ObjectStore>(
    store: &S,
    prefix: &str,
    relation: RelationId,
    field: FieldId,
    token: u64,
) -> StoreResult<Leased> {
    let key = ids_key(prefix, relation, field);
    loop {
        let Some(fetched) = store.get(&key)? else {
            let birth = format!("{LEASE_WIDTH}");
            let outcome = store.put_create(&key, birth.as_bytes())?;
            match prove_create(store, &key, birth.as_bytes(), outcome)? {
                Create::Created(_) => {
                    return Ok(Leased::Drawn {
                        range: 0..LEASE_WIDTH,
                        token,
                    });
                }
                Create::Exists | Create::Ambiguous => continue,
            }
        };
        let Some(next) = parse_counter(&fetched.bytes) else {
            return Ok(Leased::Refused(LeaseRefusal::Counter { relation, field }));
        };
        let Some(end) = next.checked_add(LEASE_WIDTH) else {
            return Ok(Leased::Refused(LeaseRefusal::Exhausted { relation, field }));
        };
        let body = format!("{end}");
        match prove_swap(
            store,
            &key,
            body.as_bytes(),
            store.put_swap(&key, body.as_bytes(), &fetched.etag)?,
        )? {
            Swap::Swapped(_) => {
                return Ok(Leased::Drawn {
                    range: next..end,
                    token,
                });
            }
            Swap::Moved | Swap::Ambiguous => {}
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
/// the current block, plus the fencing token every counter write
/// carries. A draw that outgrows the tail abandons it and leases fresh
/// — unique, never dense.
#[derive(Debug)]
pub struct Leases {
    ranges: BTreeMap<(RelationId, FieldId), Range<u64>>,
    token: u64,
}

impl Leases {
    #[must_use]
    pub fn new(token: u64) -> Self {
        Self {
            ranges: BTreeMap::new(),
            token,
        }
    }

    /// The fencing token this cache's writes carry.
    #[must_use]
    pub const fn token(&self) -> u64 {
        self.token
    }

    /// Draws `count` contiguous ids, from the cached block when it
    /// holds enough, otherwise from one fresh CAS lease. `count` is
    /// unsigned.
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
        if count == 0 {
            return Ok(Leased::Drawn {
                range: 0..0,
                token: self.token,
            });
        }
        if let Some(cached) = self.ranges.get_mut(&(relation, field)) {
            match cached.start.checked_add(count) {
                None => {
                    return Ok(Leased::Refused(LeaseRefusal::Exhausted { relation, field }));
                }
                Some(end) if end <= cached.end => {
                    let drawn = cached.start..end;
                    cached.start = end;
                    return Ok(Leased::Drawn {
                        range: drawn,
                        token: self.token,
                    });
                }
                Some(_) => {}
            }
        }
        match lease_block(store, prefix, relation, field, self.token)? {
            Leased::Drawn {
                range: block,
                token,
            } => {
                let Some(end) = block.start.checked_add(count) else {
                    return Ok(Leased::Refused(LeaseRefusal::Exhausted { relation, field }));
                };
                self.ranges.insert((relation, field), end..block.end);
                Ok(Leased::Drawn {
                    range: block.start..end,
                    token,
                })
            }
            refused @ Leased::Refused(_) => Ok(refused),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::mem::MemStore;

    const REL: RelationId = RelationId(1);
    const FIELD: FieldId = FieldId(0);
    const TOKEN: u64 = 7;

    #[test]
    fn draw_is_over_width_exhausted_or_drawn() {
        let store = MemStore::new();
        let mut leases = Leases::new(TOKEN);
        assert_eq!(
            leases
                .draw(&store, "", REL, FIELD, LEASE_WIDTH + 1)
                .expect("over-width"),
            Leased::Refused(LeaseRefusal::OverWidth {
                requested: LEASE_WIDTH + 1
            })
        );

        let Leased::Drawn { range, token } = leases.draw(&store, "", REL, FIELD, 3).expect("birth")
        else {
            panic!("birth is Drawn");
        };
        assert_eq!(range, 0..3);
        assert_eq!(token, TOKEN);

        let key = ids_key("", REL, FIELD);
        store
            .put_swap(
                &key,
                b"18446744073709551615",
                &store.get(&key).unwrap().unwrap().etag,
            )
            .expect("poison the counter at u64::MAX");
        let mut spent = Leases::new(TOKEN);
        assert_eq!(
            spent.draw(&store, "", REL, FIELD, 1).expect("exhausted"),
            Leased::Refused(LeaseRefusal::Exhausted {
                relation: REL,
                field: FIELD
            })
        );
    }

    #[test]
    fn count_is_unsigned_and_zero_is_drawn_empty() {
        let store = MemStore::new();
        let mut leases = Leases::new(TOKEN);
        let Leased::Drawn { range, token } = leases.draw(&store, "", REL, FIELD, 0).expect("zero")
        else {
            panic!("zero demand is Drawn");
        };
        assert_eq!(range, 0..0);
        assert_eq!(token, TOKEN);
        assert!(store.get(&ids_key("", REL, FIELD)).expect("get").is_none());
    }
}
