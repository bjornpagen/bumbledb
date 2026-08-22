use super::{Colt, Cursor, NodeState};

impl Colt {

    #[inline(always)]
    pub fn prefetch_bucket(&self, cursor: Cursor, hash: u64) {
        let Cursor::Node(node) = cursor else { return };
        let NodeState::Forced { map } = self.nodes[node.0 as usize] else {
            return;
        };
        let m = &self.maps[map as usize];
        let b = usize::try_from(hash).expect("64-bit usize") & (m.nbuckets - 1);
        crate::exec::kernel::prefetch_read(&raw const self.ctrl[m.ctrl_start + b * 8]);
        let base = m.bucket_start + b * m.stride();
        crate::exec::kernel::prefetch_read(&raw const self.buckets[base]);

        crate::exec::kernel::prefetch_read(&raw const self.buckets[base + 8 * m.arity]);
    }
}
