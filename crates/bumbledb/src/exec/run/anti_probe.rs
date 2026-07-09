//! The anti-probe pass (docs/architecture/40-execution.md, § anti-probe
//! filters): after residual compaction, each anti-probe attached to the
//! node probes its negated occurrence per surviving binding; a hit
//! rejects the binding — the inverted polarity of a positive probe miss,
//! compacted through the same survivor cursor-write. Existence, not
//! continuation: the negated occurrence's trie schema is one probe level
//! holding all its variables, so a single `get`-style confirmation
//! decides the probe — an anti-probe never iterates a leaf. The one
//! exception is a negated **membership** binding: the hit's positions are
//! scanned for a fact that also satisfies every var-sourced membership,
//! because rejection requires a fact matching keys AND memberships.
//!
//! The probe's semantic — "no fact matches" — is the same judgment the
//! commit-time checker runs (`storage/commit/judgment.rs`), there against
//! LMDB guards rather than COLT: one mechanism, two callers
//! (`docs/architecture/50-storage.md` § commit step 3). The sharing is
//! the semantic and the compaction machinery, not a common function.

use super::{word_base, AntiProbeSpec, Colt, Counters, JoinPhase, Source, PREFETCH_WIDTH_FLOOR};

/// Evaluates one node's anti-probes over the current survivor set,
/// compacting rejected bindings away. Batched exactly like the sibling
/// probes: phase 1 gathers keys and hashes (pure ALU), phase 1.5
/// prefetches (width-floor gated), phase 2 issues the bucket loads and
/// the kernel compacts with the **inverted** keep condition (a hit
/// clears the mask bit). `read_slot(element, slot)` resolves an
/// already-bound outer word — the two callers differ only there
/// (`run_node` reads `bindings`, `probe_pass` the element's parent row).
/// Sources are per key **word**: an interval variable's pair reads two
/// consecutive batch words or slots (the `SlotWidth` layout).
#[allow(clippy::too_many_arguments)] // the probe-pass context, unpacked —
// the same split borrows as the sibling passes
#[allow(clippy::too_many_lines)] // one pass, three probe forms (gate,
                                 // keyless membership, keyed) — the
                                 // invariants read in order
pub(super) fn anti_probe_pass<C: Counters>(
    specs: &[AntiProbeSpec],
    node_idx: usize,
    cover_vars: &[crate::ir::VarId],
    var_widths: &[(crate::ir::VarId, usize)],
    arity: usize,
    colts: &mut [Colt],
    entry_keys: &[u64],
    survivors: &mut Vec<u32>,
    probe_keys: &mut [u64],
    hashes: &mut Vec<u64>,
    mask: &mut Vec<u8>,
    anti_sources: &mut [Vec<Source>],
    point_checks: &mut Vec<(usize, usize, u64)>,
    read_slot: impl Fn(usize, usize) -> u64,
    counters: &mut C,
) {
    let width_of = |var: crate::ir::VarId| -> usize {
        var_widths
            .iter()
            .find(|(v, _)| *v == var)
            .expect("plans bind every variable")
            .1
    };
    // A point variable is scalar (one word); its per-element word source.
    let point_word = |element: usize,
                      var: crate::ir::VarId,
                      slot: usize,
                      entry_keys: &[u64],
                      read_slot: &dyn Fn(usize, usize) -> u64| {
        word_base(cover_vars, var, width_of).map_or_else(
            || read_slot(element, slot),
            |base| entry_keys[element * arity + base],
        )
    };
    for (a_idx, spec) in specs.iter().enumerate() {
        if survivors.is_empty() {
            return;
        }
        let n = survivors.len();

        // The zero-variable emptiness gate: with no key words the probe
        // asks only whether the (filtered) negated occurrence holds any
        // fact — one batch-constant answer, no per-element work. A
        // membership-carrying gate reads its point variables per
        // element, so it takes the per-element scan below instead.
        if spec.key_words == 0 && spec.point_parts.is_empty() {
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
        if spec.key_words == 0 {
            // Keyless membership gate: per element, "some fact's interval
            // holds the bound point" rejects — the existential reading
            // over the negated occurrence (docs/architecture/
            // 20-query-ir.md: a binding position matches iff the value
            // satisfies it for SOME fact / ANY element).
            let start = colts[spec.occ].start();
            mask.clear();
            mask.resize(n, 0);
            for k in 0..n {
                let element = usize::try_from(survivors[k]).expect("batch fits usize");
                point_checks.clear();
                for (start_col, end_col, var, slot) in &spec.point_parts {
                    point_checks.push((
                        *start_col,
                        *end_col,
                        point_word(element, *var, *slot, entry_keys, &read_slot),
                    ));
                }
                let hit = colts[spec.occ].any_position_matches(start, point_checks);
                counters.anti_probe(node_idx, hit);
                mask[k] = u8::from(!hit);
            }
            crate::exec::kernel::compact_u32_by_mask(survivors, mask);
            continue;
        }

        // Resolve key sources against the runtime cover choice, one per
        // key word: a variable bound by this node's cover reads the
        // batch key words at its word base; everything else reads its
        // (already bound) outer slots.
        let sources = &mut anti_sources[a_idx];
        sources.clear();
        for (var, slot, width) in &spec.parts {
            match word_base(cover_vars, *var, width_of) {
                Some(base) => {
                    for offset in 0..*width {
                        sources.push(Source::Batch(base + offset));
                    }
                }
                None => {
                    for offset in 0..*width {
                        sources.push(Source::Slot(slot + offset));
                    }
                }
            }
        }
        debug_assert_eq!(sources.len(), spec.key_words, "key widths add up");

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
                for (word, source) in sources.iter().enumerate() {
                    probe_keys[k * kw + word] = match *source {
                        Source::Batch(col) => entry_keys[element * arity + col],
                        Source::Slot(slot) => read_slot(element, slot),
                    };
                }
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
        // `get` confirms existence at the single probe level; the child
        // cursor is consumed only by a membership-carrying probe, whose
        // rejection needs a fact matching keys AND every membership —
        // the existential reading over the negated occurrence's facts
        // (docs/architecture/20-query-ir.md: the term matches iff SOME
        // fact / ANY element satisfies it).
        counters.phase_start(node_idx, JoinPhase::Probe);
        mask.clear();
        mask.resize(n, 0);
        {
            let probe_keys = &probe_keys[..n * kw];
            let hashes = &hashes[..n];
            let mask = &mut mask[..n];
            for k in 0..n {
                let element = usize::try_from(survivors[k]).expect("batch fits usize");
                let child = colts[spec.occ].get_prehashed(
                    start,
                    0,
                    &probe_keys[k * kw..(k + 1) * kw],
                    hashes[k],
                );
                let hit = match child {
                    None => false,
                    Some(child) if spec.point_parts.is_empty() => {
                        let _ = child;
                        true
                    }
                    Some(child) => {
                        point_checks.clear();
                        for (start_col, end_col, var, slot) in &spec.point_parts {
                            point_checks.push((
                                *start_col,
                                *end_col,
                                point_word(element, *var, *slot, entry_keys, &read_slot),
                            ));
                        }
                        colts[spec.occ].any_position_matches(child, point_checks)
                    }
                };
                counters.anti_probe(node_idx, hit);
                mask[k] = u8::from(!hit);
            }
        }
        crate::exec::kernel::compact_u32_by_mask(survivors, mask);
        counters.phase_end(node_idx, JoinPhase::Probe);
        hashes.clear();
    }
}
