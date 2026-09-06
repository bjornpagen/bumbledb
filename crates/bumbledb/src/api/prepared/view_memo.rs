use super::{Binding, Bound, FilterPredicate, OccMemo, Parked, ViewMemo};
use crate::exec::colt::Colt;
use crate::image::ViewEpoch;

impl ViewMemo {
    pub(super) fn new() -> Self {
        Self {
            colts: Vec::new(),
            occs: Vec::new(),
            tick: 0,
        }
    }

    pub(super) fn push(&mut self, colt: Colt, active: Binding) {
        self.colts.push(colt);
        self.occs.push(OccMemo {
            active,
            parked: std::array::from_fn(|_| None),
            spare: Vec::new(),
        });
    }

    pub(super) fn spare_mut(&mut self, occ: usize) -> &mut Vec<u32> {
        &mut self.occs[occ].spare
    }

    /// Memory-pressure trim: drop parked bindings, active views and spare
    /// buffers. Derived occurrences return to `Derived`; store occurrences
    /// to `Unbound` — the next execution rebuilds what it needs.
    pub(super) fn trim(&mut self) {
        for (occ, memo) in self.occs.iter_mut().enumerate() {
            for slot in &mut memo.parked {
                *slot = None;
            }
            memo.spare = Vec::new();
            let _ = self.colts[occ].reset(crate::image::view::View::Unbound);
            if !matches!(memo.active, Binding::Derived) {
                memo.active = Binding::Unbound;
            }
        }
    }

    pub(super) fn is_derived(&self, occ: usize) -> bool {
        matches!(self.occs[occ].active, Binding::Derived)
    }

    /// A same-epoch miss with no free memo slot means selection diversity
    /// has exhausted bounded partial-image reuse. Promote once to a full
    /// image instead of allocating and rescanning on every LRU rotation.
    /// Called AFTER bind: a vacant slot leaves the active binding Unbound;
    /// only an eviction leaves a same-epoch partial victim here.
    pub(super) fn partial_capacity_exhausted(&self, occ: usize, epoch: ViewEpoch) -> bool {
        let memo = &self.occs[occ];
        matches!(&memo.active, Binding::Bound(bound)
            if bound.epoch == epoch && bound.selections.is_some())
            && memo.parked.iter().all(Option::is_some)
    }

    pub(super) fn active_matches(
        &self,
        occ: usize,
        epoch: ViewEpoch,
        filters: &[FilterPredicate],
    ) -> bool {
        match &self.occs[occ].active {
            Binding::Bound(bound) => {
                bound.epoch == epoch && bound.filters == filters && bound.selections.is_none()
            }
            Binding::Unbound | Binding::Derived => false,
        }
    }

    pub(super) fn set_bound(
        &mut self,
        occ: usize,
        epoch: ViewEpoch,
        filters: &[FilterPredicate],
        selections: Option<&[Vec<u64>]>,
    ) {
        match &mut self.occs[occ].active {
            Binding::Bound(bound) => {
                bound.epoch = epoch;
                bound.filters.clear();
                bound.filters.extend_from_slice(filters);
                match (selections, &mut bound.selections) {
                    (Some(keys), Some(bound)) => keys.clone_into(bound),
                    _ => bound.selections = selections.map(<[Vec<u64>]>::to_vec),
                }
                bound.last_used = self.tick;
            }
            Binding::Unbound | Binding::Derived => {
                self.occs[occ].active = Binding::Bound(Bound {
                    epoch,
                    filters: filters.to_vec(),
                    selections: selections.map(<[Vec<u64>]>::to_vec),
                    last_used: self.tick,
                });
            }
        }
    }

    pub(super) fn bind(
        &mut self,
        occ: usize,
        epoch: ViewEpoch,
        filters: &[FilterPredicate],
        selections: &[Vec<u64>],
    ) -> bool {
        let tick = self.tick;
        let colt = &mut self.colts[occ];
        let occ_memo = &mut self.occs[occ];

        // — drop it, its pools, and its image Arc. Closed and frozen

        for slot in &mut occ_memo.parked {
            if slot
                .as_ref()
                .is_some_and(|parked| parked.bound.epoch.superseded_by(epoch))
            {
                *slot = None;
            }
        }
        if let Binding::Bound(bound) = &occ_memo.active
            && bound.epoch == epoch
            && bound.filters == filters
            && bound
                .selections
                .as_deref()
                .is_none_or(|keys| keys == selections)
        {
            return true;
        }
        if let Some(slot) = occ_memo.parked.iter().position(|slot| {
            slot.as_ref().is_some_and(|parked| {
                parked.bound.epoch == epoch
                    && parked.bound.filters == filters
                    && parked
                        .bound
                        .selections
                        .as_deref()
                        .is_none_or(|keys| keys == selections)
            })
        }) {
            match &mut occ_memo.active {
                Binding::Derived => {
                    return false;
                }
                Binding::Bound(active) => {
                    let parked = occ_memo.parked[slot].as_mut().expect("matched Some above");
                    colt.swap_contents_preserving_work(&mut parked.colt);
                    std::mem::swap(active, &mut parked.bound);
                    parked.bound.last_used = tick;
                }
                Binding::Unbound => {
                    let mut parked = occ_memo.parked[slot].take().expect("matched Some above");
                    colt.swap_contents_preserving_work(&mut parked.colt);
                    occ_memo.active = Binding::Bound(parked.bound);
                }
            }
            return true;
        }

        if let Binding::Bound(bound) = &occ_memo.active
            && bound.epoch == epoch
        {
            if let Some(empty) = occ_memo.parked.iter().position(Option::is_none) {
                let Binding::Bound(bound) =
                    std::mem::replace(&mut occ_memo.active, Binding::Unbound)
                else {
                    unreachable!("just matched Bound");
                };
                let fresh = colt.unbound_sibling();
                occ_memo.parked[empty] = Some(Parked {
                    bound: Bound {
                        last_used: tick,
                        ..bound
                    },
                    colt: std::mem::replace(colt, fresh),
                });
            } else if let Some(victim) = occ_memo
                .parked
                .iter_mut()
                .flatten()
                .min_by_key(|parked| parked.bound.last_used)
            {
                let Binding::Bound(active) = &mut occ_memo.active else {
                    unreachable!("just matched Bound");
                };
                colt.swap_contents_preserving_work(&mut victim.colt);
                std::mem::swap(active, &mut victim.bound);
                victim.bound.last_used = tick;
            }
        }
        false
    }
}
