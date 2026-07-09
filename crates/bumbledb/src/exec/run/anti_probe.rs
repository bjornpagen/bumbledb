//! The anti-probe pass (docs/architecture/40-execution.md, § anti-probe
//! filters): after residual compaction, each anti-probe attached to the
//! node probes its negated occurrence per surviving binding; a hit
//! rejects the binding — the inverted polarity of a positive probe miss,
//! compacted through the same survivor cursor-write. Existence, not
//! continuation: the negated occurrence's trie schema is one probe level
//! holding all its variables, so a single `get`-style confirmation
//! decides the probe — an anti-probe never iterates a leaf.
//!
//! The probe's semantic — "no fact matches" — is the same judgment the
//! commit-time checker runs (`storage/commit/judgment.rs`), there against
//! LMDB guards rather than COLT: one mechanism, two callers
//! (`docs/architecture/50-storage.md` § commit step 3). The sharing is
//! the semantic and the compaction machinery, not a common function.

use super::{AntiProbeSpec, Colt, Counters, JoinPhase, Source, PREFETCH_WIDTH_FLOOR};

/// Evaluates one node's anti-probes over the current survivor set,
/// compacting rejected bindings away. Batched exactly like the sibling
/// probes: phase 1 gathers keys and hashes (pure ALU), phase 1.5
/// prefetches (width-floor gated), phase 2 issues the bucket loads and
/// the kernel compacts with the **inverted** keep condition (a hit
/// clears the mask bit). `read_slot(element, slot)` resolves an
/// already-bound outer word — the two callers differ only there
/// (`run_node` reads `bindings`, `probe_pass` the element's parent row).
#[allow(clippy::too_many_arguments)] // the probe-pass context, unpacked —
                                     // the same split borrows as the sibling passes
pub(super) fn anti_probe_pass<C: Counters>(
    specs: &[AntiProbeSpec],
    node_idx: usize,
    cover_vars: &[crate::ir::VarId],
    arity: usize,
    colts: &mut [Colt],
    entry_keys: &[u64],
    survivors: &mut Vec<u32>,
    probe_keys: &mut [u64],
    hashes: &mut Vec<u64>,
    mask: &mut Vec<u8>,
    anti_sources: &mut [Vec<Source>],
    read_slot: impl Fn(usize, usize) -> u64,
    counters: &mut C,
) {
    for (a_idx, spec) in specs.iter().enumerate() {
        if survivors.is_empty() {
            return;
        }
        let n = survivors.len();

        // The zero-variable emptiness gate: with no key words the probe
        // asks only whether the (filtered) negated occurrence holds any
        // fact — one batch-constant answer, no per-element work.
        if spec.key_words == 0 {
            let start = colts[spec.occ].start();
            let hit = colts[spec.occ].key_count(start).magnitude() > 0;
            for _ in 0..n {
                counters.anti_probe(node_idx, hit);
            }
            if hit {
                survivors.clear();
            }
            continue;
        }

        // Resolve key sources against the runtime cover choice: a
        // variable bound by this node's cover reads the batch key
        // column; everything else reads its (already bound) outer slot.
        let sources = &mut anti_sources[a_idx];
        sources.clear();
        for (var, slot, width) in &spec.parts {
            let source =
                cover_vars
                    .iter()
                    .position(|cv| cv == var)
                    .map_or(Source::Slot(*slot), |word| {
                        // The batch key layout carries one word per cover
                        // variable; a two-slot (interval) variable can only
                        // arrive through an outer slot today.
                        debug_assert_eq!(*width, 1, "batch keys are one word per variable");
                        Source::Batch(word)
                    });
            sources.push(source);
        }

        counters.phase_start(node_idx, JoinPhase::Force);
        let start = colts[spec.occ].start();
        colts[spec.occ].ensure_forced(start, 0);
        counters.phase_end(node_idx, JoinPhase::Force);

        // Phase 1: gather every probe key and compute every hash — pure
        // ALU, no bucket loads.
        counters.phase_start(node_idx, JoinPhase::Hash);
        let kw = spec.key_words;
        hashes.clear();
        hashes.resize(n, 0);
        {
            let probe_keys = &mut probe_keys[..n * kw];
            let hashes = &mut hashes[..n];
            for (k, &e) in survivors.iter().enumerate() {
                let element = usize::try_from(e).expect("batch fits usize");
                let mut word = k * kw;
                for (i, (_, slot, width)) in spec.parts.iter().enumerate() {
                    match sources[i] {
                        Source::Batch(col) => {
                            probe_keys[word] = entry_keys[element * arity + col];
                            word += 1;
                        }
                        Source::Slot(_) => {
                            // An interval variable contributes its two
                            // consecutive slot words in layout order.
                            for offset in 0..*width {
                                probe_keys[word] = read_slot(element, slot + offset);
                                word += 1;
                            }
                        }
                    }
                }
                debug_assert_eq!(word, (k + 1) * kw, "key widths add up");
                hashes[k] = crate::exec::colt::hash_key(&probe_keys[k * kw..(k + 1) * kw]);
            }
        }
        counters.phase_end(node_idx, JoinPhase::Hash);

        // Phase 1.5: the prefetch pass, width-floor gated — see run_node.
        if n >= PREFETCH_WIDTH_FLOOR {
            crate::obs::event(
                crate::obs::names::PREFETCH_PASS,
                crate::obs::Category::Execute,
                n as u64,
                colts[spec.occ].probe_footprint_bytes() as u64,
            );
            for &hash in hashes.iter() {
                colts[spec.occ].prefetch_bucket(start, hash);
            }
        }

        // Phase 2: bucket loads, then kernel compaction with the
        // inverted keep condition — an anti-probe HIT is rejection. The
        // `get` confirms existence at the single probe level and the
        // child cursor is discarded: never a descent, never a leaf
        // iteration.
        counters.phase_start(node_idx, JoinPhase::Probe);
        mask.clear();
        mask.resize(n, 0);
        {
            let probe_keys = &probe_keys[..n * kw];
            let hashes = &hashes[..n];
            let mask = &mut mask[..n];
            let colt = &mut colts[spec.occ];
            for k in 0..n {
                let hit = colt
                    .get_prehashed(start, 0, &probe_keys[k * kw..(k + 1) * kw], hashes[k])
                    .is_some();
                counters.anti_probe(node_idx, hit);
                mask[k] = u8::from(!hit);
            }
        }
        crate::exec::kernel::compact_u32_by_mask(survivors, mask);
        counters.phase_end(node_idx, JoinPhase::Probe);
        hashes.clear();
    }
}
