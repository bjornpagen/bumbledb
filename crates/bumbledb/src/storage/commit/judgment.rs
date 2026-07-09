//! Phase 3, the containment judgment (`docs/architecture/50-storage.md`
//! § commit step 3; `30-dependencies.md` § enforcement). Source side:
//! every inserted fact satisfying a statement's source selection proves
//! its target tuple exists in the final state — scalar tuples by one
//! guard probe, interval positions by the coverage walk. Target side:
//! every key tuple disestablished by this commit probes its dependent
//! statements' `R` prefixes for surviving sources — a scalar survivor is
//! the violation outright; an interval survivor re-runs the coverage walk
//! against the final `U` state. LMDB write transactions read their own
//! writes, so both sides see exactly the state the commit would persist.
//!
//! Also home of the selection machinery shared with the insert phase's
//! `R`-puts: literals encode once per commit into [`Selections`]
//! (never per fact), and [`satisfies`] is a straight byte compare of
//! selected field slices.

use std::collections::BTreeSet;
use std::ops::Bound;

use crate::encoding::{encode_literal, encode_u64, field_bytes, FactLayout};
use crate::error::{CorruptionError, Direction, Error, Result};
use crate::obs;
use crate::schema::{FieldId, RelationId, Resolved, Schema, StatementDescriptor, StatementId};
use crate::storage::delta::WriteDelta;
use crate::storage::env::{ReadTxn, WriteTxn};
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::value::Value;

use super::applier::decode_row_id;

/// One side's selection σ, its literals pre-encoded for byte comparison.
pub(crate) enum SelectionCheck {
    /// σ is empty: every fact satisfies.
    Empty,
    /// Byte-compare each selected field's slice against its literal's
    /// canonical encoding.
    Compare(Box<[(FieldId, Box<[u8]>)]>),
    /// A String/Bytes literal was never interned: no stored fact can
    /// carry its id, so no fact satisfies σ.
    Never,
}

/// Both selections of one containment statement.
pub(crate) struct SideChecks {
    pub(crate) source: SelectionCheck,
    pub(crate) target: SelectionCheck,
}

/// An intern resolver: maps a dictionary tag plus raw bytes to an intern
/// id, or `None` when no fact can carry the value — the one seam between
/// [`Selections::encode`] (delta-aware) and [`Selections::encode_committed`]
/// (committed dictionary only).
type InternResolver<'a> = dyn FnMut(u8, &[u8]) -> Result<Option<u64>> + 'a;

/// Pre-encoded selections for every `Containment` statement, built once
/// per commit — the commit-local scratch that keeps literal encoding out
/// of the per-fact loops.
pub(crate) struct Selections {
    /// Indexed by [`StatementId`]; `None` for non-containment statements.
    checks: Box<[Option<SideChecks>]>,
}

impl Selections {
    /// Encodes every containment statement's selection literals. String
    /// and Bytes literals resolve to intern ids through the delta's
    /// pending map, then the committed dictionary — a double miss proves
    /// no fact can satisfy the selection ([`SelectionCheck::Never`]).
    pub(crate) fn encode(delta: &WriteDelta<'_>, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(delta.schema(), &mut |tag, raw| {
            delta.resolve(view, tag, raw)
        })
    }

    /// The read-only sibling of [`Selections::encode`] for
    /// `Db::verify_store`: no delta exists, so String and Bytes literals
    /// resolve through the committed dictionary alone — a miss proves no
    /// *committed* fact can satisfy the selection, exactly the judgment
    /// the sweeper re-checks.
    pub(crate) fn encode_committed(schema: &Schema, view: &ReadTxn<'_>) -> Result<Self> {
        Self::encode_with(schema, &mut |tag, raw| {
            crate::storage::dict::lookup_tagged(view, tag, raw)
        })
    }

    /// The shared constructor over an [`InternResolver`].
    fn encode_with(schema: &Schema, resolve: &mut InternResolver<'_>) -> Result<Self> {
        let checks = schema
            .statements()
            .iter()
            .map(|statement| {
                let StatementDescriptor::Containment { source, target } = &statement.descriptor
                else {
                    return Ok(None);
                };
                Ok(Some(SideChecks {
                    source: encode_selection(&source.selection, resolve)?,
                    target: encode_selection(&target.selection, resolve)?,
                }))
            })
            .collect::<Result<Box<[_]>>>()?;
        Ok(Self { checks })
    }

    /// The checks of a containment statement.
    ///
    /// # Panics
    ///
    /// On a non-containment id — programmer invariant: callers hand ids
    /// from a relation's `outgoing`/`incoming` index, which the validated
    /// schema fills with `Containment` statements only.
    pub(crate) fn containment(&self, id: StatementId) -> &SideChecks {
        self.checks[usize::from(id.0)]
            .as_ref()
            .expect("validated schema: outgoing ids name Containment statements")
    }
}

fn encode_selection(
    selection: &[(FieldId, Value)],
    resolve: &mut InternResolver<'_>,
) -> Result<SelectionCheck> {
    if selection.is_empty() {
        return Ok(SelectionCheck::Empty);
    }
    let mut fields = Vec::with_capacity(selection.len());
    for (field, literal) in selection {
        // The interned types resolve at this boundary (dictionary state is
        // per-database); everything else takes the one canonical encoding
        // shared with the fingerprint ([`encode_literal`]).
        let encoded: Box<[u8]> = match literal {
            Value::String(raw) => match resolve(crate::storage::dict::TAG_STRING, raw)? {
                Some(id) => Box::new(encode_u64(id)),
                None => return Ok(SelectionCheck::Never),
            },
            Value::Bytes(raw) => match resolve(crate::storage::dict::TAG_BYTES, raw)? {
                Some(id) => Box::new(encode_u64(id)),
                None => return Ok(SelectionCheck::Never),
            },
            literal => {
                let mut bytes = Vec::with_capacity(16);
                encode_literal(literal, &mut bytes);
                bytes.into_boxed_slice()
            }
        };
        fields.push((*field, encoded));
    }
    Ok(SelectionCheck::Compare(fields.into()))
}

/// Whether a fact satisfies a pre-encoded selection: one byte compare per
/// selected field, slices out of `fact_bytes` (interval fields compare
/// their whole 16 bytes — `field_bytes` widths come from the layout).
pub(crate) fn satisfies(check: &SelectionCheck, layout: &FactLayout, fact_bytes: &[u8]) -> bool {
    match check {
        SelectionCheck::Empty => true,
        SelectionCheck::Never => false,
        SelectionCheck::Compare(fields) => fields.iter().all(|(field, literal)| {
            field_bytes(fact_bytes, layout, usize::from(field.0)) == &literal[..]
        }),
    }
}

/// The source-side judgment: for each fact of the delta's insert set —
/// exactly the facts this commit added, by the net-disposition invariant,
/// so a redundant insert is never judged here — for each `outgoing`
/// containment statement whose source selection it satisfies, prove the
/// target tuple present (scalar) or covered (interval) in the final
/// state. The per-relation `outgoing` index drives the loops — a fact
/// whose relation has no outgoing statements touches none of this.
pub(super) fn check_source(
    txn: &WriteTxn<'_>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    delta: &WriteDelta<'_>,
    selections: &Selections,
) -> Result<()> {
    let schema = delta.schema();
    let mut checker = Checker {
        txn,
        data,
        schema,
        key: [0; MAX_KEY],
    };
    let mut key_bytes = Vec::new();
    let mut probes = 0u64;
    let mut span = obs::span(obs::names::JUDGMENT_SOURCE, obs::Category::Commit);
    for (rel, fact_bytes) in delta.inserts() {
        let relation = schema.relation(rel);
        for &sid in relation.outgoing() {
            let statement = schema.statement(sid);
            let StatementDescriptor::Containment { source, target } = &statement.descriptor else {
                unreachable!("validated schema: outgoing ids name Containment statements")
            };
            let Resolved::Containment {
                target_key,
                key_permutation,
                interval_position,
            } = &statement.resolved
            else {
                unreachable!("validated schema: Containment resolves as Containment")
            };
            let checks = selections.containment(sid);
            if !satisfies(&checks.source, relation.layout(), fact_bytes) {
                continue;
            }
            probes += 1;
            keys::permuted_guard_bytes(
                relation.layout(),
                &source.projection,
                key_permutation,
                fact_bytes,
                &mut key_bytes,
            );
            let probe = Probe {
                statement: sid,
                target_relation: target.relation,
                target_key: *target_key,
                target_check: &checks.target,
                key_bytes: &key_bytes,
                fact_bytes,
                direction: Direction::SourceUnsatisfied,
            };
            if interval_position.is_some() {
                checker.check_coverage(&probe)?;
            } else {
                checker.check_scalar(&probe)?;
            }
        }
    }
    span.set_args(probes, 0);
    span.end();
    Ok(())
}

/// The target-side judgment: every key tuple deleted in phase 1 and not
/// re-established in phase 2 probes its dependent containment
/// statements' `R` prefixes for surviving sources. Re-establishment is
/// **per statement, ψ-qualified** (`docs/architecture/50-storage.md`
/// § commit step 3): a dependent with a nonempty target selection counts
/// a re-landed guard tuple as re-established only if the establishing
/// fact satisfies its ψ — one `F` get per re-established tuple, shared
/// across that tuple's ψ-carrying dependents; empty-ψ dependents use the
/// plain set difference. A scalar survivor convicts outright: the key
/// statement's guard was the tuple's one holder and the final state no
/// longer has it. An interval tuple is a disestablished
/// *segment* `(prefix, ts, te)`: each surviving source of the prefix
/// group whose interval intersects the segment re-runs the coverage walk
/// against the final `U` state — a delete whose hole a same-delta insert
/// covers is legal, and only a failed walk convicts.
///
/// Ported subtlety: a source deleted this commit cannot have a surviving
/// `R` entry, because phase 1 removed its outgoing edges — so a survivor
/// is always live in the final state, no disposition re-check is needed,
/// and its `F` row must exist (a miss is corruption, never a race).
#[allow(clippy::too_many_lines)] // the target-side judgment, one phase per block
pub(super) fn check_target(
    txn: &WriteTxn<'_>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    delta: &WriteDelta<'_>,
    selections: &Selections,
    deleted_guards: &BTreeSet<(StatementId, Vec<u8>)>,
    inserted_guards: &BTreeSet<(StatementId, Vec<u8>)>,
) -> Result<()> {
    let schema = delta.schema();
    let mut span = obs::span(obs::names::JUDGMENT_TARGET, obs::Category::Commit);
    let mut scanned = 0u64;
    let mut key: KeyBuf = [0; MAX_KEY];
    // Affected sources of interval statements, deduped before any walk:
    // the element is the full surviving `R` key — statement ‖ prefix
    // group ‖ source interval ‖ source identity — so several
    // disestablished segments of one (statement, prefix-group) collapse
    // to one coverage walk per source.
    let mut affected: BTreeSet<Vec<u8>> = BTreeSet::new();
    for entry in deleted_guards {
        let (key_sid, guard) = entry;
        let reestablished = inserted_guards.contains(entry);
        // The establishing fact of a re-landed guard, fetched at most
        // once per tuple and shared by every ψ-carrying dependent.
        let mut establisher: Option<&[u8]> = None;
        let mut counted = false;
        for &sid in schema.dependents(*key_sid) {
            if reestablished {
                match &selections.containment(sid).target {
                    // Empty ψ: any establishing fact re-establishes —
                    // the plain set difference.
                    SelectionCheck::Empty => continue,
                    // ψ-qualification: the tuple stays disestablished
                    // for this statement unless the establishing fact
                    // satisfies its ψ. `Never` means no fact can.
                    SelectionCheck::Never => {}
                    check @ SelectionCheck::Compare(_) => {
                        let relation = key_relation(schema, *key_sid);
                        let fact = if let Some(fact) = establisher {
                            fact
                        } else {
                            let fact = establishing_fact(data, txn, relation, *key_sid, guard)?;
                            establisher = Some(fact);
                            fact
                        };
                        if satisfies(check, schema.relation(relation).layout(), fact) {
                            continue;
                        }
                    }
                }
            }
            if !counted {
                scanned += 1;
                counted = true;
            }
            let Resolved::Containment {
                interval_position, ..
            } = &schema.statement(sid).resolved
            else {
                unreachable!("validated schema: dependents name Containment statements")
            };
            if interval_position.is_none() {
                // Scalar form: any surviving entry under the exact key
                // bytes is a stranded source.
                let p_len = keys::reverse_prefix(&mut key, sid, guard);
                let survivor = data
                    .get_greater_than_or_equal_to(txn.raw(), &key[..p_len])?
                    .filter(|(k, _)| k.starts_with(&key[..p_len]));
                if let Some((r_key, _)) = survivor {
                    let (_, _, source_rel, source_row) = keys::parse_reverse_key(r_key).ok_or(
                        Error::Corruption(CorruptionError::MalformedValue("R key shape")),
                    )?;
                    let fact = fact_by_row(data, txn, source_rel, source_row)?;
                    return Err(Error::ContainmentViolation {
                        statement: sid,
                        direction: Direction::TargetRequired,
                        fact: fact.into(),
                    });
                }
            } else {
                // Interval form: conservatively scan the whole prefix
                // group and filter by intersection. An optimized lower
                // bound would need the maximum source-interval length,
                // which we refuse to track — the group is small and this
                // is the delete path.
                let ts = &guard[guard.len() - 16..guard.len() - 8];
                let te = &guard[guard.len() - 8..];
                let p_len = keys::reverse_prefix(&mut key, sid, &guard[..guard.len() - 16]);
                let bounds: (Bound<&[u8]>, Bound<&[u8]>) =
                    (Bound::Included(&key[..p_len]), Bound::Unbounded);
                for group_entry in data.range(txn.raw(), &bounds)? {
                    let (k, _) = group_entry?;
                    if !k.starts_with(&key[..p_len]) {
                        break;
                    }
                    let Some((_, key_bytes, _, _)) = keys::parse_reverse_key(k) else {
                        return Err(Error::Corruption(CorruptionError::MalformedValue(
                            "R key shape",
                        )));
                    };
                    // Same statement, same target key: any other key-bytes
                    // width is corrupt data, a hard error.
                    if key_bytes.len() != guard.len() {
                        return Err(Error::Corruption(CorruptionError::MalformedValue(
                            "R key width",
                        )));
                    }
                    // Half-open intersection of the source interval
                    // `[ss, se)` with the disestablished `[ts, te)`:
                    // `ss < te && ts < se`, byte compare on the 8-byte
                    // order-preserving halves.
                    let ss = &key_bytes[key_bytes.len() - 16..key_bytes.len() - 8];
                    let se = &key_bytes[key_bytes.len() - 8..];
                    if ss < te && ts < se {
                        affected.insert(k.to_vec());
                    }
                }
            }
        }
    }
    // The deduped walks, each against the final `U` state.
    let mut checker = Checker {
        txn,
        data,
        schema,
        key: [0; MAX_KEY],
    };
    for r_key in &affected {
        let (sid, key_bytes, source_rel, source_row) =
            keys::parse_reverse_key(r_key).expect("affected set holds parsed R keys");
        let statement = schema.statement(sid);
        let StatementDescriptor::Containment { target, .. } = &statement.descriptor else {
            unreachable!("validated schema: dependents name Containment statements")
        };
        let Resolved::Containment { target_key, .. } = &statement.resolved else {
            unreachable!("validated schema: Containment resolves as Containment")
        };
        let fact_bytes = fact_by_row(data, txn, source_rel, source_row)?;
        let probe = Probe {
            statement: sid,
            target_relation: target.relation,
            target_key: *target_key,
            target_check: &selections.containment(sid).target,
            key_bytes,
            fact_bytes,
            direction: Direction::TargetRequired,
        };
        checker.check_coverage(&probe)?;
    }
    span.set_args(scanned, 0);
    span.end();
    Ok(())
}

/// The relation a key (`Functionality`) statement guards.
fn key_relation(schema: &Schema, key: StatementId) -> RelationId {
    let StatementDescriptor::Functionality { relation, .. } = &schema.statement(key).descriptor
    else {
        unreachable!("validated schema: guard-set ids name Functionality statements")
    };
    *relation
}

/// The fact that re-established a key guard in phase 2, reached through
/// the guard's own `U` entry — the ψ-qualification subject. Both gets
/// hit state this commit just wrote (write txns read their own writes),
/// so a miss is corruption, never a race.
fn establishing_fact<'t>(
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    txn: &'t WriteTxn<'_>,
    relation: RelationId,
    key: StatementId,
    guard: &[u8],
) -> Result<&'t [u8]> {
    let mut buf: KeyBuf = [0; MAX_KEY];
    let u_len = keys::guard_key(&mut buf, relation, key, guard);
    let value = data
        .get(txn.raw(), &buf[..u_len])?
        .ok_or(Error::Corruption(CorruptionError::MalformedValue(
            "re-established U guard",
        )))?;
    fact_by_row(data, txn, relation, decode_row_id(value)?)
}

/// Fetches a fact's canonical bytes by row — the surviving source (the
/// violation payload or the coverage walk's subject) and the
/// re-establishing target (the ψ check's subject). Borrowed from the
/// transaction: the target side reads, never copies, until an error is
/// built.
fn fact_by_row<'t>(
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    txn: &'t WriteTxn<'_>,
    relation: RelationId,
    row_id: u64,
) -> Result<&'t [u8]> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let f_len = keys::fact_key(&mut key, relation, row_id);
    data.get(txn.raw(), &key[..f_len])?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation,
            row_id,
        }))
}

/// One (source fact, containment statement) judgment pair: everything a
/// target probe needs, borrowed from the driving loop. Both sides build
/// these — the source side for each inserted fact inside σ, the target
/// side for each surviving source whose required window a delete touched.
struct Probe<'a> {
    statement: StatementId,
    target_relation: RelationId,
    /// The `Functionality` statement whose `U` guard is probed.
    target_key: StatementId,
    target_check: &'a SelectionCheck,
    /// The source fact's projection, already in target guard order.
    key_bytes: &'a [u8],
    /// The source fact — the violation payload.
    fact_bytes: &'a [u8],
    /// Which side's judgment a miss convicts.
    direction: Direction,
}

impl Probe<'_> {
    /// The aborting error: the judgment speaks about sources, so the
    /// payload is the source fact — the inserted fact whose target is
    /// missing, or the survivor whose required target was disestablished.
    fn unsatisfied(&self) -> Error {
        Error::ContainmentViolation {
            statement: self.statement,
            direction: self.direction,
            fact: self.fact_bytes.into(),
        }
    }
}

/// Working state threaded through the judgment probes — both sides share
/// the scalar probe and the coverage walk.
struct Checker<'a, 'env> {
    txn: &'a WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    schema: &'a Schema,
    key: KeyBuf,
}

impl Checker<'_, '_> {
    /// Scalar target probe: one `U` get on the target key's guard. A miss
    /// is the violation; a hit with a nonempty target selection
    /// additionally checks the found fact against σ (one `F` get).
    fn check_scalar(&mut self, probe: &Probe<'_>) -> Result<()> {
        let u_len = keys::guard_key(
            &mut self.key,
            probe.target_relation,
            probe.target_key,
            probe.key_bytes,
        );
        let Some(value) = self.data.get(self.txn.raw(), &self.key[..u_len])? else {
            return Err(probe.unsatisfied());
        };
        self.check_segment(probe, value)
    }

    /// The coverage walk (`docs/architecture/30-dependencies.md`
    /// § pointwise lifting): the source interval `[s, e)` must be jointly
    /// covered by the target's guard entries sharing its scalar prefix.
    /// Sound in one forward pass because the target's own pointwise key
    /// keeps the prefix group's intervals disjoint and start-ordered. All
    /// comparisons are on the 8-byte encoded halves — order-preserving,
    /// so byte compare is numeric compare, and a `MAX`-sentinel end is
    /// just the largest end.
    fn check_coverage(&mut self, probe: &Probe<'_>) -> Result<()> {
        // The scratch holds the full guard key
        // `U | rel | stmt | prefix | s | e` (the acceptance gate puts the
        // interval last, so its 16 bytes are the tail). Only slices of it
        // are used: the group prefix, the seek key `group ‖ s`, and the
        // source bounds.
        let full_len = keys::guard_key(
            &mut self.key,
            probe.target_relation,
            probe.target_key,
            probe.key_bytes,
        );
        let group_len = full_len - 16;
        let seek_len = full_len - 8;
        let source_end: [u8; 8] = self.key[seek_len..full_len]
            .try_into()
            .expect("fixed-width slice");

        // Entry (the walk's step 1): the segment covering `s`. A segment
        // starting exactly at `s` has full key `seek ‖ its end`, so the ≥
        // probe lands on it first when it exists; otherwise the group's
        // predecessor must still be running at `s` — its start ≤ s by
        // byte order, its end > s checked here. Anything else is the
        // entry gap.
        let at_or_after = self
            .data
            .get_greater_than_or_equal_to(self.txn.raw(), &self.key[..seek_len])?
            .filter(|(k, _)| k.starts_with(&self.key[..seek_len]));
        let (entry_key, entry_value) = match at_or_after {
            Some(hit) => hit,
            None => match self
                .data
                .get_lower_than(self.txn.raw(), &self.key[..seek_len])?
            {
                Some((k, v)) if k.starts_with(&self.key[..group_len]) => {
                    if k.len() != full_len {
                        return Err(Error::Corruption(CorruptionError::MalformedValue(
                            "U guard key length",
                        )));
                    }
                    if k[full_len - 8..] <= self.key[group_len..seek_len] {
                        // Predecessor ended at or before s: entry gap.
                        return Err(probe.unsatisfied());
                    }
                    (k, v)
                }
                _ => return Err(probe.unsatisfied()),
            },
        };
        if entry_key.len() != full_len {
            return Err(Error::Corruption(CorruptionError::MalformedValue(
                "U guard key length",
            )));
        }
        self.check_segment(probe, entry_value)?;
        let mut covered: [u8; 8] = entry_key[full_len - 8..]
            .try_into()
            .expect("fixed-width slice");

        // Chain (the walk's step 2): extend `covered` to the source's end.
        let bounds: (Bound<&[u8]>, Bound<&[u8]>) = (Bound::Excluded(entry_key), Bound::Unbounded);
        let mut chain = self.data.range(self.txn.raw(), &bounds)?;
        while covered < source_end {
            // Gap or prefix exhaustion before reaching `e` is the
            // violation.
            let Some(entry) = chain.next() else {
                return Err(probe.unsatisfied());
            };
            let (k, v) = entry?;
            if !k.starts_with(&self.key[..group_len]) {
                return Err(probe.unsatisfied());
            }
            if k.len() != full_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U guard key length",
                )));
            }
            // The next segment must start at or before `covered`. The
            // target key's own disjointness makes `start == covered` the
            // only non-gap case, but the walk writes ≤ and lets the key's
            // own invariant carry that proof.
            if k[group_len..seek_len] > covered[..] {
                return Err(probe.unsatisfied());
            }
            self.check_segment(probe, v)?;
            let end = &k[full_len - 8..];
            if end > &covered[..] {
                covered.copy_from_slice(end);
            }
        }
        Ok(())
    }

    /// The per-segment target-selection check: with an empty σ the guard
    /// hit alone is the proof; otherwise the found target fact is fetched
    /// (one `F` get via the guard's row id) and byte-checked against σ.
    fn check_segment(&self, probe: &Probe<'_>, value: &[u8]) -> Result<()> {
        if matches!(probe.target_check, SelectionCheck::Empty) {
            return Ok(());
        }
        let row_id = decode_row_id(value)?;
        // Own scratch: `self.key` still holds the caller's guard key.
        let mut key: KeyBuf = [0; MAX_KEY];
        let f_len = keys::fact_key(&mut key, probe.target_relation, row_id);
        let target_fact =
            self.data
                .get(self.txn.raw(), &key[..f_len])?
                .ok_or(Error::Corruption(CorruptionError::MissingFact {
                    relation: probe.target_relation,
                    row_id,
                }))?;
        let layout = self.schema.relation(probe.target_relation).layout();
        if satisfies(probe.target_check, layout, target_fact) {
            Ok(())
        } else {
            Err(probe.unsatisfied())
        }
    }
}
