use super::{
    pack_child, unpack_child, Chunk, Colt, NodeRef, NodeState, Positions, Slot, CHUNK_LEN,
};

impl Colt {
    /// Appends a position to an occupied slot's child: singleton inline
    /// first, a chunked node from the second position on. `child_at`
    /// indexes the bucket slab's packed child word.
    pub(super) fn append_child(&mut self, child_at: usize, position: u32) {
        match unpack_child(self.buckets[child_at]) {
            Slot::Single(first_position) => {
                // Second position: allocate the chunked child node now.
                let chunk_idx = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                let mut chunk = Chunk {
                    positions: [0; CHUNK_LEN],
                    len: 2,
                    next: u32::MAX,
                };
                chunk.positions[0] = first_position;
                chunk.positions[1] = position;
                self.chunks.push(chunk);
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
                let last_chunk = &mut self.chunks[last as usize];
                if usize::from(last_chunk.len) < CHUNK_LEN {
                    last_chunk.positions[usize::from(last_chunk.len)] = position;
                    last_chunk.len += 1;
                    self.nodes[node_ref.0 as usize] = NodeState::Unforced(Positions::Chunks {
                        first,
                        last,
                        count: count + 1,
                    });
                } else {
                    let new_idx = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                    let mut chunk = Chunk {
                        positions: [0; CHUNK_LEN],
                        len: 1,
                        next: u32::MAX,
                    };
                    chunk.positions[0] = position;
                    self.chunks.push(chunk);
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
