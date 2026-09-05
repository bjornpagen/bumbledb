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

    pub(super) fn active_matches(
        &self,
        occ: usize,
        epoch: ViewEpoch,
        filters: &[FilterPredicate],
    ) -> bool {
        match &self.occs[occ].active {
            Binding::Bound(bound) => bound.epoch == epoch && bound.filters == filters,
            Binding::Unbound | Binding::Derived => false,
        }
    }

    pub(super) fn set_bound(&mut self, occ: usize, epoch: ViewEpoch, filters: &[FilterPredicate]) {
        match &mut self.occs[occ].active {
            Binding::Bound(bound) => {
                bound.epoch = epoch;
                bound.filters.clear();
                bound.filters.extend_from_slice(filters);
                bound.last_used = self.tick;
            }
            Binding::Unbound | Binding::Derived => {
                self.occs[occ].active = Binding::Bound(Bound {
                    epoch,
                    filters: filters.to_vec(),
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
        {
            return true;
        }
        if let Some(slot) = occ_memo.parked.iter().position(|slot| {
            slot.as_ref().is_some_and(|parked| {
                parked.bound.epoch == epoch && parked.bound.filters == filters
            })
        }) {
            match &mut occ_memo.active {
                Binding::Derived => {
                    return false;
                }
                Binding::Bound(active) => {
                    let parked = occ_memo.parked[slot].as_mut().expect("matched Some above");
                    std::mem::swap(colt, &mut parked.colt);
                    std::mem::swap(active, &mut parked.bound);
                    parked.bound.last_used = tick;
                }
                Binding::Unbound => {
                    let parked = occ_memo.parked[slot].take().expect("matched Some above");
                    *colt = parked.colt;
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
                std::mem::swap(colt, &mut victim.colt);
                std::mem::swap(active, &mut victim.bound);
                victim.bound.last_used = tick;
            }
        }
        false
    }
}
