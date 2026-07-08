//! Phase 3, containment source side (`docs/architecture/50-storage.md`
//! § commit step 3; `30-dependencies.md` § enforcement): every inserted
//! fact satisfying a statement's source selection proves its target tuple
//! exists in the final state — scalar tuples by one guard probe, interval
//! positions by the coverage walk. LMDB write transactions read their own
//! writes, so the probes see exactly the state the commit would persist.
//!
//! Also home of the selection machinery shared with the insert phase's
//! `R`-puts: literals encode once per commit into [`Selections`]
//! (never per fact), and [`satisfies`] is a straight byte compare of
//! selected field slices.

use std::ops::Bound;

use crate::encoding::{
    encode_bool, encode_i64, encode_interval_i64, encode_interval_u64, encode_u64, field_bytes,
    FactLayout,
};
use crate::error::{CorruptionError, Direction, Error, Result};
use crate::obs;
use crate::schema::{
    FieldId, LiteralValue, RelationId, Resolved, Schema, StatementDescriptor, StatementId,
};
use crate::storage::delta::{Disposition, WriteDelta};
use crate::storage::env::{ReadTxn, WriteTxn};
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

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
        let checks = delta
            .schema()
            .statements()
            .iter()
            .map(|statement| {
                let StatementDescriptor::Containment { source, target } = &statement.descriptor
                else {
                    return Ok(None);
                };
                Ok(Some(SideChecks {
                    source: encode_selection(delta, view, &source.selection)?,
                    target: encode_selection(delta, view, &target.selection)?,
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
    delta: &WriteDelta<'_>,
    view: &ReadTxn<'_>,
    selection: &[(FieldId, LiteralValue)],
) -> Result<SelectionCheck> {
    if selection.is_empty() {
        return Ok(SelectionCheck::Empty);
    }
    let mut fields = Vec::with_capacity(selection.len());
    for (field, literal) in selection {
        let encoded: Box<[u8]> = match literal {
            LiteralValue::Bool(v) => Box::new([encode_bool(*v)]),
            LiteralValue::Enum(ordinal) => Box::new([*ordinal]),
            LiteralValue::U64(v) => Box::new(encode_u64(*v)),
            LiteralValue::I64(v) => Box::new(encode_i64(*v)),
            LiteralValue::IntervalU64(s, e) => Box::new(encode_interval_u64(*s, *e)),
            LiteralValue::IntervalI64(s, e) => Box::new(encode_interval_i64(*s, *e)),
            LiteralValue::String(raw) => {
                let value =
                    std::str::from_utf8(raw).expect("validated schema: string literals are UTF-8");
                match delta.resolve_str(view, value)? {
                    Some(id) => Box::new(encode_u64(id)),
                    None => return Ok(SelectionCheck::Never),
                }
            }
            LiteralValue::Bytes(raw) => match delta.resolve_bytes(view, raw)? {
                Some(id) => Box::new(encode_u64(id)),
                None => return Ok(SelectionCheck::Never),
            },
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

/// The source-side judgment: for each inserted fact, for each `outgoing`
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
    let mut checker = SourceChecker {
        txn,
        data,
        schema,
        key: [0; MAX_KEY],
    };
    let mut key_bytes = Vec::new();
    let mut probes = 0u64;
    let mut span = obs::span(obs::names::JUDGMENT_SOURCE, obs::Category::Commit);
    for (rel, fact_bytes, disposition) in delta.entries() {
        if disposition != Disposition::Insert {
            continue;
        }
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

/// One satisfying (inserted fact, containment statement) pair: everything
/// a target probe needs, borrowed from the driving loop.
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
}

impl Probe<'_> {
    /// The aborting error: the judgment speaks about sources, so the
    /// payload is the source fact whose target is missing.
    fn unsatisfied(&self) -> Error {
        Error::ContainmentViolation {
            statement: self.statement,
            side: Direction::SourceUnsatisfied,
            fact: self.fact_bytes.into(),
        }
    }
}

/// Working state threaded through the source-side probes.
struct SourceChecker<'a, 'env> {
    txn: &'a WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    schema: &'a Schema,
    key: KeyBuf,
}

impl SourceChecker<'_, '_> {
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
