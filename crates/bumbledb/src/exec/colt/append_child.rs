use super::{
    CHUNK_LEN, Chunk, Colt, NodeRef, NodeState, Positions, Slot, pack_child, unpack_child,
};

impl Colt {
    /// Reserves one chunk frame of `cap` positions at the slab tail,
    /// returning its index.
    fn alloc_chunk(&mut self, cap: usize) -> u32 {
        let idx = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
        let start = u32::try_from(self.chunk_positions.len()).expect("position slab fits u32");
        self.chunk_positions
            .resize(self.chunk_positions.len() + cap, 0);
        self.chunks.push(Chunk {
            start,
            cap: u8::try_from(cap).expect("chunk capacities fit u8"),
            len: 0,
            next: u32::MAX,
        });
        idx
    }

    /// Appends a position to an occupied slot's child: singleton inline
    /// first, a chunked node from the second position on — the first
    /// chunk small ([`super::FIRST_CHUNK_CAP`] — the graded geometry:
    /// small fanouts fit whole), later chunks full [`CHUNK_LEN`].
    /// `child_at` indexes the bucket slab's packed child word.
    pub(super) fn append_child(&mut self, child_at: usize, position: u32) {
        match unpack_child(self.buckets[child_at]) {
            Slot::Single(first_position) => {
                // Second position: allocate the chunked child node now.
                let chunk_idx = self.alloc_chunk(usize::from(self.first_chunk_cap));
                let c = self.chunks[chunk_idx as usize];
                self.chunk_positions[c.start as usize] = first_position;
                self.chunk_positions[c.start as usize + 1] = position;
                self.chunks[chunk_idx as usize].len = 2;
                let node_ref =
                    NodeRef(u32::try_from(self.nodes.len()).expect("node count fits u32"));
                self.nodes.push(NodeState::Unforced(Positions::Chunks {
                    first: chunk_idx,
                    last: chunk_idx,
                    count: 2,
                }));
                self.buckets[child_at] = pack_child(Slot::Node(node_ref));
            }
            Slot::Node(node_ref) => {
                let NodeState::Unforced(Positions::Chunks { first, last, count }) =
                    self.nodes[node_ref.0 as usize]
                else {
                    unreachable!("chunked children stay unforced during their parent's force");
                };
                let last_chunk = self.chunks[last as usize];
                if last_chunk.len < last_chunk.cap {
                    self.chunk_positions
                        [last_chunk.start as usize + usize::from(last_chunk.len)] = position;
                    self.chunks[last as usize].len += 1;
                    self.nodes[node_ref.0 as usize] = NodeState::Unforced(Positions::Chunks {
                        first,
                        last,
                        count: count + 1,
                    });
                } else {
                    let new_idx = self.alloc_chunk(CHUNK_LEN);
                    let c = self.chunks[new_idx as usize];
                    self.chunk_positions[c.start as usize] = position;
                    self.chunks[new_idx as usize].len = 1;
                    self.chunks[last as usize].next = new_idx;
                    self.nodes[node_ref.0 as usize] = NodeState::Unforced(Positions::Chunks {
                        first,
                        last: new_idx,
                        count: count + 1,
                    });
                }
            }
        }
    }
}
