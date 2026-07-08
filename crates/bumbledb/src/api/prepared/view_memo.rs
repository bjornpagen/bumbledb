use super::{FilterPredicate, ParkedView, ViewMemo};

impl ViewMemo {
    /// Binds `occ`'s active slot to `(generation, filters)`: an active
    /// hit is free, a parked hit swaps in, and a miss parks the active
    /// binding (into an empty slot first, else the LRU victim) and
    /// reports `false` so the caller rebuilds in place.
    pub(super) fn bind(
        &mut self,
        occ: usize,
        generation: u64,
        filters: &[FilterPredicate],
    ) -> bool {
        // Stale reaping first: generations only advance, so a parked
        // binding below this one is provably unhittable — drop it, its
        // pools, and its image Arc. Fires only when the generation moved
        // (within a generation every parked entry is current), so the
        // zero-alloc/zero-dealloc discipline of the warm window holds.
        for slot in &mut self.parked[occ] {
            if slot
                .as_ref()
                .is_some_and(|parked| parked.generation < generation)
            {
                *slot = None;
            }
        }
        if self.generation[occ] == Some(generation) && self.filters[occ] == filters {
            return true;
        }
        if let Some(slot) = self.parked[occ].iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|parked| parked.generation == generation && parked.filters == filters)
        }) {
            let parked = self.parked[occ][slot].as_mut().expect("matched Some above");
            std::mem::swap(&mut self.colts[occ], &mut parked.colt);
            std::mem::swap(&mut self.filters[occ], &mut parked.filters);
            // A parked entry exists only after a same-generation park, so
            // the outgoing active binding is bound (post-reap both sides
            // are at `generation`; the swap just rotates which is active).
            let outgoing = self.generation[occ]
                .replace(parked.generation)
                .expect("a parked hit implies an executed active binding");
            parked.generation = outgoing;
            parked.last_used = self.tick;
            return true;
        }
        // A current-generation active binding is still hittable — park it
        // into an empty slot (first park constructs the ParkedView inside
        // the sanctioned view-rebuild window), else over the LRU victim
        // (post-reap every survivor is current-generation, so LRU is the
        // whole policy). A stale or unbound active can never hit again:
        // rebuild it in place (zero-residual occurrences always land
        // here, so their parked slots stay empty forever).
        if self.generation[occ] == Some(generation) {
            if let Some(empty) = self.parked[occ].iter().position(Option::is_none) {
                let fresh = self.colts[occ].unbound_sibling();
                self.parked[occ][empty] = Some(ParkedView {
                    generation,
                    filters: std::mem::take(&mut self.filters[occ]),
                    colt: std::mem::replace(&mut self.colts[occ], fresh),
                    last_used: self.tick,
                });
                self.generation[occ] = None;
            } else if let Some(victim) = self.parked[occ]
                .iter_mut()
                .flatten()
                .min_by_key(|parked| parked.last_used)
            {
                std::mem::swap(&mut self.colts[occ], &mut victim.colt);
                std::mem::swap(&mut self.filters[occ], &mut victim.filters);
                victim.generation = generation;
                victim.last_used = self.tick;
            }
        }
        false
    }
}
