//! The rejection-evidence codec: one deterministic byte spelling of a
//! complete violated-statement set with bounded, labeled examples. Every
//! violated statement appears; only examples drop under the byte budget, and
//! a skeleton that cannot fit refuses. Examples are canonical rows; strict
//! decode refuses foreign families, unsorted statements and trailing bytes.

use crate::canonical::{CanonicalRow, RowError};
use crate::error::{CitedFact, Violation, Violations};
use crate::work::WorkError;
use crate::{Value, WorkContext};

#[cfg(test)]
use super::judge::{CandidateFact, JudgedViolation};
use super::{RelationId, Schema, StatementId, StatementKind, StatementRef, StatementView};

/// The evidence frame's family magic. Unique to this codec — no command,
/// decision, receipt, head, checkpoint or canonical-row frame shares it.
pub const FAMILY: &[u8] = b"bdb.evidence.v1\0";

/// The frame layout counter; strict decode refuses any other value.
pub const LAYOUT: u16 = 1;

/// The one frame kind under this family.
const EVIDENCE: u8 = 1;

const TAG_FUNCTIONALITY: u8 = 0;
const TAG_CONTAINMENT: u8 = 1;
const TAG_CAPACITY: u8 = 2;

/// `family ‖ layout(u16) ‖ kind(u8) ‖ violation count(u32)`.
const HEADER_LEN: usize = FAMILY.len() + 2 + 1 + 4;
/// `statement(u16) ‖ kind(u8) ‖ truncated(u8) ‖ example count(u32)`.
const VIOLATION_FIXED_LEN: usize = 2 + 1 + 1 + 4;
/// `relation(u32) ‖ fact length(u32)` before the fact bytes.
const EXAMPLE_FIXED_LEN: usize = 4 + 4;

/// Encoding refusal. Every arm is total and typed; none is a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// The operation allowance stopped; the caller retries or refuses —
    /// resource exhaustion never becomes shorter evidence.
    Work(WorkError),
    /// An example fact refused canonical re-encoding.
    Row(RowError),
    /// A rejection is nonempty by construction; refusing an empty set here
    /// keeps "no violations" unrepresentable as evidence.
    Empty,
    /// Violations must arrive in strictly increasing statement-id order
    /// (the judge's canonical diagnostic order, one row per statement).
    Unordered,
    /// The cited statement does not exist in this schema.
    ForeignStatement,
    /// A cited example names a relation this schema does not have.
    ForeignRelation {
        relation: RelationId,
    },
    /// The complete example-free statement skeleton alone exceeds the
    /// budget. The caller must refuse before deciding, never truncate the
    /// statement set.
    Budget {
        needed: usize,
        budget: usize,
    },
    LengthOverflow,
    Allocation,
}

impl From<WorkError> for EvidenceError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<RowError> for EvidenceError {
    fn from(error: RowError) -> Self {
        match error {
            RowError::Work(work) => Self::Work(work),
            other => Self::Row(other),
        }
    }
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rejection evidence encode: {self:?}")
    }
}
impl std::error::Error for EvidenceError {}

/// Strict frame-decode refusal: bounded grammar failures, never partial
/// data. Offsets are byte positions inside the presented frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDecodeError {
    /// The frame exceeds the caller's evidence byte cap.
    LimitExceeded,
    Family,
    Layout {
        got: u16,
    },
    Kind {
        got: u8,
    },
    Truncated {
        at: usize,
    },
    TrailingBytes {
        at: usize,
    },
    Tag {
        at: usize,
        got: u8,
    },
    /// A count field is zero where the grammar requires presence, or
    /// claims more elements than the remaining bytes could hold.
    InvalidCount,
    /// Statement ids must be strictly increasing (each statement appears
    /// exactly once, in canonical diagnostic order).
    Unordered,
    LengthOverflow,
    Allocation,
}

impl std::fmt::Display for EvidenceDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rejection evidence decode: {self:?}")
    }
}
impl std::error::Error for EvidenceDecodeError {}

/// Interpreting decoded evidence against a schema failed: the bytes are
/// grammatical but do not belong to this theory (or an example row refuses
/// the strict canonical decoder). Never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceInterpretError {
    Work(WorkError),
    Row(RowError),
    ForeignStatement {
        statement: StatementId,
    },
    /// The statement exists but has a different kind in this schema.
    KindMismatch {
        statement: StatementId,
    },
    ForeignRelation {
        relation: RelationId,
    },
}

impl From<WorkError> for EvidenceInterpretError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl std::fmt::Display for EvidenceInterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rejection evidence interpretation: {self:?}")
    }
}
impl std::error::Error for EvidenceInterpretError {}

/// One labeled example fact: the relation and the fact's canonical row
/// bytes (opaque at frame level; schema-checked at interpretation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFact {
    pub relation: RelationId,
    pub fact: Box<[u8]>,
}

/// One violated statement as decoded evidence: the stable
/// materialized-order identity, the kind tag,
/// the exact widened capacity measure, bounded examples and the
/// truncation label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceViolation {
    pub statement: StatementId,
    pub kind: StatementKind,
    /// Present exactly for capacity violations: the exact widened total of
    /// the witnessed violating group, untruncated.
    pub measure: Option<u128>,
    pub examples: Box<[EvidenceFact]>,
    /// True when offending facts exist beyond the recorded examples —
    /// either the judge's own example budget or this codec's byte budget
    /// dropped some. The verdict (the statement set) is never truncated.
    pub examples_truncated: bool,
}

/// The decoded complete evidence: every violated statement, in strictly
/// increasing statement-id order, exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViolationEvidence {
    violations: Box<[EvidenceViolation]>,
}

impl ViolationEvidence {
    /// Nonempty, ordered by statement id.
    #[must_use]
    pub fn violations(&self) -> &[EvidenceViolation] {
        &self.violations
    }

    /// Reconstructs judge output from evidence: the exact inverse of
    /// [`encode_judged`] when the byte budget dropped no example.
    ///
    /// # Errors
    /// Refuses evidence foreign to `schema`, malformed example rows, or
    /// stopped work.
    #[cfg(test)]
    pub(crate) fn to_judged(
        &self,
        schema: &Schema,
        work: &WorkContext,
    ) -> Result<Box<[JudgedViolation]>, EvidenceInterpretError> {
        let mut out = Vec::new();
        out.try_reserve_exact(self.violations.len())
            .map_err(|_| EvidenceInterpretError::Row(RowError::Allocation))?;
        for violation in &self.violations {
            work.checkpoint()?;
            check_statement(schema, violation)?;
            let mut examples = Vec::new();
            examples
                .try_reserve_exact(violation.examples.len())
                .map_err(|_| EvidenceInterpretError::Row(RowError::Allocation))?;
            for example in &violation.examples {
                let values = decode_example(schema, example, work)?;
                examples.push(CandidateFact {
                    relation: example.relation,
                    values,
                });
            }
            out.push(JudgedViolation {
                statement: violation.statement,
                kind: violation.kind,
                measure: violation.measure,
                examples: examples.into_boxed_slice(),
                examples_truncated: violation.examples_truncated,
            });
        }
        Ok(out.into_boxed_slice())
    }

    /// Reconstructs the public rejection value ([`Violations`]) so decoded
    /// receipt evidence renders through the ONE existing surface
    /// ([`crate::schema::render_rejection`]) with no second vocabulary.
    /// The convicting `fact` bytes are the first cited example (exactly
    /// how the live rejection path builds them); the citation list is the
    /// decoded example set, and each frame truncation label survives as
    /// [`Violations::examples_truncated`] — so re-encoding a decoded
    /// rejection reproduces the same labels.
    ///
    /// # Errors
    /// Refuses evidence foreign to `schema`, malformed example rows, or
    /// stopped work.
    /// # Panics
    /// Only on programmer-invariant violations (the decoded frame's
    /// internal consistency); never on evidence bytes.
    pub fn to_violations(
        &self,
        schema: &Schema,
        work: &WorkContext,
    ) -> Result<Violations, EvidenceInterpretError> {
        let mut citations = Vec::new();
        citations
            .try_reserve_exact(self.violations.len())
            .map_err(|_| EvidenceInterpretError::Row(RowError::Allocation))?;
        let mut truncated = Vec::new();
        truncated
            .try_reserve_exact(self.violations.len())
            .map_err(|_| EvidenceInterpretError::Row(RowError::Allocation))?;
        for violation in &self.violations {
            work.checkpoint()?;
            let view = check_statement(schema, violation)?;
            let reference = match view {
                StatementView::Key(id, _) => StatementRef::Key(id),
                StatementView::Containment(id, _) => StatementRef::Containment(id),
                StatementView::Capacity(id, _) => StatementRef::Capacity(id),
            };
            let fact: Box<[u8]> = violation
                .examples
                .first()
                .map_or_else(|| Box::<[u8]>::from([]), |example| example.fact.clone());
            let typed = match violation.kind {
                StatementKind::Functionality => Violation::functionality(reference, fact),
                StatementKind::Containment => Violation::containment(reference, fact),
                StatementKind::Capacity => {
                    Violation::capacity(reference, fact, violation.measure.unwrap_or(0))
                }
            };
            let mut cited = Vec::new();
            cited
                .try_reserve_exact(violation.examples.len())
                .map_err(|_| EvidenceInterpretError::Row(RowError::Allocation))?;
            for example in &violation.examples {
                let values = decode_example(schema, example, work)?;
                let field_count = schema
                    .relation_checked(example.relation)
                    .expect("checked by decode_example")
                    .fields()
                    .len();
                cited.push(CitedFact::new(example.relation, field_count, values));
            }
            citations.push((typed, cited.into_boxed_slice()));
            truncated.push(violation.examples_truncated);
        }
        Ok(Violations::from_citations(
            citations.into_boxed_slice(),
            truncated.into_boxed_slice(),
        ))
    }
}

fn check_statement<'s>(
    schema: &'s Schema,
    violation: &EvidenceViolation,
) -> Result<StatementView<'s>, EvidenceInterpretError> {
    let view = schema.statement_checked(violation.statement).ok_or(
        EvidenceInterpretError::ForeignStatement {
            statement: violation.statement,
        },
    )?;
    let kind = match view {
        StatementView::Key(..) => StatementKind::Functionality,
        StatementView::Containment(..) => StatementKind::Containment,
        StatementView::Capacity(..) => StatementKind::Capacity,
    };
    if kind == violation.kind {
        Ok(view)
    } else {
        Err(EvidenceInterpretError::KindMismatch {
            statement: violation.statement,
        })
    }
}

fn decode_example(
    schema: &Schema,
    example: &EvidenceFact,
    work: &WorkContext,
) -> Result<Box<[Value]>, EvidenceInterpretError> {
    let relation = schema.relation_checked(example.relation).ok_or(
        EvidenceInterpretError::ForeignRelation {
            relation: example.relation,
        },
    )?;
    let decoded =
        crate::canonical::decode(relation.fields(), &example.fact, work).map_err(|error| {
            match error {
                RowError::Work(work) => EvidenceInterpretError::Work(work),
                other => EvidenceInterpretError::Row(other),
            }
        })?;
    Ok(decoded.values().to_vec().into_boxed_slice())
}

/// One violation prepared for framing: fixed fields plus pre-encoded
/// canonical example rows (their work reservations stay alive until the
/// frame bytes are written).
struct Part {
    statement: StatementId,
    kind: StatementKind,
    measure: Option<u128>,
    judge_truncated: bool,
    examples: Vec<(RelationId, CanonicalRow)>,
}

impl Part {
    fn fixed_len(&self) -> usize {
        VIOLATION_FIXED_LEN
            + match self.kind {
                StatementKind::Functionality | StatementKind::Containment => 0,
                StatementKind::Capacity => 16,
            }
    }
}

/// Encodes a rejection under a byte budget; deterministic for a given
/// `(schema, violations, max_bytes)`.
///
/// The public [`Violations`] carries the judge's own per-statement
/// example-truncation label ([`Violations::examples_truncated`]); the
/// frame's truncation label is the OR of that label and this codec's own
/// byte-budget drops.
///
/// # Errors
/// Refuses stopped work, malformed examples, foreign statements/relations,
/// an unordered or empty violation set, and a budget the complete statement
/// skeleton cannot fit ([`EvidenceError::Budget`] — refuse before
/// deciding, never a shorter verdict).
pub fn encode_violations(
    schema: &Schema,
    violations: &Violations,
    max_bytes: usize,
    work: &WorkContext,
) -> Result<Vec<u8>, EvidenceError> {
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(violations.len())
        .map_err(|_| EvidenceError::Allocation)?;
    for (index, (violation, cited)) in violations.citations().enumerate() {
        work.checkpoint()?;
        let (statement, kind) = statement_slot(schema, violation.statement())?;
        let measure = match violation {
            Violation::Functionality { .. } | Violation::Containment { .. } => None,
            Violation::Capacity { measure, .. } => Some(*measure),
        };
        let mut examples = Vec::new();
        examples
            .try_reserve_exact(cited.len())
            .map_err(|_| EvidenceError::Allocation)?;
        for fact in cited {
            examples.push(encode_example(
                schema,
                fact.relation(),
                fact.values(),
                work,
            )?);
        }
        parts.push(Part {
            statement,
            kind,
            measure,
            // The judge's own per-statement label, preserved across the
            // public boundary by `api::db`'s rejection bridge.
            judge_truncated: violations.examples_truncated(index),
            examples,
        });
    }
    encode_parts(&parts, max_bytes, work)
}

/// Encodes judge output; `decode(bytes)?.to_judged(schema, work)` reproduces
/// it, truncation labels included, when the byte budget dropped nothing.
///
/// # Errors
/// As [`encode_violations`].
#[cfg(test)]
pub(crate) fn encode_judged(
    schema: &Schema,
    judged: &[JudgedViolation],
    max_bytes: usize,
    work: &WorkContext,
) -> Result<Vec<u8>, EvidenceError> {
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(judged.len())
        .map_err(|_| EvidenceError::Allocation)?;
    for violation in judged {
        work.checkpoint()?;
        let measure = match violation.kind {
            StatementKind::Capacity => Some(violation.measure.unwrap_or(0)),
            StatementKind::Functionality | StatementKind::Containment => None,
        };
        let mut examples = Vec::new();
        examples
            .try_reserve_exact(violation.examples.len())
            .map_err(|_| EvidenceError::Allocation)?;
        for example in &violation.examples {
            examples.push(encode_example(
                schema,
                example.relation,
                &example.values,
                work,
            )?);
        }
        parts.push(Part {
            statement: violation.statement,
            kind: violation.kind,
            measure,
            judge_truncated: violation.examples_truncated,
            examples,
        });
    }
    encode_parts(&parts, max_bytes, work)
}

fn statement_slot(
    schema: &Schema,
    reference: StatementRef,
) -> Result<(StatementId, StatementKind), EvidenceError> {
    match reference {
        StatementRef::Key(id) => schema
            .key_checked(id)
            .map(|statement| (statement.id, StatementKind::Functionality)),
        StatementRef::Containment(id) => schema
            .containment_checked(id)
            .map(|statement| (statement.id, StatementKind::Containment)),
        StatementRef::Capacity(id) => schema
            .capacity_checked(id)
            .map(|statement| (statement.id, StatementKind::Capacity)),
    }
    .ok_or(EvidenceError::ForeignStatement)
}

fn encode_example(
    schema: &Schema,
    relation: RelationId,
    values: &[Value],
    work: &WorkContext,
) -> Result<(RelationId, CanonicalRow), EvidenceError> {
    let sealed = schema
        .relation_checked(relation)
        .ok_or(EvidenceError::ForeignRelation { relation })?;
    let row = CanonicalRow::encode(sealed.fields(), values, work)?;
    Ok((relation, row))
}

fn encode_parts(
    parts: &[Part],
    max_bytes: usize,
    work: &WorkContext,
) -> Result<Vec<u8>, EvidenceError> {
    if parts.is_empty() {
        return Err(EvidenceError::Empty);
    }
    for pair in parts.windows(2) {
        if pair[1].statement <= pair[0].statement {
            return Err(EvidenceError::Unordered);
        }
    }

    // The complete example-free statement skeleton must fit whole.
    let mut skeleton = HEADER_LEN;
    for part in parts {
        skeleton = skeleton
            .checked_add(part.fixed_len())
            .ok_or(EvidenceError::LengthOverflow)?;
    }
    if skeleton > max_bytes {
        return Err(EvidenceError::Budget {
            needed: skeleton,
            budget: max_bytes,
        });
    }

    // Deterministic truncation: the largest UNIFORM per-violation example
    // count `k` whose encoding fits the budget. Examples are added one
    // index at a time across all violations (round-robin by rank), so the
    // kept set is a pure function of the input and the budget.
    let max_rank = parts
        .iter()
        .map(|part| part.examples.len())
        .max()
        .unwrap_or(0);
    let mut total = skeleton;
    let mut keep = 0usize;
    'ranks: for rank in 0..max_rank {
        let mut next = total;
        for part in parts {
            work.checkpoint()?;
            if let Some((_, row)) = part.examples.get(rank) {
                let example_len = EXAMPLE_FIXED_LEN
                    .checked_add(row.as_bytes().len())
                    .ok_or(EvidenceError::LengthOverflow)?;
                next = next
                    .checked_add(example_len)
                    .ok_or(EvidenceError::LengthOverflow)?;
            }
        }
        if next > max_bytes {
            break 'ranks;
        }
        total = next;
        keep = rank + 1;
    }

    if u32::try_from(parts.len()).is_err() {
        return Err(EvidenceError::LengthOverflow);
    }

    work.checkpoint()?;
    let mut out = Vec::new();
    out.try_reserve_exact(total)
        .map_err(|_| EvidenceError::Allocation)?;
    out.extend_from_slice(FAMILY);
    out.extend_from_slice(&LAYOUT.to_be_bytes());
    out.push(EVIDENCE);
    out.extend_from_slice(
        &u32::try_from(parts.len())
            .expect("checked above")
            .to_be_bytes(),
    );
    for part in parts {
        let kept = part.examples.len().min(keep);
        out.extend_from_slice(&part.statement.0.to_be_bytes());
        // The detail slots are kind-driven so the writer and the sizing
        // arithmetic agree by construction (a capacity always carries its
        // measure word).
        match part.kind {
            StatementKind::Functionality => out.push(TAG_FUNCTIONALITY),
            StatementKind::Containment => out.push(TAG_CONTAINMENT),
            StatementKind::Capacity => {
                out.push(TAG_CAPACITY);
                out.extend_from_slice(&part.measure.unwrap_or(0).to_be_bytes());
            }
        }
        let truncated = part.judge_truncated || kept < part.examples.len();
        out.push(u8::from(truncated));
        out.extend_from_slice(
            &u32::try_from(kept)
                .map_err(|_| EvidenceError::LengthOverflow)?
                .to_be_bytes(),
        );
        for (relation, row) in part.examples.iter().take(kept) {
            out.extend_from_slice(&relation.0.to_be_bytes());
            out.extend_from_slice(
                &u32::try_from(row.as_bytes().len())
                    .map_err(|_| EvidenceError::LengthOverflow)?
                    .to_be_bytes(),
            );
            out.extend_from_slice(row.as_bytes());
        }
    }
    debug_assert_eq!(out.len(), total, "size arithmetic matches the writer");
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], EvidenceDecodeError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(EvidenceDecodeError::LengthOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(EvidenceDecodeError::Truncated { at: self.at })?;
        self.at = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], EvidenceDecodeError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
}

/// Strict, schema-free frame decode. `max_bytes` is the caller's evidence
/// cap. Example facts stay opaque
/// canonical row bytes; interpret them with
/// [`ViolationEvidence::to_violations`].
///
/// # Errors
/// Every grammar refusal in [`EvidenceDecodeError`]; never partial data.
pub fn decode(bytes: &[u8], max_bytes: usize) -> Result<ViolationEvidence, EvidenceDecodeError> {
    if bytes.len() > max_bytes {
        return Err(EvidenceDecodeError::LimitExceeded);
    }
    let mut input = Reader { bytes, at: 0 };
    if input.take(FAMILY.len())? != FAMILY {
        return Err(EvidenceDecodeError::Family);
    }
    let layout = u16::from_be_bytes(input.array()?);
    if layout != LAYOUT {
        return Err(EvidenceDecodeError::Layout { got: layout });
    }
    let kind = input.array::<1>()?[0];
    if kind != EVIDENCE {
        return Err(EvidenceDecodeError::Kind { got: kind });
    }
    let count = u32::from_be_bytes(input.array()?) as usize;
    if count == 0 || count > input.remaining() / VIOLATION_FIXED_LEN {
        return Err(EvidenceDecodeError::InvalidCount);
    }
    let mut violations = Vec::new();
    violations
        .try_reserve_exact(count)
        .map_err(|_| EvidenceDecodeError::Allocation)?;
    let mut previous: Option<StatementId> = None;
    for _ in 0..count {
        let statement = StatementId(u16::from_be_bytes(input.array()?));
        if previous.is_some_and(|last| statement <= last) {
            return Err(EvidenceDecodeError::Unordered);
        }
        previous = Some(statement);
        let at = input.at;
        let tag = input.array::<1>()?[0];
        let (kind, measure) = match tag {
            TAG_FUNCTIONALITY => (StatementKind::Functionality, None),
            TAG_CONTAINMENT => (StatementKind::Containment, None),
            TAG_CAPACITY => {
                let measure = u128::from_be_bytes(input.array()?);
                (StatementKind::Capacity, Some(measure))
            }
            got => return Err(EvidenceDecodeError::Tag { at, got }),
        };
        let at = input.at;
        let truncated = match input.array::<1>()?[0] {
            0 => false,
            1 => true,
            got => return Err(EvidenceDecodeError::Tag { at, got }),
        };
        let example_count = u32::from_be_bytes(input.array()?) as usize;
        if example_count > input.remaining() / EXAMPLE_FIXED_LEN {
            return Err(EvidenceDecodeError::InvalidCount);
        }
        let mut examples = Vec::new();
        examples
            .try_reserve_exact(example_count)
            .map_err(|_| EvidenceDecodeError::Allocation)?;
        for _ in 0..example_count {
            let relation = RelationId(u32::from_be_bytes(input.array()?));
            let length = u32::from_be_bytes(input.array()?) as usize;
            let span = input.take(length)?;
            let mut fact = Vec::new();
            fact.try_reserve_exact(span.len())
                .map_err(|_| EvidenceDecodeError::Allocation)?;
            fact.extend_from_slice(span);
            examples.push(EvidenceFact {
                relation,
                fact: fact.into_boxed_slice(),
            });
        }
        violations.push(EvidenceViolation {
            statement,
            kind,
            measure,
            examples: examples.into_boxed_slice(),
            examples_truncated: truncated,
        });
    }
    if input.at != bytes.len() {
        return Err(EvidenceDecodeError::TrailingBytes { at: input.at });
    }
    Ok(ViolationEvidence {
        violations: violations.into_boxed_slice(),
    })
}

#[cfg(test)]
mod tests;
