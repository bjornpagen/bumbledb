//! The successor tenant cache bookkeeping core (chapter 31, C09 consumer).
//!
//! One typed registry of per-tenant owner slots keyed by the digest of the
//! COMPLETE canonical binding. Every acquire mints a fresh one-shot
//! [`TenantBorrow`] bound to the exact slot incarnation; releasing twice is
//! harmless; a stale borrow can never decrement a successor slot. Active
//! work holds separately counted [`OperationLease`]s, so a zero borrow
//! count alone never permits teardown under an in-flight operation.
//!
//! This module owns bookkeeping ONLY: no wall-clock TTL, no lease renewal,
//! no `_shared` pinned magic tenant (explicitly held borrows express
//! pinning), no thread, no filesystem verb and no eviction timer. The
//! native runtime (ts/crate, C09) drives it: kernel directory locks,
//! executor scheduling and actual owner teardown stay native obligations,
//! and the registry hands owners OUT for teardown rather than dropping
//! them behind the caller's back. The 0.x renewable-TTL `Replica` LRU and
//! its disposable-directory eviction are deleted whole.

use std::collections::{BTreeMap, BTreeSet};

use crate::history::DatabaseIdentity;

/// The complete canonical cache binding: logical identity, physical
/// format/layout commitments and the explicitly configured authority
/// location. Matching schema and generation does not establish origin —
/// only exact equality of this whole record does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantBinding {
    pub identity: DatabaseIdentity,
    /// The store/codec layout the cache bytes were written with.
    pub layout: u16,
    /// The explicitly configured authority location (bucket/prefix or
    /// directory). A location change requires explicit remount; it is
    /// never inferred from matching content.
    pub location: Box<str>,
}

impl TenantBinding {
    /// The canonical binding bytes: fixed-width fields then the
    /// length-prefixed location, so no two distinct records share bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let location = self.location.as_bytes();
        let mut out = Vec::with_capacity(16 + 16 + 32 + 2 + 8 + location.len());
        out.extend_from_slice(self.identity.database_id.as_core().as_bytes());
        out.extend_from_slice(self.identity.incarnation_id.as_core().as_bytes());
        out.extend_from_slice(&self.identity.schema_id.0);
        out.extend_from_slice(&self.layout.to_be_bytes());
        out.extend_from_slice(&(location.len() as u64).to_be_bytes());
        out.extend_from_slice(location);
        out
    }

    /// The binding digest keying the slot and naming the local directory.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        *blake3::Hasher::new_derive_key("bumbledb.tenants.v1/binding")
            .update(&self.canonical_bytes())
            .finalize()
            .as_bytes()
    }

    /// The fixed-width lowercase local cache name. Human tenant labels are
    /// display metadata and never concatenated into cache paths; case
    /// folding and Unicode spellings cannot mint aliases of this name.
    #[must_use]
    pub fn local_name(&self) -> String {
        use std::fmt::Write as _;
        self.digest()
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

/// Registry capacity knobs. `max_open` bounds READY + OPENING slots (an
/// opening tenant is accounted before its asynchronous work starts).
#[derive(Debug, Clone, Copy)]
pub struct TenantOptions {
    pub max_open: usize,
}

/// One independently spent borrow token: the exact slot incarnation plus
/// this borrow's own id. Plain data — spending happens at the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantBorrow {
    key: [u8; 32],
    slot: u64,
    borrow: u64,
}

/// One counted in-flight operation lease. Ends exactly once through
/// [`TenantRegistry::end_operation`]; teardown waits for a zero count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationLease {
    key: [u8; 32],
    slot: u64,
    lease: u64,
}

/// A registered open attempt: installed before any asynchronous open work
/// begins, completed exactly once with the outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenTicket {
    key: [u8; 32],
    slot: u64,
}

/// Why an acquire refused. Typed and total — never a silent fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantRefusal {
    /// The slot's recorded binding is not byte-identical to the request's
    /// (including a digest collision): refuse before serving data.
    BindingMismatch,
    /// The registry is at `max_open` and nothing is evictable.
    Capacity,
    /// The slot is closing; a successor may open after teardown finishes.
    Closing,
    /// The slot faulted; recovery is an explicit close + reopen.
    Faulted,
}

/// The acquire outcome: a live borrow, a join on the one in-flight open,
/// a ticket obliging THIS caller to perform the open, or a refusal.
#[derive(Debug)]
pub enum Acquire {
    Ready(TenantBorrow),
    /// An open for this exact binding is already registered; the caller
    /// joins it (re-acquire after `complete_open` installs the slot).
    Joined {
        waiters: usize,
    },
    /// No slot existed: the registry recorded Opening under capacity and
    /// this caller must drive the actual open, then `complete_open`.
    Open(OpenTicket),
    Refused(TenantRefusal),
}

/// The completion outcome for an open ticket.
#[derive(Debug)]
pub enum CompletedOpen<O> {
    /// The owner is installed; the opener's own borrow is minted.
    Installed(TenantBorrow),
    /// Close arrived during the open: the slot is spent and the opened
    /// owner is handed BACK for teardown — it never installs, and no
    /// ready slot or timer appears in the closing epoch.
    ClosedDuringOpen(O),
}

/// A release result. All arms are harmless; none can touch another slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Released,
    /// This exact borrow was already spent — double release is a no-op.
    AlreadyReleased,
    /// The slot incarnation is gone (closed/evicted/reopened): a stale
    /// borrow cannot decrement a successor.
    StaleSlot,
}

/// Why `finish_close` cannot complete yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseBlocked {
    /// In-flight operation leases still hold the owner.
    Operations(usize),
    /// The slot is not closing / does not exist.
    NotClosing,
    /// The open has not completed; complete it first (it will observe the
    /// closing epoch and hand the owner back).
    StillOpening,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Ready,
    Closing,
    Faulted,
}

struct Slot<O> {
    id: u64,
    binding: TenantBinding,
    phase: Phase,
    owner: Option<O>,
    borrows: BTreeSet<u64>,
    leases: BTreeSet<u64>,
    waiters: usize,
    recency: u64,
}

/// One bounded slot inventory row (`TenantRegistry::report`).
#[derive(Debug, Clone)]
pub struct SlotReport {
    pub binding: TenantBinding,
    pub state: &'static str,
    pub borrows: usize,
    pub leases: usize,
}

/// The registry: one owner per binding digest, fresh borrows per acquire,
/// counted operations, joined opens/closes and explicit pressure.
pub struct TenantRegistry<O> {
    options: TenantOptions,
    next: u64,
    tick: u64,
    slots: BTreeMap<[u8; 32], Slot<O>>,
}

impl<O> TenantRegistry<O> {
    #[must_use]
    pub fn new(options: TenantOptions) -> Self {
        Self {
            options,
            next: 1,
            tick: 0,
            slots: BTreeMap::new(),
        }
    }

    fn mint(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.checked_add(1).expect("u64 id space");
        id
    }

    fn touch(&mut self, key: [u8; 32]) {
        self.tick += 1;
        let tick = self.tick;
        if let Some(slot) = self.slots.get_mut(&key) {
            slot.recency = tick;
        }
    }

    /// Open slot count (READY + OPENING + CLOSING + FAULTED — everything
    /// still holding registry accounting).
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.slots.len()
    }

    /// Acquires a borrow for `binding`, registering an open when absent.
    /// An exact-match live slot shares; ANY binding disagreement under the
    /// same digest refuses (`BindingMismatch`) before serving data.
    pub fn acquire(&mut self, binding: &TenantBinding) -> Acquire {
        let key = binding.digest();
        if let Some(slot) = self.slots.get_mut(&key) {
            if slot.binding != *binding {
                return Acquire::Refused(TenantRefusal::BindingMismatch);
            }
            return match slot.phase {
                Phase::Ready => {
                    let borrow = {
                        let id = self.mint();
                        let slot = self.slots.get_mut(&key).expect("slot just observed");
                        slot.borrows.insert(id);
                        TenantBorrow {
                            key,
                            slot: slot.id,
                            borrow: id,
                        }
                    };
                    self.touch(key);
                    Acquire::Ready(borrow)
                }
                Phase::Opening => {
                    slot.waiters += 1;
                    let waiters = slot.waiters;
                    Acquire::Joined { waiters }
                }
                Phase::Closing => Acquire::Refused(TenantRefusal::Closing),
                Phase::Faulted => Acquire::Refused(TenantRefusal::Faulted),
            };
        }
        if self.slots.len() >= self.options.max_open {
            return Acquire::Refused(TenantRefusal::Capacity);
        }
        let slot_id = self.mint();
        self.tick += 1;
        self.slots.insert(
            key,
            Slot {
                id: slot_id,
                binding: binding.clone(),
                phase: Phase::Opening,
                owner: None,
                borrows: BTreeSet::new(),
                leases: BTreeSet::new(),
                waiters: 0,
                recency: self.tick,
            },
        );
        Acquire::Open(OpenTicket { key, slot: slot_id })
    }

    /// Completes a registered open with the actual owner. If close arrived
    /// during the open, the slot is spent and the owner is handed back for
    /// teardown — it never installs into the closing epoch.
    ///
    /// # Panics
    /// A ticket for a slot the registry does not know is a caller logic
    /// error (tickets are completed exactly once).
    pub fn complete_open(&mut self, ticket: OpenTicket, owner: O) -> CompletedOpen<O> {
        let slot = self
            .slots
            .get_mut(&ticket.key)
            .filter(|slot| slot.id == ticket.slot)
            .expect("an open ticket is completed exactly once against its own slot");
        if slot.phase != Phase::Opening {
            // Close raced the open: spend the slot, hand the owner back.
            self.slots.remove(&ticket.key);
            return CompletedOpen::ClosedDuringOpen(owner);
        }
        slot.phase = Phase::Ready;
        slot.owner = Some(owner);
        slot.waiters = 0;
        let id = self.mint();
        let slot = self.slots.get_mut(&ticket.key).expect("slot just updated");
        slot.borrows.insert(id);
        let borrow = TenantBorrow {
            key: ticket.key,
            slot: ticket.slot,
            borrow: id,
        };
        self.touch(ticket.key);
        CompletedOpen::Installed(borrow)
    }

    /// Fails a registered open: the slot is removed (waiters re-acquire
    /// and observe the refusal from their own attempt).
    pub fn fail_open(&mut self, ticket: OpenTicket) {
        if self
            .slots
            .get(&ticket.key)
            .is_some_and(|slot| slot.id == ticket.slot && slot.owner.is_none())
        {
            self.slots.remove(&ticket.key);
        }
    }

    /// Releases one borrow. One-shot per borrow; double release and stale
    /// (post-reopen) release are harmless and cannot touch a successor.
    pub fn release(&mut self, borrow: TenantBorrow) -> Release {
        let Some(slot) = self
            .slots
            .get_mut(&borrow.key)
            .filter(|slot| slot.id == borrow.slot)
        else {
            return Release::StaleSlot;
        };
        if slot.borrows.remove(&borrow.borrow) {
            Release::Released
        } else {
            Release::AlreadyReleased
        }
    }

    /// Borrows the owner for one operation, taking a counted lease. The
    /// borrow must be live and the slot Ready (a closing/faulted slot
    /// refuses; eviction cannot start once a lease is held).
    pub fn begin_operation(&mut self, borrow: TenantBorrow) -> Option<(OperationLease, &O)> {
        let lease_id = self.mint();
        let slot = self
            .slots
            .get_mut(&borrow.key)
            .filter(|slot| slot.id == borrow.slot)?;
        if slot.phase != Phase::Ready || !slot.borrows.contains(&borrow.borrow) {
            return None;
        }
        slot.leases.insert(lease_id);
        let owner = slot.owner.as_ref()?;
        Some((
            OperationLease {
                key: borrow.key,
                slot: borrow.slot,
                lease: lease_id,
            },
            owner,
        ))
    }

    /// Ends one operation lease (exactly once; stale leases are no-ops).
    pub fn end_operation(&mut self, lease: OperationLease) {
        if let Some(slot) = self
            .slots
            .get_mut(&lease.key)
            .filter(|slot| slot.id == lease.slot)
        {
            slot.leases.remove(&lease.lease);
        }
    }

    /// Marks a slot faulted after its owner reported an unrecoverable
    /// error: new operations refuse; close + reopen is the recovery path.
    pub fn fault(&mut self, borrow: TenantBorrow) {
        if let Some(slot) = self
            .slots
            .get_mut(&borrow.key)
            .filter(|slot| slot.id == borrow.slot)
        {
            slot.phase = Phase::Faulted;
        }
    }

    /// Begins closing the slot for `binding`: revokes future borrows and
    /// idle capabilities (Ready → Closing), leaves in-flight operation
    /// leases counted. Idempotent; an Opening slot is marked so its
    /// completion tears down instead of installing.
    pub fn begin_close(&mut self, binding: &TenantBinding) -> bool {
        let key = binding.digest();
        let Some(slot) = self.slots.get_mut(&key) else {
            return false;
        };
        match slot.phase {
            Phase::Opening => {
                // The open completes into a spent slot (ClosedDuringOpen).
                slot.phase = Phase::Closing;
                true
            }
            Phase::Ready | Phase::Faulted => {
                slot.phase = Phase::Closing;
                slot.borrows.clear();
                true
            }
            Phase::Closing => true,
        }
    }

    /// Completes a close: removes the slot and hands the owner OUT for
    /// actual native teardown, only when no operation lease remains.
    /// Registry state never depends on a forgotten callback: the caller
    /// retries after its operations drain.
    ///
    /// # Errors
    /// `Operations(n)` while leases remain; `StillOpening` before the open
    /// completes; `NotClosing` when there is nothing to close.
    pub fn finish_close(&mut self, binding: &TenantBinding) -> Result<O, CloseBlocked> {
        let key = binding.digest();
        let Some(slot) = self.slots.get_mut(&key) else {
            return Err(CloseBlocked::NotClosing);
        };
        if slot.phase != Phase::Closing {
            return Err(CloseBlocked::NotClosing);
        }
        if slot.owner.is_none() {
            return Err(CloseBlocked::StillOpening);
        }
        if !slot.leases.is_empty() {
            return Err(CloseBlocked::Operations(slot.leases.len()));
        }
        let slot = self.slots.remove(&key).expect("slot observed above");
        Ok(slot.owner.expect("owner observed above"))
    }

    /// Live borrow/operation-lease counts for one binding's slot (`None`
    /// when no slot exists). Bookkeeping reads for the native cache's
    /// evict/inspect verbs — never a teardown decision by itself.
    #[must_use]
    pub fn counts(&self, binding: &TenantBinding) -> Option<(usize, usize)> {
        let slot = self.slots.get(&binding.digest())?;
        if slot.binding != *binding {
            return None;
        }
        Some((slot.borrows.len(), slot.leases.len()))
    }

    /// One bounded slot inventory row (identities and counters only —
    /// never payloads).
    #[must_use]
    pub fn report(&self) -> Vec<SlotReport> {
        self.slots
            .values()
            .map(|slot| SlotReport {
                binding: slot.binding.clone(),
                state: match slot.phase {
                    Phase::Opening => "opening",
                    Phase::Ready => "ready",
                    Phase::Closing => "closing",
                    Phase::Faulted => "faulted",
                },
                borrows: slot.borrows.len(),
                leases: slot.leases.len(),
            })
            .collect()
    }

    /// Pressure: selects unborrowed, un-leased READY slots least-recent
    /// first until at most `keep` slots remain, removing them and handing
    /// their owners out for teardown. Never evicts a slot with a live
    /// borrow or operation lease; never admits unlimited work by evicting
    /// pinned slots.
    pub fn evict_idle(&mut self, keep: usize) -> Vec<(TenantBinding, O)> {
        let mut evicted = Vec::new();
        while self.slots.len() > keep {
            let candidate = self
                .slots
                .iter()
                .filter(|(_, slot)| {
                    slot.phase == Phase::Ready && slot.borrows.is_empty() && slot.leases.is_empty()
                })
                .min_by_key(|(_, slot)| slot.recency)
                .map(|(key, _)| *key);
            let Some(key) = candidate else { break };
            let slot = self.slots.remove(&key).expect("candidate just observed");
            if let Some(owner) = slot.owner {
                evicted.push((slot.binding, owner));
            }
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{DatabaseId, IncarnationId};
    use bumbledb::Id128;

    fn binding(seed: u8, location: &str) -> TenantBinding {
        TenantBinding {
            identity: DatabaseIdentity {
                database_id: DatabaseId::from_core(Id128::from_bytes([seed; 16])),
                incarnation_id: IncarnationId::from_core(Id128::from_bytes([seed ^ 0xff; 16])),
                schema_id: bumbledb::SchemaFingerprint([seed.wrapping_add(1); 32]),
            },
            layout: 1,
            location: location.into(),
        }
    }

    fn registry(max_open: usize) -> TenantRegistry<&'static str> {
        TenantRegistry::new(TenantOptions { max_open })
    }

    fn open_ready(registry: &mut TenantRegistry<&'static str>, b: &TenantBinding) -> TenantBorrow {
        let Acquire::Open(ticket) = registry.acquire(b) else {
            panic!("fresh binding must yield an open ticket");
        };
        match registry.complete_open(ticket, "owner") {
            CompletedOpen::Installed(borrow) => borrow,
            CompletedOpen::ClosedDuringOpen(_) => panic!("no close raced"),
        }
    }

    #[test]
    fn local_names_are_fixed_width_lowercase_digests_not_labels() {
        let a = binding(1, "s3://bucket/tenant-a");
        let b = binding(1, "s3://bucket/tenant-A");
        assert_eq!(a.local_name().len(), 64);
        assert!(
            a.local_name()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        // A location spelling difference is a DIFFERENT binding: no case
        // folding can alias two authorities onto one cache name.
        assert_ne!(a.local_name(), b.local_name());
    }

    #[test]
    fn distinct_borrows_release_only_themselves() {
        let mut registry = registry(4);
        let b = binding(1, "s3://bucket/a");
        let first = open_ready(&mut registry, &b);
        let Acquire::Ready(second) = registry.acquire(&b) else {
            panic!("live slot shares by fresh borrow");
        };
        assert_ne!(first, second, "every acquire mints a fresh borrow");
        assert_eq!(registry.release(first), Release::Released);
        assert_eq!(
            registry.release(first),
            Release::AlreadyReleased,
            "double release is harmless"
        );
        // The second borrow is untouched: an operation still admits.
        let leased = registry.begin_operation(second);
        assert!(leased.is_some(), "sibling borrow survives the release");
    }

    #[test]
    fn stale_borrow_cannot_touch_a_successor_slot() {
        let mut registry = registry(4);
        let b = binding(2, "s3://bucket/b");
        let old = open_ready(&mut registry, &b);
        assert!(registry.begin_close(&b));
        registry.finish_close(&b).expect("no leases held");
        // Reopen: a NEW slot incarnation under the same binding digest.
        let fresh = open_ready(&mut registry, &b);
        assert_eq!(
            registry.release(old),
            Release::StaleSlot,
            "a stale borrow cannot decrement the successor"
        );
        assert!(registry.begin_operation(old).is_none());
        assert!(registry.begin_operation(fresh).is_some());
    }

    #[test]
    fn binding_mismatch_refuses_before_serving_data() {
        let mut registry = registry(4);
        let a = binding(3, "s3://bucket/c");
        let _ = open_ready(&mut registry, &a);
        // Same identity, different explicitly configured location: never
        // the same cache, even though a human label might match.
        let moved = binding(3, "s3://other-bucket/c");
        // Different location → different digest → a separate slot (not a
        // mismatch refusal, which needs a digest collision / same key).
        match registry.acquire(&moved) {
            Acquire::Open(ticket) => registry.fail_open(ticket),
            other => panic!("distinct binding opens its own slot, got {other:?}"),
        }
    }

    #[test]
    fn operation_lease_blocks_close_and_eviction_after_borrow_release() {
        let mut registry = registry(4);
        let b = binding(4, "s3://bucket/d");
        let borrow = open_ready(&mut registry, &b);
        let (lease, owner) = registry.begin_operation(borrow).expect("ready slot");
        assert_eq!(*owner, "owner");
        // Borrow released while the operation runs: zero borrows alone
        // must not permit teardown.
        assert_eq!(registry.release(borrow), Release::Released);
        assert!(
            registry.evict_idle(0).is_empty(),
            "leased slot never evicts"
        );
        assert!(registry.begin_close(&b));
        assert_eq!(
            registry.finish_close(&b),
            Err(CloseBlocked::Operations(1)),
            "close joins the in-flight operation"
        );
        registry.end_operation(lease);
        assert_eq!(registry.finish_close(&b), Ok("owner"));
        assert_eq!(registry.open_count(), 0);
    }

    #[test]
    fn close_during_open_never_installs_a_ready_slot() {
        let mut registry = registry(4);
        let b = binding(5, "s3://bucket/e");
        let Acquire::Open(ticket) = registry.acquire(&b) else {
            panic!("fresh binding opens");
        };
        assert!(
            registry.begin_close(&b),
            "closing the opening slot registers"
        );
        match registry.complete_open(ticket, "late-owner") {
            CompletedOpen::ClosedDuringOpen(owner) => {
                assert_eq!(owner, "late-owner", "the owner is handed back for teardown");
            }
            CompletedOpen::Installed(_) => panic!("a closing epoch must not install"),
        }
        assert_eq!(registry.open_count(), 0, "no live slot or timer remains");
    }

    #[test]
    fn opening_counts_toward_capacity_and_joiners_share_one_attempt() {
        let mut registry = registry(1);
        let b = binding(6, "s3://bucket/f");
        let Acquire::Open(ticket) = registry.acquire(&b) else {
            panic!("first opens");
        };
        // Same binding: joins the one attempt, no second open.
        assert!(matches!(
            registry.acquire(&b),
            Acquire::Joined { waiters: 1 }
        ));
        // Different binding: capacity includes the opening slot.
        assert!(matches!(
            registry.acquire(&binding(7, "s3://bucket/g")),
            Acquire::Refused(TenantRefusal::Capacity)
        ));
        let CompletedOpen::Installed(borrow) = registry.complete_open(ticket, "owner") else {
            panic!("open installs");
        };
        assert!(registry.begin_operation(borrow).is_some());
    }

    #[test]
    fn pressure_evicts_least_recent_unpinned_only() {
        let mut registry = registry(8);
        let a = binding(8, "s3://bucket/h");
        let b = binding(9, "s3://bucket/i");
        let c = binding(10, "s3://bucket/j");
        let borrow_a = open_ready(&mut registry, &a);
        let borrow_b = open_ready(&mut registry, &b);
        let borrow_c = open_ready(&mut registry, &c);
        // a is pinned by its live borrow; b and c release (idle).
        assert_eq!(registry.release(borrow_b), Release::Released);
        assert_eq!(registry.release(borrow_c), Release::Released);
        // Touch c so b is the least-recent idle slot.
        let Acquire::Ready(touch_c) = registry.acquire(&c) else {
            panic!("re-acquire touches recency");
        };
        assert_eq!(registry.release(touch_c), Release::Released);
        let evicted = registry.evict_idle(2);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].0, b, "least-recent idle slot evicts first");
        // The pinned slot never evicts even under keep=0.
        let evicted = registry.evict_idle(0);
        assert_eq!(evicted.len(), 1, "only the idle c remains evictable");
        assert_eq!(evicted[0].0, c);
        assert!(
            registry.begin_operation(borrow_a).is_some(),
            "pinned slot lives"
        );
    }

    #[test]
    fn faulted_slot_refuses_new_operations_until_explicit_close() {
        let mut registry = registry(4);
        let b = binding(11, "s3://bucket/k");
        let borrow = open_ready(&mut registry, &b);
        registry.fault(borrow);
        assert!(
            registry.begin_operation(borrow).is_none(),
            "faulted refuses"
        );
        assert!(matches!(
            registry.acquire(&b),
            Acquire::Refused(TenantRefusal::Faulted)
        ));
        assert!(registry.begin_close(&b));
        assert_eq!(registry.finish_close(&b), Ok("owner"));
    }
}
