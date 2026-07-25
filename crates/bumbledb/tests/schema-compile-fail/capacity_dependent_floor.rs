//! A dependent bound in the floor slot — dependent bounds are hi-slot
//! only (ruled 2026-07-24, C6): a dependent floor has no use case, and
//! inversion with idents is statically undecidable, so the descriptor
//! cannot carry the shape and the lowering refuses the spelling.
//@ error: a dependent bound in the floor slot
//@ error: hi-slot only (ruled 2026-07-24, C6)

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{supply..9000} Device(pool);
}
