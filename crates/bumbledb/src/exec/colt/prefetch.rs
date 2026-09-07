use super::{Colt, Cursor, Map, NodeState};

impl Colt {
    #[inline(always)]
    pub(crate) fn prefetch_bucket_with_children<const CHILDREN: bool>(
        &self,
        cursor: Cursor,
        hash: u64,
    ) {
        let Cursor::Node(node) = cursor else { return };
        let NodeState::Forced { map } = self.nodes[node.0 as usize] else {
            return;
        };
        let m = &self.maps[map as usize];
        self.prefetch_map::<CHILDREN>(m, hash);
    }

    #[inline(always)]
    pub(super) fn prefetch_map<const CHILDREN: bool>(&self, m: &Map, hash: u64) {
        let b = usize::try_from(hash).expect("64-bit usize") & (m.nbuckets - 1);
        crate::exec::kernel::prefetch_read(&raw const self.ctrl[m.ctrl_start + b * 8]);
        let base = m.bucket_start + b * m.stride();
        crate::exec::kernel::prefetch_read(&raw const self.buckets[base]);

        if CHILDREN {
            crate::exec::kernel::prefetch_read(&raw const self.buckets[base + 8 * m.arity]);
        }
    }
}
