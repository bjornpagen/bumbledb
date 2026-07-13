//! The sink shapes: `CountDistinct` steered across **all six types**,
//! and Arg-restriction (`ArgMax`/`ArgMin`) with constructed ties, both
//! directions, key-projected and multi-carry variants
//! (`docs/architecture/20-query-ir.md` § aggregation).

use bumbledb::{AggOp, FindTerm, Term};

use crate::gen::Rng;
use crate::querygen::target::ids;
use crate::querygen::Builder;

/// One `CountDistinct` per query, its input variable drawn to cover
/// every structural type across a batch: u64, i64, bool, string,
/// bytes, and interval. Half the typed variants carry a group key; the
/// rest are global (one group, empty key).
pub(super) fn count_distinct(b: &mut Builder, rng: &mut Rng) {
    let over = match rng.range(7) {
        // U64: distinct accounts, optionally per entry.
        0 => {
            let posting = b.add_atom(ids::POSTING);
            let over = b.bind_var(posting, ids::posting::ACCOUNT);
            if rng.chance(1, 2) {
                let entry = b.bind_var(posting, ids::posting::ENTRY);
                b.find_var(entry);
            }
            over
        }
        // I64: distinct amounts, optionally per account.
        1 => {
            let posting = b.add_atom(ids::POSTING);
            let over = b.bind_var(posting, ids::posting::AMOUNT);
            if rng.chance(1, 2) {
                let account = b.bind_var(posting, ids::posting::ACCOUNT);
                b.find_var(account);
            }
            over
        }
        // Vocabulary: distinct currencies, optionally per holder.
        2 => {
            let account = b.add_atom(ids::ACCOUNT);
            let over = b.bind_var(account, ids::account::CURRENCY);
            if rng.chance(1, 2) {
                let holder = b.bind_var(account, ids::account::HOLDER);
                b.find_var(holder);
            }
            over
        }
        // Bool: distinct reconciliation states per account.
        3 => {
            let posting = b.add_atom(ids::POSTING);
            let over = b.bind_var(posting, ids::posting::RECONCILED);
            if rng.chance(1, 2) {
                let account = b.bind_var(posting, ids::posting::ACCOUNT);
                b.find_var(account);
            }
            over
        }
        // String: distinct holder names, global.
        4 => {
            let holder = b.add_atom(ids::HOLDER);
            b.bind_var(holder, ids::holder::NAME)
        }
        // bytes<N>: distinct digests — the 32-byte extref (maximal
        // cardinality), or a pad-boundary tag (widths 8/16/64 rotate:
        // small vocabularies, so distinctness folds real duplicates),
        // optionally per window-group id.
        5 => {
            let transfer = b.add_atom(ids::TRANSFER);
            match rng.range(4) {
                0 => b.bind_var(transfer, ids::transfer::EXTREF),
                which => {
                    // TAGS is width order 7/8/9/16/63/64: pick 8/16/64.
                    let field = ids::transfer::TAGS[match which {
                        1 => 1,
                        2 => 3,
                        _ => 5,
                    }];
                    let over = b.bind_var(transfer, field);
                    if rng.chance(1, 2) {
                        let id = b.bind_var(transfer, ids::transfer::ID);
                        b.find_var(id);
                    }
                    over
                }
            }
        }
        // Interval: distinct active windows, optionally per account —
        // distinctness of interval *values*, the two-word type.
        _ => {
            let mandate = b.add_atom(ids::MANDATE);
            let over = b.bind_var(mandate, ids::mandate::ACTIVE);
            if rng.chance(1, 2) {
                let account = b.bind_var(mandate, ids::mandate::ACCOUNT);
                b.find_var(account);
            }
            over
        }
    };
    b.finds.push(FindTerm::Aggregate {
        op: AggOp::CountDistinct,
        over: Some(over),
    });
    // A quarter multi-aggregate: Count beside CountDistinct — the
    // witness-multiplicity vs value-distinctness pair.
    if rng.chance(1, 4) {
        b.finds.push(FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        });
    }
}

/// Arg-restriction: latest/earliest posting per account (or globally).
/// The key alternates between the corpus's **tie-rich** field
/// (`amount`, quantized to 8 levels — the extreme is attained by many
/// rows, so ties are constructed, never hoped for) and the **tie-free**
/// field (`at`, strictly monotone). Both directions; a third of the
/// queries project the key itself (a second Arg term carrying the key);
/// a quarter carry a second variable (multi-carry coherence).
pub(super) fn arg(b: &mut Builder, rng: &mut Rng) {
    let posting = b.add_atom(ids::POSTING);
    let carried = b.bind_var(posting, ids::posting::ID);
    let key = if rng.chance(1, 2) {
        b.bind_var(posting, ids::posting::AMOUNT)
    } else {
        b.bind_var(posting, ids::posting::AT)
    };
    // The group key: per-account, or global (empty key, one group).
    if rng.chance(4, 5) {
        let account = b.bind_var(posting, ids::posting::ACCOUNT);
        b.find_var(account);
    }
    let op = if rng.chance(1, 2) {
        AggOp::ArgMax { key }
    } else {
        AggOp::ArgMin { key }
    };
    b.finds.push(FindTerm::Aggregate {
        op,
        over: Some(carried),
    });
    if rng.chance(7, 20) {
        // Key-projected: the key rides out as a carried value too.
        b.finds.push(FindTerm::Aggregate {
            op,
            over: Some(key),
        });
    }
    if rng.chance(1, 4) {
        // Multi-carry: all carried values come from the same surviving
        // bindings, by the restriction-first semantics.
        let instrument = b.bind_var(posting, ids::posting::INSTRUMENT);
        b.finds.push(FindTerm::Aggregate {
            op,
            over: Some(instrument),
        });
    }
    // Half the Arg queries join through Account — restriction over a
    // join, not just a scan.
    if rng.chance(1, 2) {
        let account_join = b.var_at(0, ids::posting::ACCOUNT).expect("var or fresh");
        let account = b.add_atom(ids::ACCOUNT);
        b.bind(account, ids::account::ID, Term::Var(account_join));
    }
}
