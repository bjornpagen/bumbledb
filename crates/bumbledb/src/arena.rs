//! A minimal bump arena over `Vec<u8>` chunks. Its one consumer is the
//! write delta (fact bytes accumulate here and free as a whole at commit
//! or abort); the executor's scratch is retained-capacity `Vec` pools,
//! not this type. No external crate, no `unsafe`:
//! allocations hand out index-based [`ArenaSlice`] handles, never pointers,
//! so chunk storage may move without invalidating anything.

/// Default chunk capacity; oversized allocations get their own chunk.
const CHUNK_CAPACITY: usize = 64 * 1024;

/// An index-based handle to bytes stored in an [`Arena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaSlice {
    chunk: u32,
    start: u32,
    len: u32,
}

/// Bump allocator: bytes go in, handles come out; freeing is dropping (or
/// resetting) the arena as a whole. No per-value heap objects.
#[derive(Debug, Default)]
pub struct Arena {
    chunks: Vec<Vec<u8>>,
    /// The open chunk ordinary allocations bump into — an explicit
    /// index, not "the last chunk": an oversized spill pushes its own
    /// exactly-sized chunk past it without advancing it, so the open
    /// chunk's free tail stays live instead of stranding.
    active: usize,
}

impl Arena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies `bytes` into the arena, returning its handle.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: a single allocation or
    /// chunk offset exceeding `u32::MAX` (the scale axiom keeps every
    /// allocation orders of magnitude below that).
    pub fn alloc(&mut self, bytes: &[u8]) -> ArenaSlice {
        let chunk_idx = if bytes.len() > CHUNK_CAPACITY {
            // A dedicated exactly-sized chunk, pushed past the active
            // one — which keeps its free tail for the next ordinary
            // allocation.
            self.chunks.push(Vec::with_capacity(bytes.len()));
            self.chunks.len() - 1
        } else {
            let needs_new_chunk = match self.chunks.get(self.active) {
                Some(chunk) => chunk.len() + bytes.len() > chunk.capacity(),
                None => true,
            };
            if needs_new_chunk {
                self.chunks.push(Vec::with_capacity(CHUNK_CAPACITY));
                self.active = self.chunks.len() - 1;
            }
            self.active
        };
        let chunk = &mut self.chunks[chunk_idx];
        let start = chunk.len();
        chunk.extend_from_slice(bytes);
        ArenaSlice {
            chunk: u32::try_from(chunk_idx).expect("chunk count fits u32"),
            start: u32::try_from(start).expect("chunk offset fits u32"),
            len: u32::try_from(bytes.len()).expect("allocation length fits u32"),
        }
    }

    /// Resolves a handle back to its bytes.
    #[must_use]
    pub fn get(&self, slice: ArenaSlice) -> &[u8] {
        let chunk = &self.chunks[slice.chunk as usize];
        &chunk[slice.start as usize..(slice.start + slice.len) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_bytes() {
        let mut arena = Arena::new();
        let a = arena.alloc(b"hello");
        let b = arena.alloc(b"");
        let c = arena.alloc(&[0xFF; 100]);
        assert_eq!(arena.get(a), b"hello");
        assert_eq!(arena.get(b), b"");
        assert_eq!(arena.get(c), &[0xFF; 100]);
        // Earlier handles stay valid after later allocations.
        for _ in 0..1000 {
            arena.alloc(b"filler");
        }
        assert_eq!(arena.get(a), b"hello");
    }

    #[test]
    fn spills_into_new_chunks_without_moving_old_bytes() {
        let mut arena = Arena::new();
        let (a, b, c) = (
            vec![1u8; 40 * 1024],
            vec![2u8; 40 * 1024],  // exceeds the first chunk
            vec![3u8; 200 * 1024], // its own chunk
        );
        let first = arena.alloc(&a);
        let second = arena.alloc(&b);
        let oversized = arena.alloc(&c);
        assert_eq!(arena.get(first), a);
        assert_eq!(arena.get(second), b);
        assert_eq!(arena.get(oversized), c);
    }

    #[test]
    fn oversized_spills_keep_the_open_chunk_active() {
        let mut arena = Arena::new();
        let before = arena.alloc(b"narrow");
        let big = vec![9u8; 200 * 1024];
        let oversized = arena.alloc(&big);
        let after = arena.alloc(b"narrow again");
        assert_eq!(
            before.chunk, after.chunk,
            "the open chunk's free tail survives an oversized spill"
        );
        assert_ne!(oversized.chunk, before.chunk);
        assert_eq!(arena.get(before), b"narrow");
        assert_eq!(arena.get(oversized), big.as_slice());
        assert_eq!(arena.get(after), b"narrow again");
    }
}
