//! The batch footprint: a pure function of the schema descriptor and
//! the raw-valued ops, recomputable by any consumer (L6 is the
//! soundness obligation its keys serve). Keys are blake3 over tagged
//! raw values — state-independent, so equal keys mean equal values
//! across writers and stores; no intern id ever reaches a key.
//!
//! In-batch net disposition comes first: per fact id the last op wins,
//! and every derived entry (F, K, C, W) is emitted from the surviving
//! net rows. The capacity child delta is the signed sum of the net
//! rows' weights — an op-derived bound, never an effect claim; the
//! evaporation interval around it is [`CapacityProfile`]'s job.

use std::collections::BTreeMap;

use bumbledb::Value;
use bumbledb::schema::{
    LiteralSet, RelationId, SchemaDescriptor, StatementDescriptor, StatementId, ValueType, Weight,
};

use crate::codec::{
    CLASS_CAPACITY, CLASS_CONTAINMENT, CLASS_FACT, CLASS_KEY, NullSink, Op, OpKind, ValueShape,
    append_value,
};

/// The C-class entry mode: a source insert needs the target group; a
/// target insert or delete moves its support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContainmentMode {
    Need,
    SupportAdd,
    SupportRemove,
}

impl ContainmentMode {
    pub(crate) const fn wire(self) -> u8 {
        match self {
            Self::Need => 1,
            Self::SupportAdd => 2,
            Self::SupportRemove => 3,
        }
    }
}

/// The W-class entry mode: the one place a number is representable is
/// the child-delta arm — a mode on any other class is unencodable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapacityMode {
    ChildDelta(i64),
    ParentAdd,
    ParentRemove,
}

impl CapacityMode {
    pub(crate) const fn wire(self) -> u8 {
        match self {
            Self::ChildDelta(_) => 1,
            Self::ParentAdd => 2,
            Self::ParentRemove => 3,
        }
    }
}

/// One footprint entry. The per-class shapes make illegal combinations
/// unrepresentable: F carries no statement, K carries no mode, and the
/// delta exists only inside [`CapacityMode::ChildDelta`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Entry {
    Fact {
        fid: [u8; 32],
        mode: OpKind,
    },
    Key {
        statement: StatementId,
        key: [u8; 32],
    },
    Containment {
        statement: StatementId,
        key: [u8; 32],
        mode: ContainmentMode,
    },
    Capacity {
        statement: StatementId,
        key: [u8; 32],
        mode: CapacityMode,
    },
}

impl Entry {
    /// The section's sort-and-identity tuple: (class, statement, key,
    /// mode), one entry per tuple. F sorts under its class with the
    /// statement slot zero (no statement field exists for F), and the
    /// child delta is payload, not identity.
    #[must_use]
    pub fn sort_key(&self) -> (u8, u16, [u8; 32], u8) {
        match self {
            Self::Fact { fid, mode } => (CLASS_FACT, 0, *fid, mode.wire()),
            Self::Key { statement, key } => (CLASS_KEY, statement.0, *key, 0),
            Self::Containment {
                statement,
                key,
                mode,
            } => (CLASS_CONTAINMENT, statement.0, *key, mode.wire()),
            Self::Capacity {
                statement,
                key,
                mode,
            } => (CLASS_CAPACITY, statement.0, *key, mode.wire()),
        }
    }

    /// The share coordinate: identity minus the mode — two batches
    /// share a key when these coincide, whatever the modes say.
    #[must_use]
    pub fn share_key(&self) -> (u8, u16, [u8; 32]) {
        let (class, statement, key, _) = self.sort_key();
        (class, statement, key)
    }
}

/// Descriptor-parse refusals: the vocabulary is built parse-all-first,
/// so emission never re-checks statement shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyError {
    StatementCountOverflow,
    UnknownRelation {
        statement: StatementId,
        relation: RelationId,
    },
    UnknownField {
        statement: StatementId,
        relation: RelationId,
        field: u16,
    },
    ProjectionArityMismatch {
        statement: StatementId,
    },
    ProjectionTypeMismatch {
        statement: StatementId,
        position: usize,
    },
    SelectionLiteralShape {
        statement: StatementId,
        relation: RelationId,
        field: u16,
    },
    WeightFieldShape {
        statement: StatementId,
        field: u16,
    },
}

/// Emission refusals: typed, naming op, relation, row, and field where
/// one exists. The delta overflow names the statement and parent key
/// whose signed sum left the wire's i64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootprintError {
    UnknownRelation {
        op: usize,
        relation: RelationId,
    },
    ClosedRelation {
        op: usize,
        relation: RelationId,
    },
    Arity {
        op: usize,
        relation: RelationId,
        row: usize,
    },
    Value {
        op: usize,
        relation: RelationId,
        row: usize,
        field: u16,
        cause: ValueShape,
    },
    WeightShape {
        op: usize,
        relation: RelationId,
        row: usize,
        statement: StatementId,
    },
    DeltaOverflow {
        statement: StatementId,
        key: [u8; 32],
    },
}

#[derive(Debug, Clone)]
struct KeyRole {
    statement: StatementId,
    projection: Box<[u16]>,
}

#[derive(Debug, Clone)]
struct SideRole {
    statement: StatementId,
    projection: Box<[u16]>,
    selection: Box<[(u16, LiteralSet)]>,
}

#[derive(Debug, Clone)]
struct ChildRole {
    statement: StatementId,
    projection: Box<[u16]>,
    selection: Box<[(u16, LiteralSet)]>,
    weight: Weight,
}

/// One relation's derived emission view.
#[derive(Debug, Clone)]
pub struct RelationInfo {
    ordinary: bool,
    layout: Box<[ValueType]>,
    keys: Box<[KeyRole]>,
    needs: Box<[SideRole]>,
    supports: Box<[SideRole]>,
    children: Box<[ChildRole]>,
    parents: Box<[SideRole]>,
}

impl RelationInfo {
    #[must_use]
    pub const fn is_ordinary(&self) -> bool {
        self.ordinary
    }

    #[must_use]
    pub const fn layout(&self) -> &[ValueType] {
        &self.layout
    }
}

/// The descriptor parsed for emission: per relation, its layout and
/// every statement role it plays. Closed relations carry no roles, and
/// closed-target statements contribute none anywhere — closed
/// statements are conflict-free by construction.
#[derive(Debug, Clone)]
pub struct Vocabulary {
    relations: Box<[RelationInfo]>,
}

impl Vocabulary {
    /// Parses the descriptor once; every later call trusts the result.
    pub fn new(descriptor: &SchemaDescriptor) -> Result<Self, VocabularyError> {
        let mut relations: Vec<RelationInfo> = descriptor
            .relations
            .iter()
            .map(|relation| RelationInfo {
                ordinary: relation.extension.is_none(),
                layout: relation
                    .fields
                    .iter()
                    .map(|field| field.value_type)
                    .collect(),
                keys: Box::from([]),
                needs: Box::from([]),
                supports: Box::from([]),
                children: Box::from([]),
                parents: Box::from([]),
            })
            .collect();

        let mut bins = RoleBins::new(relations.len());

        let statements = descriptor.materialized_statements();
        for (index, statement) in statements.iter().enumerate() {
            let id = StatementId(
                u16::try_from(index).map_err(|_| VocabularyError::StatementCountOverflow)?,
            );
            match statement {
                StatementDescriptor::Functionality {
                    relation,
                    projection,
                } => {
                    let info = ordinary(&relations, *relation, id)?;
                    let Some(info) = info else { continue };
                    let projection =
                        checked_projection(info, *relation, projection.iter().map(|f| f.0), id)?;
                    bins.keys[relation_index(*relation)].push(KeyRole {
                        statement: id,
                        projection,
                    });
                }
                StatementDescriptor::Containment { source, target } => {
                    add_statement(&relations, &mut bins, id, source, target, None)?;
                }
                StatementDescriptor::Capacity {
                    target,
                    weight,
                    source,
                    ..
                } => {
                    add_statement(&relations, &mut bins, id, source, target, Some(*weight))?;
                }
            }
        }

        for (index, info) in relations.iter_mut().enumerate() {
            info.keys = std::mem::take(&mut bins.keys[index]).into_boxed_slice();
            info.needs = std::mem::take(&mut bins.needs[index]).into_boxed_slice();
            info.supports = std::mem::take(&mut bins.supports[index]).into_boxed_slice();
            info.children = std::mem::take(&mut bins.children[index]).into_boxed_slice();
            info.parents = std::mem::take(&mut bins.parents[index]).into_boxed_slice();
        }

        Ok(Self {
            relations: relations.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn relation(&self, id: RelationId) -> Option<&RelationInfo> {
        self.relations.get(relation_index(id))
    }

    fn ordinary_info(
        &self,
        op: usize,
        relation: RelationId,
    ) -> Result<&RelationInfo, FootprintError> {
        match self.relation(relation) {
            Some(info) if info.ordinary => Ok(info),
            Some(_) => Err(FootprintError::ClosedRelation { op, relation }),
            None => Err(FootprintError::UnknownRelation { op, relation }),
        }
    }
}

fn relation_index(id: RelationId) -> usize {
    usize::try_from(id.0).expect("u32 fits usize")
}

struct RoleBins {
    keys: Vec<Vec<KeyRole>>,
    needs: Vec<Vec<SideRole>>,
    supports: Vec<Vec<SideRole>>,
    children: Vec<Vec<ChildRole>>,
    parents: Vec<Vec<SideRole>>,
}

impl RoleBins {
    fn new(count: usize) -> Self {
        Self {
            keys: vec![Vec::new(); count],
            needs: vec![Vec::new(); count],
            supports: vec![Vec::new(); count],
            children: vec![Vec::new(); count],
            parents: vec![Vec::new(); count],
        }
    }
}

/// One two-sided statement's roles: source-side emission (`need` or
/// weighted child, per the weight argument) and target-side emission
/// (`support` moves or parent moves). A closed target drops the whole
/// statement; a closed source drops only the source-side roles.
fn add_statement(
    relations: &[RelationInfo],
    bins: &mut RoleBins,
    id: StatementId,
    source: &bumbledb::schema::Side,
    target: &bumbledb::schema::Side,
    weight: Option<Weight>,
) -> Result<(), VocabularyError> {
    let Some(target_info) = ordinary(relations, target.relation, id)? else {
        return Ok(());
    };
    let target_projection = checked_projection(
        target_info,
        target.relation,
        target.projection.iter().map(|f| f.0),
        id,
    )?;
    let target_selection = checked_selection(target_info, target.relation, &target.selection, id)?;

    if let Some(source_info) = ordinary(relations, source.relation, id)? {
        let source_projection = checked_projection(
            source_info,
            source.relation,
            source.projection.iter().map(|f| f.0),
            id,
        )?;
        check_pairing(
            source_info,
            &source_projection,
            target_info,
            &target_projection,
            id,
        )?;
        let source_selection =
            checked_selection(source_info, source.relation, &source.selection, id)?;
        match weight {
            None => bins.needs[relation_index(source.relation)].push(SideRole {
                statement: id,
                projection: source_projection,
                selection: source_selection,
            }),
            Some(weight) => {
                check_weight(source_info, weight, id)?;
                bins.children[relation_index(source.relation)].push(ChildRole {
                    statement: id,
                    projection: source_projection,
                    selection: source_selection,
                    weight,
                });
            }
        }
    }

    let target_role = SideRole {
        statement: id,
        projection: target_projection,
        selection: target_selection,
    };
    match weight {
        None => bins.supports[relation_index(target.relation)].push(target_role),
        Some(_) => bins.parents[relation_index(target.relation)].push(target_role),
    }
    Ok(())
}

/// A statement leg lands on an ordinary relation, a closed one (the
/// statement contributes nothing), or nowhere (a parse refusal).
fn ordinary(
    relations: &[RelationInfo],
    relation: RelationId,
    statement: StatementId,
) -> Result<Option<&RelationInfo>, VocabularyError> {
    let info = relations
        .get(relation_index(relation))
        .ok_or(VocabularyError::UnknownRelation {
            statement,
            relation,
        })?;
    Ok(info.ordinary.then_some(info))
}

fn checked_projection<I: Iterator<Item = u16>>(
    info: &RelationInfo,
    relation: RelationId,
    fields: I,
    statement: StatementId,
) -> Result<Box<[u16]>, VocabularyError> {
    fields
        .map(|field| {
            if usize::from(field) < info.layout.len() {
                Ok(field)
            } else {
                Err(VocabularyError::UnknownField {
                    statement,
                    relation,
                    field,
                })
            }
        })
        .collect()
}

fn checked_selection(
    info: &RelationInfo,
    relation: RelationId,
    selection: &[(bumbledb::schema::FieldId, LiteralSet)],
    statement: StatementId,
) -> Result<Box<[(u16, LiteralSet)]>, VocabularyError> {
    selection
        .iter()
        .map(|(field, literals)| {
            let index = field.0;
            let ty = info
                .layout
                .get(usize::from(index))
                .ok_or(VocabularyError::UnknownField {
                    statement,
                    relation,
                    field: index,
                })?;
            for literal in literals.literals() {
                if append_value(&mut NullSink, literal, *ty).is_err() {
                    return Err(VocabularyError::SelectionLiteralShape {
                        statement,
                        relation,
                        field: index,
                    });
                }
            }
            Ok((index, literals.clone()))
        })
        .collect()
}

fn check_pairing(
    source: &RelationInfo,
    source_projection: &[u16],
    target: &RelationInfo,
    target_projection: &[u16],
    statement: StatementId,
) -> Result<(), VocabularyError> {
    if source_projection.len() != target_projection.len() {
        return Err(VocabularyError::ProjectionArityMismatch { statement });
    }
    for (position, (s, t)) in source_projection
        .iter()
        .zip(target_projection.iter())
        .enumerate()
    {
        if source.layout[usize::from(*s)] != target.layout[usize::from(*t)] {
            return Err(VocabularyError::ProjectionTypeMismatch {
                statement,
                position,
            });
        }
    }
    Ok(())
}

fn check_weight(
    source: &RelationInfo,
    weight: Weight,
    statement: StatementId,
) -> Result<(), VocabularyError> {
    match weight {
        Weight::Unit => Ok(()),
        Weight::Field(field) => match source.layout.get(usize::from(field.0)) {
            Some(ValueType::U64) => Ok(()),
            _ => Err(VocabularyError::WeightFieldShape {
                statement,
                field: field.0,
            }),
        },
        Weight::DurationOf(field) => match source.layout.get(usize::from(field.0)) {
            Some(ValueType::Interval { .. } | ValueType::FixedInterval { .. }) => Ok(()),
            _ => Err(VocabularyError::WeightFieldShape {
                statement,
                field: field.0,
            }),
        },
    }
}

/// One net-surviving row: per fact id, the batch's last op wins, and
/// every emitted entry derives from these survivors.
pub(crate) struct NetRow {
    pub(crate) op: usize,
    pub(crate) row: usize,
    pub(crate) fid: [u8; 32],
    pub(crate) mode: OpKind,
}

struct RowCtx {
    op: usize,
    relation: RelationId,
    row: usize,
}

fn hash_row(
    layout: &[ValueType],
    prefix: &[u8],
    fields: impl Iterator<Item = u16>,
    row: &[Value],
    ctx: &RowCtx,
) -> Result<[u8; 32], FootprintError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prefix);
    for field in fields {
        let index = usize::from(field);
        append_value(&mut hasher, &row[index], layout[index]).map_err(|cause| {
            FootprintError::Value {
                op: ctx.op,
                relation: ctx.relation,
                row: ctx.row,
                field,
                cause,
            }
        })?;
    }
    Ok(*hasher.finalize().as_bytes())
}

/// `fid = blake3(relation_id_le ++ tagged raw values of the full row)`.
fn fact_id(
    relation: RelationId,
    layout: &[ValueType],
    row: &[Value],
    ctx: &RowCtx,
) -> Result<[u8; 32], FootprintError> {
    hash_row(
        layout,
        &relation.0.to_le_bytes(),
        0..field_count(layout),
        row,
        ctx,
    )
}

/// `fkey = blake3(statement_id_le ++ tagged raw values of the
/// projection, in field order)`.
fn footprint_key(
    statement: StatementId,
    layout: &[ValueType],
    projection: &[u16],
    row: &[Value],
    ctx: &RowCtx,
) -> Result<[u8; 32], FootprintError> {
    hash_row(
        layout,
        &statement.0.to_le_bytes(),
        projection.iter().copied(),
        row,
        ctx,
    )
}

fn field_count(layout: &[ValueType]) -> u16 {
    u16::try_from(layout.len()).expect("field count fits u16")
}

fn selected(row: &[Value], selection: &[(u16, LiteralSet)]) -> bool {
    selection.iter().all(|(field, literals)| {
        literals
            .literals()
            .iter()
            .any(|literal| *literal == row[usize::from(*field)])
    })
}

fn weight_of(
    weight: Weight,
    row: &[Value],
    statement: StatementId,
    ctx: &RowCtx,
) -> Result<u64, FootprintError> {
    let shape = || FootprintError::WeightShape {
        op: ctx.op,
        relation: ctx.relation,
        row: ctx.row,
        statement,
    };
    match weight {
        Weight::Unit => Ok(1),
        Weight::Field(field) => match row.get(usize::from(field.0)) {
            Some(Value::U64(value)) => Ok(*value),
            _ => Err(shape()),
        },
        Weight::DurationOf(field) => match row.get(usize::from(field.0)) {
            Some(Value::IntervalU64(interval)) => Ok(interval.end() - interval.start()),
            Some(Value::IntervalI64(interval)) => Ok(interval.end().abs_diff(interval.start())),
            _ => Err(shape()),
        },
    }
}

pub(crate) fn net_rows(vocabulary: &Vocabulary, ops: &[Op]) -> Result<Vec<NetRow>, FootprintError> {
    struct Candidate {
        fid: [u8; 32],
        seq: usize,
        op: usize,
        row: usize,
        mode: OpKind,
    }

    let mut candidates: Vec<Candidate> =
        Vec::with_capacity(ops.iter().map(|op| op.rows.len()).sum());
    let mut seq = 0usize;
    for (op_index, op) in ops.iter().enumerate() {
        let info = vocabulary.ordinary_info(op_index, op.relation)?;
        for (row_index, row) in op.rows.iter().enumerate() {
            if row.len() != info.layout.len() {
                return Err(FootprintError::Arity {
                    op: op_index,
                    relation: op.relation,
                    row: row_index,
                });
            }
            let ctx = RowCtx {
                op: op_index,
                relation: op.relation,
                row: row_index,
            };
            let fid = fact_id(op.relation, &info.layout, row, &ctx)?;
            candidates.push(Candidate {
                fid,
                seq,
                op: op_index,
                row: row_index,
                mode: op.kind,
            });
            seq += 1;
        }
    }

    candidates.sort_unstable_by_key(|candidate| (candidate.fid, candidate.seq));

    let mut net: Vec<NetRow> = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let survivor = NetRow {
            op: candidate.op,
            row: candidate.row,
            fid: candidate.fid,
            mode: candidate.mode,
        };
        match net.last_mut() {
            Some(last) if last.fid == candidate.fid => *last = survivor,
            _ => net.push(survivor),
        }
    }
    Ok(net)
}

/// The pure footprint derivation: sorted ascending by (class,
/// statement, key, mode), duplicates merged, child deltas summed per
/// parent key into one signed number (overflow is a typed refusal —
/// such a batch is unencodable).
pub fn footprint(vocabulary: &Vocabulary, ops: &[Op]) -> Result<Vec<Entry>, FootprintError> {
    let net = net_rows(vocabulary, ops)?;

    let mut entries: Vec<Entry> = Vec::new();
    let mut deltas: Vec<(StatementId, [u8; 32], i128)> = Vec::new();

    for survivor in &net {
        let op = &ops[survivor.op];
        let info = vocabulary.ordinary_info(survivor.op, op.relation)?;
        let row = &op.rows[survivor.row];
        let ctx = RowCtx {
            op: survivor.op,
            relation: op.relation,
            row: survivor.row,
        };

        entries.push(Entry::Fact {
            fid: survivor.fid,
            mode: survivor.mode,
        });

        for role in &info.keys {
            let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
            entries.push(Entry::Key {
                statement: role.statement,
                key,
            });
        }

        if survivor.mode == OpKind::Insert {
            for role in &info.needs {
                if selected(row, &role.selection) {
                    let key =
                        footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                    entries.push(Entry::Containment {
                        statement: role.statement,
                        key,
                        mode: ContainmentMode::Need,
                    });
                }
            }
        }

        for role in &info.supports {
            if selected(row, &role.selection) {
                let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                let mode = match survivor.mode {
                    OpKind::Insert => ContainmentMode::SupportAdd,
                    OpKind::Delete => ContainmentMode::SupportRemove,
                };
                entries.push(Entry::Containment {
                    statement: role.statement,
                    key,
                    mode,
                });
            }
        }

        for role in &info.children {
            if selected(row, &role.selection) {
                let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                let weight = i128::from(weight_of(role.weight, row, role.statement, &ctx)?);
                let signed = match survivor.mode {
                    OpKind::Insert => weight,
                    OpKind::Delete => -weight,
                };
                deltas.push((role.statement, key, signed));
            }
        }

        for role in &info.parents {
            if selected(row, &role.selection) {
                let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                let mode = match survivor.mode {
                    OpKind::Insert => CapacityMode::ParentAdd,
                    OpKind::Delete => CapacityMode::ParentRemove,
                };
                entries.push(Entry::Capacity {
                    statement: role.statement,
                    key,
                    mode,
                });
            }
        }
    }

    deltas.sort_unstable_by_key(|entry| (entry.0, entry.1));
    let mut start = 0;
    while start < deltas.len() {
        let (statement, key, mut sum) = deltas[start];
        let mut end = start + 1;
        while end < deltas.len() && deltas[end].0 == statement && deltas[end].1 == key {
            sum += deltas[end].2;
            end += 1;
        }
        let delta =
            i64::try_from(sum).map_err(|_| FootprintError::DeltaOverflow { statement, key })?;
        entries.push(Entry::Capacity {
            statement,
            key,
            mode: CapacityMode::ChildDelta(delta),
        });
        start = end;
    }

    entries.sort_unstable_by_key(Entry::sort_key);
    entries.dedup();
    Ok(entries)
}

/// One parent group's coordinate: the capacity statement and the
/// parent determinant key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapacityKey {
    pub statement: StatementId,
    pub key: [u8; 32],
}

/// One batch's quantitative posture at one parent group: the published
/// signed delta, the evaporation widening on each side (net inserts
/// can evaporate downward, net deletes upward), and the parent row's
/// own moves. The effective delta at any reachable base lies in
/// [`Self::min`], [`Self::max`] — the interval the W commute test
/// consumes (L7's hypothesis on the measured class).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapacityProfile {
    pub delta: i128,
    pub widen_down: i128,
    pub widen_up: i128,
    pub parent_add: bool,
    pub parent_remove: bool,
}

impl CapacityProfile {
    #[must_use]
    pub const fn min(&self) -> i128 {
        self.delta - self.widen_down
    }

    #[must_use]
    pub const fn max(&self) -> i128 {
        self.delta + self.widen_up
    }

    /// The exact point interval [0, 0] with no parent moves — the one
    /// posture that commutes with a parent removal.
    #[must_use]
    pub const fn is_inert(&self) -> bool {
        self.min() == 0 && self.max() == 0 && !self.parent_add && !self.parent_remove
    }
}

/// Recomputes every W-class coordinate a batch touches, with its
/// evaporation-widened interval — recomputable by any intersector from
/// the ops it already holds, so re-running the arithmetic against a
/// winner-updated measure needs no wire change.
pub fn capacity_profiles(
    vocabulary: &Vocabulary,
    ops: &[Op],
) -> Result<BTreeMap<CapacityKey, CapacityProfile>, FootprintError> {
    let net = net_rows(vocabulary, ops)?;
    let mut profiles: BTreeMap<CapacityKey, CapacityProfile> = BTreeMap::new();

    for survivor in &net {
        let op = &ops[survivor.op];
        let info = vocabulary.ordinary_info(survivor.op, op.relation)?;
        let row = &op.rows[survivor.row];
        let ctx = RowCtx {
            op: survivor.op,
            relation: op.relation,
            row: survivor.row,
        };

        for role in &info.children {
            if selected(row, &role.selection) {
                let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                let weight = i128::from(weight_of(role.weight, row, role.statement, &ctx)?);
                let profile = profiles
                    .entry(CapacityKey {
                        statement: role.statement,
                        key,
                    })
                    .or_default();
                match survivor.mode {
                    OpKind::Insert => {
                        profile.delta += weight;
                        profile.widen_down += weight;
                    }
                    OpKind::Delete => {
                        profile.delta -= weight;
                        profile.widen_up += weight;
                    }
                }
            }
        }

        for role in &info.parents {
            if selected(row, &role.selection) {
                let key = footprint_key(role.statement, &info.layout, &role.projection, row, &ctx)?;
                let profile = profiles
                    .entry(CapacityKey {
                        statement: role.statement,
                        key,
                    })
                    .or_default();
                match survivor.mode {
                    OpKind::Insert => profile.parent_add = true,
                    OpKind::Delete => profile.parent_remove = true,
                }
            }
        }
    }

    Ok(profiles)
}
