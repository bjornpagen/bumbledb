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
//! Exhausted is that draw, not the block increment: a last partial
//! block still leases when `next + count` fits. Counter mutations
//! carry the writer's fencing token (20): a stale holder's write is
//! the token the store CAS does not win. A cache-hit `Drawn` is
//! not a write; a write always names the token it rides.

use std::collections::BTreeMap;
use std::ops::Range;

use bumbledb::schema::{FieldId, RelationId};

use crate::store::{Create, Etag, Fenced, ObjectStore, Result as StoreResult, StoreKey, Swap};

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
/// the counter write rode, or the writer's token when the draw was a
/// cache hit and there was no write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leased {
    Drawn { range: Range<u64>, token: u64 },
    Refused(LeaseRefusal),
}

/// Counter birth. The write is [`Fenced`]: `token` is an argument of
/// `put_create`, not a field discarded before the call. A stale holder
/// is the token the store CAS does not win (20). `Drawn` takes this
/// token, not a token that skipped the write.
fn put_create<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    bytes: &[u8],
    token: u64,
) -> StoreResult<(Create, u64)> {
    let write = Fenced { bytes, token };
    Ok((store.put_create(key, write)?, write.token))
}

/// Counter CAS. The write is [`Fenced`]: `token` is an argument of
/// `put_swap`, not a field discarded before the call. A stale holder
/// is the token the store CAS does not win (20). `Drawn` takes this
/// token, not a token that skipped the write.
fn put_swap<S: ObjectStore>(
    store: &S,
    key: &StoreKey,
    bytes: &[u8],
    etag: &Etag,
    token: u64,
) -> StoreResult<(Swap, u64)> {
    let write = Fenced { bytes, token };
    Ok((store.put_swap(key, write, etag)?, write.token))
}

/// Leases one block for `(relation, field)` that can serve `count`:
/// birth via create-only PUT claiming `[0, 4096)`, otherwise GET + CAS
/// with unbounded `Moved` retry — every retry is someone else's
/// successful lease, so the loop always advances globally. `Ambiguous`
/// is not a draw: every writer that read the same `next` writes the
/// same increment body, so a GET that matches those bytes does not
/// name this writer. The loop re-reads and takes the next block
/// (unique, never dense). `token` is the fencing token this write
/// carries (20). Exhausted is `next + count`, decided before the CAS;
/// a last partial block saturates at `u64::MAX` when the width would
/// overflow but the draw still fits. `OverWidth` is not this
/// function's refusal.
///
/// # Errors
pub fn lease_block<S: ObjectStore>(
    store: &S,
    prefix: &str,
    relation: RelationId,
    field: FieldId,
    token: u64,
    count: u64,
) -> StoreResult<Leased> {
    let key = ids_key(prefix, relation, field);
    loop {
        let Some(fetched) = store.get(&key)? else {
            let birth = format!("{LEASE_WIDTH}");
            let (outcome, token) = put_create(store, &key, birth.as_bytes(), token)?;
            match outcome {
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
        // 10 §3: Exhausted is `next + count`, not the block width.
        if next.checked_add(count).is_none() {
            return Ok(Leased::Refused(LeaseRefusal::Exhausted { relation, field }));
        }
        let end = next.saturating_add(LEASE_WIDTH);
        let body = format!("{end}");
        let (outcome, token) = put_swap(store, &key, body.as_bytes(), &fetched.etag, token)?;
        match outcome {
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

/// Canonical decimal ASCII u64: the whole body, no leading zero unless
/// the value is zero. The counter is not a protocol document — 20 names
/// batch, manifest, checkpoint, sidecar — so this walk is digits, not
/// the document `Text` grammar that dies with JSON. Overflow is a
/// refusal (00 §6): a number the digits cannot name is unconstructible.
fn parse_counter(bytes: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;
    let mut len = 0usize;
    for &byte in bytes {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            _ => return None,
        };
        value = value.checked_mul(10)?.checked_add(u64::from(digit))?;
        len += 1;
    }
    if len == 0 || (len > 1 && bytes[0] == b'0') {
        return None;
    }
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
    /// unsigned. A cache hit is not a write; a miss writes under
    /// `self.token`.
    ///
    /// # Errors
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
        match lease_block(store, prefix, relation, field, self.token, count)? {
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
        let after_birth = store.get(&key).unwrap().unwrap();
        assert_eq!(after_birth.bytes, b"4096");
        let Leased::Drawn { range, token } = leases.draw(&store, "", REL, FIELD, 2).expect("cache")
        else {
            panic!("cache hit is Drawn");
        };
        assert_eq!(range, 3..5);
        assert_eq!(token, TOKEN);
        let after_hit = store.get(&key).unwrap().unwrap();
        assert_eq!(
            after_hit.bytes, after_birth.bytes,
            "cache-hit Drawn is not a write"
        );
        assert_eq!(after_hit.etag, after_birth.etag);

        store
            .put_swap(
                &key,
                Fenced {
                    bytes: b"18446744073709551615",
                    token: TOKEN,
                },
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
    fn exhausted_is_next_plus_count_not_the_block_width() {
        let store = MemStore::new();
        let mut birth = Leases::new(TOKEN);
        assert!(matches!(
            birth.draw(&store, "", REL, FIELD, 1).expect("birth"),
            Leased::Drawn { .. }
        ));
        let key = ids_key("", REL, FIELD);
        let next = u64::MAX - 10;
        store
            .put_swap(
                &key,
                Fenced {
                    bytes: next.to_string().as_bytes(),
                    token: TOKEN,
                },
                &store.get(&key).unwrap().unwrap().etag,
            )
            .expect("counter just below u64::MAX");

        let mut fits = Leases::new(TOKEN);
        let Leased::Drawn { range, token } = fits.draw(&store, "", REL, FIELD, 1).expect("fits")
        else {
            panic!("next + 1 fits: Drawn, not Exhausted-by-width");
        };
        assert_eq!(range, next..next + 1);
        assert_eq!(token, TOKEN);
        let after = store.get(&key).unwrap().unwrap();
        assert_eq!(
            after.bytes,
            u64::MAX.to_string().as_bytes(),
            "the last partial block saturates at u64::MAX"
        );

        let mut over = Leases::new(TOKEN);
        store
            .put_swap(
                &key,
                Fenced {
                    bytes: next.to_string().as_bytes(),
                    token: TOKEN,
                },
                &store.get(&key).unwrap().unwrap().etag,
            )
            .expect("restore next");
        assert_eq!(
            over.draw(&store, "", REL, FIELD, 11)
                .expect("count overflows"),
            Leased::Refused(LeaseRefusal::Exhausted {
                relation: REL,
                field: FIELD
            })
        );
        assert_eq!(
            store.get(&key).unwrap().unwrap().bytes,
            next.to_string().as_bytes(),
            "Exhausted does not mutate the counter"
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

    #[test]
    fn a_lower_token_loses_the_counter_cas() {
        let store = MemStore::new();
        let mut leases = Leases::new(TOKEN);
        assert!(matches!(
            leases.draw(&store, "", REL, FIELD, 1).expect("birth"),
            Leased::Drawn { .. }
        ));
        let key = ids_key("", REL, FIELD);
        let etag = store.get(&key).unwrap().unwrap().etag;
        assert_eq!(
            store
                .put_swap(
                    &key,
                    Fenced {
                        bytes: b"8192",
                        token: TOKEN - 1,
                    },
                    &etag,
                )
                .expect("stale fence"),
            Swap::Moved,
            "a stale holder's write is the token the CAS no longer wins"
        );
        assert_eq!(
            store.get(&key).unwrap().unwrap().bytes,
            b"4096",
            "the lower token did not mutate the counter"
        );
        assert_eq!(
            store
                .put_swap(
                    &key,
                    Fenced {
                        bytes: b"8192",
                        token: TOKEN,
                    },
                    &etag,
                )
                .expect("current fence"),
            Swap::Swapped(crate::store::fs::content_etag(b"8192")),
        );
        assert_eq!(store.get(&key).unwrap().unwrap().bytes, b"8192");
    }

    #[test]
    fn counter_is_canonical_decimal_digits() {
        assert_eq!(parse_counter(b"0"), Some(0));
        assert_eq!(parse_counter(b"4096"), Some(4096));
        assert_eq!(parse_counter(b"18446744073709551615"), Some(u64::MAX));
        assert_eq!(parse_counter(b""), None);
        assert_eq!(parse_counter(b"007"), None, "leading zero is not canonical");
        assert_eq!(parse_counter(b"4 096"), None);
        assert_eq!(parse_counter(b"4096\n"), None);
        assert_eq!(parse_counter(b"18446744073709551616"), None);
    }

    /// A CAS that lands and then reports Ambiguous. Every writer that
    /// read the same `next` writes the same increment body, so a GET
    /// that matches those bytes is not this writer's draw.
    struct HideFirstSwap {
        inner: MemStore,
        hidden: std::sync::atomic::AtomicBool,
    }

    impl crate::store::ObjectStore for HideFirstSwap {
        fn get(&self, key: &StoreKey) -> StoreResult<Option<crate::store::Fetched>> {
            self.inner.get(key)
        }

        fn get_if_changed(
            &self,
            key: &StoreKey,
            etag: &crate::store::Etag,
        ) -> StoreResult<crate::store::Poll> {
            self.inner.get_if_changed(key, etag)
        }

        fn put_create<'a>(
            &self,
            key: &StoreKey,
            body: impl Into<Fenced<'a>>,
        ) -> StoreResult<Create> {
            self.inner.put_create(key, body)
        }

        fn put_swap<'a>(
            &self,
            key: &StoreKey,
            body: impl Into<Fenced<'a>>,
            etag: &crate::store::Etag,
        ) -> StoreResult<Swap> {
            let outcome = self.inner.put_swap(key, body, etag)?;
            if matches!(outcome, Swap::Swapped(_))
                && !self.hidden.swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                return Ok(Swap::Ambiguous);
            }
            Ok(outcome)
        }

        fn delete(&self, key: &StoreKey) -> StoreResult<()> {
            self.inner.delete(key)
        }
    }

    #[test]
    fn an_ambiguous_increment_is_not_this_writers_block() {
        let store = HideFirstSwap {
            inner: MemStore::new(),
            hidden: std::sync::atomic::AtomicBool::new(false),
        };
        let birth = lease_block(&store, "", REL, FIELD, TOKEN, 1).expect("birth");
        let Leased::Drawn { range, .. } = birth else {
            panic!("birth is Drawn");
        };
        assert_eq!(range, 0..LEASE_WIDTH);
        let next = lease_block(&store, "", REL, FIELD, TOKEN, 1).expect("retry");
        let Leased::Drawn { range, .. } = next else {
            panic!("retry is Drawn");
        };
        assert_eq!(
            range,
            LEASE_WIDTH * 2..LEASE_WIDTH * 3,
            "Ambiguous is not a draw; the loop takes the next block"
        );
    }
}
