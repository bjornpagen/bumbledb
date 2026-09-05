//! Exact scratch-backed text forward/reverse lookup with a charged,
//! bounded resident cache. Cache admits only after a successful write.

use std::collections::BTreeMap;

use super::{
    charge, entry_retained, work_error, ScratchAppend, ScratchCapability, ScratchMapId,
    ScratchProbe, ScratchRelation, ScratchVisitor,
};
use crate::error::Result;
use crate::work::{ByteReservation, WorkContext};

/// Resident cache bound: a hook, not a duplicate of the whole scratch map.
const TEXT_CACHE_BYTES: usize = 64 << 10;

/// Exact text↔token maps on one [`ScratchRelation`], plus a charged
/// bounded resident cache. Writes go through [`ScratchAppend`]; the cache
/// admits only after that visitor finishes. A failed append leaves no hit.
///
/// This is not TextEq: warm join equality stays on L04.
pub struct ScratchTextLookup {
    relation: ScratchRelation,
    cache: ChargedTextCache,
}

/// Charged bounded resident copies of committed forward/reverse entries.
struct ChargedTextCache {
    forward: BTreeMap<Box<[u8]>, Box<[u8]>>,
    reverse: BTreeMap<Box<[u8]>, Box<[u8]>>,
    charges: Vec<ByteReservation>,
    bytes: usize,
    charged: usize,
    limit: usize,
}

impl ChargedTextCache {
    fn new(limit: usize) -> Self {
        Self {
            forward: BTreeMap::new(),
            reverse: BTreeMap::new(),
            charges: Vec::new(),
            bytes: 0,
            charged: 0,
            limit,
        }
    }

    /// Admit both directions after a committed write. Over-bound or
    /// charge refusal skips the cache; scratch remains the source.
    fn admit(&mut self, work: &WorkContext, text: &[u8], token: &[u8]) {
        if self.forward.contains_key(text) {
            return;
        }
        let grown = entry_retained(text, token).saturating_add(entry_retained(token, text));
        if self.bytes.saturating_add(grown) > self.limit {
            return;
        }
        if charge(
            work,
            &mut self.bytes,
            &mut self.charged,
            &mut self.charges,
            grown,
        )
        .is_err()
        {
            return;
        }
        self.forward.insert(Box::from(text), Box::from(token));
        self.reverse.insert(Box::from(token), Box::from(text));
    }
}

impl ScratchTextLookup {
    /// Open forward/reverse on one relation under a live capability.
    /// Construct the capability with [`ScratchCapability::on_work`], not
    /// [`ScratchCapability::start`] on an already-running execute ledger.
    ///
    /// # Errors
    /// Named-map open failure.
    pub fn open(capability: &ScratchCapability) -> Result<Self> {
        let mut relation = capability.relation();
        relation.open_map(ScratchMapId::TextForward)?;
        relation.open_map(ScratchMapId::TextReverse)?;
        let limit = capability
            .policy()
            .ram_bytes_per_relation
            .min(TEXT_CACHE_BYTES);
        Ok(Self {
            relation,
            cache: ChargedTextCache::new(limit),
        })
    }

    /// Scoped forward borrow. Work, admission, and I/O are `Err`; miss is
    /// [`ScratchProbe::Miss`], never a silent false.
    ///
    /// # Errors
    /// As [`ScratchRelation::lookup`].
    pub fn lookup_forward<R>(
        &mut self,
        text: &[u8],
        visit: impl FnOnce(ScratchProbe<&[u8]>) -> Result<R>,
    ) -> Result<R> {
        if let Some(token) = self.cache.forward.get(text) {
            self.relation.work.step(1).map_err(work_error)?;
            return visit(ScratchProbe::Hit(token.as_ref()));
        }
        self.relation
            .lookup(ScratchMapId::TextForward, text, visit)
    }

    /// Scoped reverse borrow. Same error contract as [`Self::lookup_forward`].
    ///
    /// # Errors
    /// As [`ScratchRelation::lookup`].
    pub fn lookup_reverse<R>(
        &mut self,
        token: &[u8],
        visit: impl FnOnce(ScratchProbe<&[u8]>) -> Result<R>,
    ) -> Result<R> {
        if let Some(text) = self.cache.reverse.get(token) {
            self.relation.work.step(1).map_err(work_error)?;
            return visit(ScratchProbe::Hit(text.as_ref()));
        }
        self.relation
            .lookup(ScratchMapId::TextReverse, token, visit)
    }

    /// Forward: exact text bytes → token bytes. Cache hit only for a
    /// committed put. Failure is `Err`, not [`ScratchProbe::Miss`].
    ///
    /// # Errors
    /// As [`Self::lookup_forward`].
    pub fn get_forward(&mut self, text: &[u8], out: &mut Vec<u8>) -> Result<ScratchProbe<()>> {
        self.lookup_forward(text, |probe| Ok(copy_probe(probe, out)))
    }

    /// Reverse: exact token bytes → text bytes. Cache hit only for a
    /// committed put. Failure is `Err`, not [`ScratchProbe::Miss`].
    ///
    /// # Errors
    /// As [`Self::lookup_reverse`].
    pub fn get_reverse(&mut self, token: &[u8], out: &mut Vec<u8>) -> Result<ScratchProbe<()>> {
        self.lookup_reverse(token, |probe| Ok(copy_probe(probe, out)))
    }

    /// Write both directions through [`ScratchAppend`], then admit the
    /// cache. Failure aborts the uncommitted pair; neither side is cached.
    ///
    /// # Errors
    /// As [`ScratchAppend::append`] / [`ScratchAppend::finish`].
    pub fn put(&mut self, text: &[u8], token: &[u8]) -> Result<()> {
        if self
            .lookup_forward(text, |probe| Ok(probe.is_hit()))?
        {
            return Ok(());
        }
        {
            let mut append = ScratchAppend::new(&mut self.relation);
            append.append(ScratchMapId::TextForward, text, token)?;
            append.append(ScratchMapId::TextReverse, token, text)?;
            append.finish()?;
        }
        self.cache
            .admit(&self.relation.work, text, token);
        Ok(())
    }

    /// Visit committed forward entries (authoritative map, not cache-only).
    ///
    /// # Errors
    /// As [`ScratchRelation::visit_map`].
    pub fn visit_forward(&mut self, visitor: &mut impl ScratchVisitor) -> Result<()> {
        self.relation
            .visit_map(ScratchMapId::TextForward, visitor)
    }

    /// Visit committed reverse entries (authoritative map, not cache-only).
    ///
    /// # Errors
    /// As [`ScratchRelation::visit_map`].
    pub fn visit_reverse(&mut self, visitor: &mut impl ScratchVisitor) -> Result<()> {
        self.relation
            .visit_map(ScratchMapId::TextReverse, visitor)
    }
}

fn copy_probe(probe: ScratchProbe<&[u8]>, out: &mut Vec<u8>) -> ScratchProbe<()> {
    out.clear();
    match probe {
        ScratchProbe::Hit(bytes) => {
            out.extend_from_slice(bytes);
            ScratchProbe::Hit(())
        }
        ScratchProbe::Miss => ScratchProbe::Miss,
    }
}
