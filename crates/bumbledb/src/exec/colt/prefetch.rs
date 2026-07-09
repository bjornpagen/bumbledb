use super::{Colt, Cursor, NodeState};

impl Colt {
    /// Prefetches the bucket a hash will probe (phase 1.5): the
    /// bucket's ctrl
    /// group line, key block line, and child block line. A no-op
    /// for pinned rows and unforced nodes.
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
        // The children sub-block can sit on the bucket's second line
        // (stride 8·arity + 8 words = 128 B at arity 1).
        crate::exec::kernel::prefetch_read(&raw const self.buckets[base + 8 * m.arity]);
    }
}
