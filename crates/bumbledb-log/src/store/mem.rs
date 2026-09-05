//! `MemStore`: the conditional-store verbs over one in-process map, with a
//! deterministic fault script. The mutex proves every outcome, so ambiguity
//! never arises spontaneously — it is *injected*, exactly where a schedule
//! demands it, including the "applied but unacknowledged" arm a real
//! transport produces. Tests and deterministic schedules only; emulator
//! green is not S3 qualification (C07).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use crate::writer::verbs::{
    ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion, ListPage, ObjectRead, PutOutcome,
};

/// Which verb a scripted fault applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    ReadHead,
    CreateHead,
    ReplaceHead,
    PutObject,
    GetObject,
    ListObjects,
    DeleteObject,
}

/// One scripted behavior for the next matching operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    /// Fail with a transport error before any state change.
    Error,
    /// Apply the mutation, then report an indeterminate/failed outcome —
    /// the "response lost after the request landed" arm.
    IndeterminateApplied,
    /// Drop the mutation and report indeterminate — the "request never
    /// arrived" arm. For reads this behaves like `Error`.
    IndeterminateDropped,
}

/// The backend's transport failure. Carries the op for schedule assertions.
#[derive(Debug)]
pub struct MemFault {
    pub op: Op,
    pub key: String,
}

impl fmt::Display for MemFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "injected transport fault: {:?} on {}", self.op, self.key)
    }
}

impl std::error::Error for MemFault {}

/// A deterministic pause point for interleaved schedules: an operation the
/// gate hook selects blocks here (outside every store lock) until `open`.
pub struct Gate {
    opened: Mutex<bool>,
    signal: Condvar,
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate {
    #[must_use]
    pub fn new() -> Self {
        Self {
            opened: Mutex::new(false),
            signal: Condvar::new(),
        }
    }

    pub fn open(&self) {
        let mut opened = self.opened.lock().unwrap_or_else(PoisonError::into_inner);
        *opened = true;
        self.signal.notify_all();
    }

    fn wait(&self) {
        let mut opened = self.opened.lock().unwrap_or_else(PoisonError::into_inner);
        while !*opened {
            opened = self
                .signal
                .wait(opened)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Decide, quickly and without blocking, whether THIS call pauses; the
/// returned gate is waited on outside the hook lock and outside the state
/// lock, so other callers proceed while one is paused.
type GateHook = Box<dyn FnMut(Op, &str) -> Option<Arc<Gate>> + Send>;

struct Head {
    generation: u64,
    body: Vec<u8>,
}

#[derive(Default)]
struct State {
    heads: BTreeMap<String, Head>,
    objects: BTreeMap<String, Vec<u8>>,
    script: Vec<(Op, Behavior)>,
    log: Vec<(Op, String)>,
}

/// Deterministic conditional store over one `BTreeMap`, single process.
/// Version tokens are monotone per-head generations; listing is exact and
/// ordered with a configurable page size.
pub struct MemStore {
    state: Mutex<State>,
    gate: Mutex<Option<GateHook>>,
    page_size: usize,
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            gate: Mutex::new(None),
            page_size: 1_000,
        }
    }

    /// A store whose listings return at most `page_size` keys per page —
    /// pagination schedules (GC-09, S3-05 shapes) use small pages.
    #[must_use]
    pub fn with_page_size(page_size: usize) -> Self {
        Self {
            state: Mutex::new(State::default()),
            gate: Mutex::new(None),
            page_size: page_size.max(1),
        }
    }

    /// Install a deterministic scheduling hook: for each operation it may
    /// return a [`Gate`] this call must wait on. The wait happens outside
    /// every lock, so concurrent callers proceed while one is paused — the
    /// SIGSTOP-shaped pause a schedule injects exactly where it wants it.
    pub fn set_gate(&self, hook: impl FnMut(Op, &str) -> Option<Arc<Gate>> + Send + 'static) {
        *self.gate.lock().unwrap_or_else(PoisonError::into_inner) = Some(Box::new(hook));
    }

    fn consult_gate(&self, op: Op, key: &str) {
        let gate = {
            let mut slot = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
            slot.as_mut().and_then(|hook| hook(op, key))
        };
        if let Some(gate) = gate {
            gate.wait();
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queue one scripted fault for the next matching operation. Faults are
    /// consumed first-match-first-served per op kind.
    pub fn fail_next(&self, op: Op, behavior: Behavior) {
        self.lock().script.push((op, behavior));
    }

    /// Every operation observed so far, in order, for schedule assertions.
    #[must_use]
    pub fn operations(&self) -> Vec<(Op, String)> {
        self.lock().log.clone()
    }

    /// Direct inspection: current object keys (tests assert exact sets).
    #[must_use]
    pub fn object_keys(&self) -> Vec<String> {
        self.lock().objects.keys().cloned().collect()
    }

    /// Hostile-schedule support: corrupt one stored object's bytes in place.
    /// Returns whether the key existed.
    pub fn corrupt_object(&self, key: &str, mutate: impl FnOnce(&mut Vec<u8>)) -> bool {
        let mut state = self.lock();
        match state.objects.get_mut(key) {
            Some(bytes) => {
                mutate(bytes);
                true
            }
            None => false,
        }
    }

    /// Hostile-schedule support: overwrite a head body without moving its
    /// version token (a storage-level corruption, not a protocol write).
    pub fn corrupt_head(&self, key: &str, mutate: impl FnOnce(&mut Vec<u8>)) -> bool {
        let mut state = self.lock();
        match state.heads.get_mut(key) {
            Some(head) => {
                mutate(&mut head.body);
                true
            }
            None => false,
        }
    }

    fn take_fault(state: &mut State, op: Op) -> Option<Behavior> {
        let index = state
            .script
            .iter()
            .position(|(fault_op, _)| *fault_op == op)?;
        Some(state.script.remove(index).1)
    }

    fn observe(state: &mut State, op: Op, key: &str) {
        state.log.push((op, key.to_string()));
    }
}

fn version(generation: u64) -> HeadVersion {
    HeadVersion(Box::from(generation.to_be_bytes()))
}

impl ConditionalStore for MemStore {
    type Error = MemFault;

    fn read_head(&self, head_key: &str) -> Result<HeadRead, MemFault> {
        self.consult_gate(Op::ReadHead, head_key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::ReadHead, head_key);
        if Self::take_fault(&mut state, Op::ReadHead).is_some() {
            return Err(MemFault {
                op: Op::ReadHead,
                key: head_key.to_string(),
            });
        }
        Ok(match state.heads.get(head_key) {
            Some(head) => HeadRead::Present {
                version: version(head.generation),
                body: Box::from(head.body.as_slice()),
            },
            None => HeadRead::Absent,
        })
    }

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, MemFault> {
        self.consult_gate(Op::CreateHead, head_key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::CreateHead, head_key);
        let fault = Self::take_fault(&mut state, Op::CreateHead);
        match fault {
            Some(Behavior::Error) => {
                return Err(MemFault {
                    op: Op::CreateHead,
                    key: head_key.to_string(),
                });
            }
            Some(Behavior::IndeterminateDropped) => {
                return Ok(ConditionalOutcome::Indeterminate);
            }
            _ => {}
        }
        if state.heads.contains_key(head_key) {
            return Ok(ConditionalOutcome::PreconditionFailed);
        }
        state.heads.insert(
            head_key.to_string(),
            Head {
                generation: 1,
                body: body.to_vec(),
            },
        );
        if fault == Some(Behavior::IndeterminateApplied) {
            return Ok(ConditionalOutcome::Indeterminate);
        }
        Ok(ConditionalOutcome::Published {
            version: version(1),
        })
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, MemFault> {
        self.consult_gate(Op::ReplaceHead, head_key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::ReplaceHead, head_key);
        let fault = Self::take_fault(&mut state, Op::ReplaceHead);
        match fault {
            Some(Behavior::Error) => {
                return Err(MemFault {
                    op: Op::ReplaceHead,
                    key: head_key.to_string(),
                });
            }
            Some(Behavior::IndeterminateDropped) => {
                return Ok(ConditionalOutcome::Indeterminate);
            }
            _ => {}
        }
        let Some(head) = state.heads.get_mut(head_key) else {
            return Ok(ConditionalOutcome::PreconditionFailed);
        };
        if version(head.generation) != *expected {
            return Ok(ConditionalOutcome::PreconditionFailed);
        }
        head.generation += 1;
        head.body = body.to_vec();
        let generation = head.generation;
        if fault == Some(Behavior::IndeterminateApplied) {
            return Ok(ConditionalOutcome::Indeterminate);
        }
        Ok(ConditionalOutcome::Published {
            version: version(generation),
        })
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, MemFault> {
        self.consult_gate(Op::PutObject, key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::PutObject, key);
        let fault = Self::take_fault(&mut state, Op::PutObject);
        match fault {
            Some(Behavior::Error) => {
                return Err(MemFault {
                    op: Op::PutObject,
                    key: key.to_string(),
                });
            }
            Some(Behavior::IndeterminateDropped) => return Ok(PutOutcome::Indeterminate),
            _ => {}
        }
        // Immutable names: identical bytes are idempotent, conflicting bytes
        // refuse — creation never overwrites a colliding payload (chapter 41),
        // exactly like the filesystem adapter.
        if let Some(existing) = state.objects.get(key)
            && existing != body
        {
            return Err(MemFault {
                op: Op::PutObject,
                key: key.to_string(),
            });
        }
        state.objects.insert(key.to_string(), body.to_vec());
        if fault == Some(Behavior::IndeterminateApplied) {
            return Ok(PutOutcome::Indeterminate);
        }
        Ok(PutOutcome::Stored)
    }

    fn get_object(&self, key: &str) -> Result<ObjectRead, MemFault> {
        self.consult_gate(Op::GetObject, key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::GetObject, key);
        if Self::take_fault(&mut state, Op::GetObject).is_some() {
            return Err(MemFault {
                op: Op::GetObject,
                key: key.to_string(),
            });
        }
        Ok(match state.objects.get(key) {
            Some(bytes) => ObjectRead::Present {
                body: Box::from(bytes.as_slice()),
            },
            None => ObjectRead::Absent,
        })
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, MemFault> {
        self.consult_gate(Op::ListObjects, prefix);
        let mut state = self.lock();
        Self::observe(&mut state, Op::ListObjects, prefix);
        if Self::take_fault(&mut state, Op::ListObjects).is_some() {
            return Err(MemFault {
                op: Op::ListObjects,
                key: prefix.to_string(),
            });
        }
        let resume = after
            .map(|token| String::from_utf8_lossy(token).into_owned())
            .unwrap_or_default();
        let keys: Vec<String> = state
            .objects
            .keys()
            .filter(|key| key.starts_with(prefix) && key.as_str() > resume.as_str())
            .take(self.page_size)
            .cloned()
            .collect();
        let next = if keys.len() == self.page_size {
            keys.last().map(|last| Box::from(last.as_bytes()))
        } else {
            None
        };
        Ok(ListPage { keys, next })
    }

    fn delete_object(&self, key: &str) -> Result<(), MemFault> {
        self.consult_gate(Op::DeleteObject, key);
        let mut state = self.lock();
        Self::observe(&mut state, Op::DeleteObject, key);
        let fault = Self::take_fault(&mut state, Op::DeleteObject);
        if let Some(Behavior::Error | Behavior::IndeterminateDropped) = fault {
            return Err(MemFault {
                op: Op::DeleteObject,
                key: key.to_string(),
            });
        }
        state.objects.remove(key);
        if fault == Some(Behavior::IndeterminateApplied) {
            // The delete landed but the response was lost.
            return Err(MemFault {
                op: Op::DeleteObject,
                key: key.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_grammar_is_exact_and_versions_are_monotone() {
        let store = MemStore::new();
        assert_eq!(store.read_head("p/HEAD").unwrap(), HeadRead::Absent);
        let v1 = match store.create_head("p/HEAD", b"one").unwrap() {
            ConditionalOutcome::Published { version } => version,
            other => panic!("create publishes: {other:?}"),
        };
        assert_eq!(
            store.create_head("p/HEAD", b"two").unwrap(),
            ConditionalOutcome::PreconditionFailed,
            "a never-reused head is never re-created over"
        );
        let v2 = match store.replace_head("p/HEAD", &v1, b"two").unwrap() {
            ConditionalOutcome::Published { version } => version,
            other => panic!("exact swap publishes: {other:?}"),
        };
        assert_ne!(v1, v2);
        assert_eq!(
            store.replace_head("p/HEAD", &v1, b"three").unwrap(),
            ConditionalOutcome::PreconditionFailed,
            "a stale version cannot win after the head moved"
        );
    }

    #[test]
    fn injected_ambiguity_distinguishes_applied_from_dropped() {
        let store = MemStore::new();
        let v1 = match store.create_head("p/HEAD", b"one").unwrap() {
            ConditionalOutcome::Published { version } => version,
            other => panic!("{other:?}"),
        };
        store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
        assert_eq!(
            store.replace_head("p/HEAD", &v1, b"two").unwrap(),
            ConditionalOutcome::Indeterminate
        );
        match store.read_head("p/HEAD").unwrap() {
            HeadRead::Present { body, .. } => assert_eq!(&*body, b"two", "the CAS landed"),
            HeadRead::Absent => panic!("head exists"),
        }
        store.fail_next(Op::PutObject, Behavior::IndeterminateDropped);
        assert_eq!(
            store.put_object("p/objects/1/chunk/aa", b"x").unwrap(),
            PutOutcome::Indeterminate
        );
        assert_eq!(
            store.get_object("p/objects/1/chunk/aa").unwrap(),
            ObjectRead::Absent,
            "the dropped request never arrived"
        );
    }

    #[test]
    fn listing_pages_are_bounded_ordered_and_resumable() {
        let store = MemStore::with_page_size(2);
        for name in [
            "p/objects/1/chunk/aa",
            "p/objects/1/chunk/bb",
            "p/objects/2/chunk/cc",
        ] {
            assert_eq!(store.put_object(name, b"x").unwrap(), PutOutcome::Stored);
        }
        let first = store.list_objects("p/objects/", None).unwrap();
        assert_eq!(first.keys.len(), 2);
        let next = first.next.expect("continuation");
        let second = store.list_objects("p/objects/", Some(&next)).unwrap();
        assert_eq!(second.keys, vec!["p/objects/2/chunk/cc".to_string()]);
    }
}
