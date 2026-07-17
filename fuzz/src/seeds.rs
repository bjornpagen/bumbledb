//! The seed-corpus converter (TODO.md § PHASE A-FUZZ; the corpus
//! discipline in `fuzz/README.md`). Fuzzer inputs are entropy streams —
//! 8-byte little-endian words — so a seed is a WORD RECIPE, not a
//! serialized case: the leading words pin a generation tier (the knobs
//! in [`crate::theory`] and [`crate::query::run`]'s hostile arm), the
//! tail cycles the boundary vocabulary the conformance suite exercises
//! (width edges, caps and fences, Allen-mask boundaries, ray and
//! ceiling sentinels), and the deterministic zero tail completes every
//! draw. The checked-in seeds under `corpus/{theory,query}/seed-*` are
//! this module's output verbatim; regenerate with
//! `BUMBLEDB_FUZZ_SEED_DIR=fuzz/corpus cargo test -p bumbledb-fuzz seeds`.
//! Post-session `cargo fuzz cmin` owns their afterlife, per the corpus
//! policy.

/// The boundary vocabulary, one draw-word each: small structural draws,
/// the rule/predicate fence (16/17), the occurrence cap (20/21), the
/// width and depth edges (63/64/65), the extension boundary (255/256),
/// the Allen-mask shoulder pair, and the ceiling sentinels.
const BOUNDARY: [u64; 16] = [
    1,
    2,
    3,
    16,
    17,
    20,
    21,
    63,
    64,
    65,
    255,
    256,
    8190,
    8191,
    u64::MAX - 1,
    u64::MAX,
];

/// A word stream: the tier prefix, then `len` boundary words cycling
/// from `offset` at `stride` — different offsets and strides land the
/// same vocabulary on different draw positions.
fn stream(prefix: &[u64], offset: usize, stride: usize, len: usize) -> Vec<u8> {
    prefix
        .iter()
        .copied()
        .chain((0..len).map(|i| BOUNDARY[(offset + i * stride) % BOUNDARY.len()]))
        .flat_map(u64::to_le_bytes)
        .collect()
}

/// The theory target's seeds: the zero-tail input, plus three
/// offset/stride mixes per tier (word 0 ≡ 3 mod 4 lands the
/// adversarial tier; everything else the structurally-free arm).
#[must_use]
pub fn theory_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = vec![("seed-theory-zero".to_owned(), stream(&[0], 0, 1, 0))];
    for k in 0..3 {
        seeds.push((
            format!("seed-theory-free-mix-{k}"),
            stream(&[0], k * 5, k + 1, 12),
        ));
        seeds.push((
            format!("seed-theory-adv-mix-{k}"),
            stream(&[3], k * 5, k + 1, 12),
        ));
    }
    seeds
}

/// The query target's seeds: the hostile knob is word 0 (≡ 0 mod 4),
/// the tier knob word 1 (0/1 the free arm — kept — 2 the adversarial
/// tier, 3 the program arm); word 0 ≡ 1 lands the valid parity arm.
#[must_use]
pub fn query_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = vec![("seed-query-zero".to_owned(), stream(&[0], 0, 1, 0))];
    for k in 0..3 {
        seeds.push((
            format!("seed-query-hostile-free-mix-{k}"),
            stream(&[0, 0], k * 5, k + 1, 12),
        ));
        seeds.push((
            format!("seed-query-hostile-adv-mix-{k}"),
            stream(&[0, 2], k * 5, k + 1, 12),
        ));
        seeds.push((
            format!("seed-query-hostile-program-mix-{k}"),
            stream(&[0, 3], k * 5, k + 1, 12),
        ));
        seeds.push((
            format!("seed-query-valid-mix-{k}"),
            stream(&[1], k * 5, k + 1, 12),
        ));
    }
    seeds
}

#[cfg(test)]
mod tests {
    use super::{query_seeds, theory_seeds};

    /// Every theory seed replays through the real runner — the run IS
    /// the oracle stack, so a panicking seed is a finding, not a bad
    /// seed.
    #[test]
    fn every_theory_seed_replays_through_the_runner() {
        for (name, bytes) in theory_seeds() {
            eprintln!("theory seed: {name}");
            crate::theory(&bytes);
        }
    }

    /// Every query seed replays through the real runner (the cached
    /// Tiny world builds once).
    #[test]
    fn every_query_seed_replays_through_the_runner() {
        for (name, bytes) in query_seeds() {
            eprintln!("query seed: {name}");
            crate::query::run(&bytes);
        }
    }

    /// The writer: `BUMBLEDB_FUZZ_SEED_DIR=fuzz/corpus cargo test -p
    /// bumbledb-fuzz seeds` regenerates the checked-in files byte for
    /// byte. Without the variable this test is a no-op — the corpus is
    /// never written as a side effect.
    #[test]
    fn write_the_seed_corpus_when_asked() {
        let Some(root) = std::env::var_os("BUMBLEDB_FUZZ_SEED_DIR") else {
            return;
        };
        let root = std::path::PathBuf::from(root);
        for (target, seeds) in [("theory", theory_seeds()), ("query", query_seeds())] {
            let dir = root.join(target);
            std::fs::create_dir_all(&dir).expect("seed corpus dir");
            for (name, bytes) in seeds {
                std::fs::write(dir.join(name), bytes).expect("write seed");
            }
        }
    }
}
