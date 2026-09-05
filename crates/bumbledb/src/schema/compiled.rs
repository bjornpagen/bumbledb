//! Compiled theory: one sealed schema's physical access paths (chapter 10 §2,
//! chapter 61). Storage, admission, keyed lookup and planning consume this
//! table — they do not each reinterpret the schema.
//!
//! A [`ProjectionId`] is deterministic under the sealed schema and physical
//! format; incidental hash-map iteration does not assign persistent ids.
//! Identical physical projections share one index even when statement-side
//! selections differ.

use std::collections::BTreeMap;
use std::sync::Arc;

use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

use super::judge::DeltaShape;
use super::{
    CapacityEnforcement, Enforcement, FieldDescriptor, KeyForm, KeyId, KeyStatement, Schema,
    StatementView,
};
use crate::Value;
use crate::schema::StatementKind;
use crate::schema::{FixedIntervalElement, IntervalElement, ValueType};

/// Stable physical projection identity (chapter 40). Assigned in canonical
/// statement order at compile time; survives incidental reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectionId(pub u16);

/// Compile failed because the sealed schema needs more interned projections
/// than [`ProjectionId`] can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileError {
    ProjectionIdExhausted,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectionIdExhausted => {
                f.write_str("compiled projection id space exhausted")
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile-time physical key encoding (chapter 40 §exact keys versus
/// fingerprints). Selected once per compiled access path, never per row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyEncoding {
    /// Order-preserving scalar grouping bytes (≤16 encoded scalar bytes,
    /// complete LMDB physical key fits the backend bound).
    ExactBounded { scalar_width: u8 },
    /// Fixed 16-byte BLAKE3 fingerprint; canonical projection confirms equality.
    FingerprintBucket,
}

impl KeyEncoding {
    /// Routing bytes width in the physical determinant index key (after the
    /// namespace tag and projection id, before the optional interval tail
    /// and row surrogate).
    #[must_use]
    pub const fn routing_width(self) -> usize {
        match self {
            Self::ExactBounded { scalar_width } => scalar_width as usize,
            Self::FingerprintBucket => crate::storage::store::fingerprint::FP_LEN,
        }
    }
}

/// Intern identity for one physical projection (C1). Distinct from a
/// statement id: storage, judge and planner share this key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionInternKey {
    pub relation: RelationId,
    pub projection: Box<[FieldId]>,
    pub encoding: KeyEncoding,
}

/// Checked optimization witness consumed by planner/fallback (C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistinctnessWitness {
    /// Full-row exact equality is the identity; collisions remain visible.
    FullRowEquality,
    /// Scalar projection is unique under the sealed key law.
    ScalarKeyUnique { projection: ProjectionId },
    /// Existence-only suffix may stop after the first sufficient witness.
    ExistenceOnly { projection: ProjectionId },
}

/// Cross-relation positional map between a statement-side projection and
/// interned physical field order (C1). Present even when the side has no
/// physical index: closed data keeps the coordinate, not a dummy index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionBinding {
    /// Interned physical index, if this side has one.
    pub projection: Option<ProjectionId>,
    /// Statement-side projection (interval included). Logical group order.
    pub logical: Box<[FieldId]>,
    /// Intern/key field order (interval included). Identity when unindexed.
    pub intern_order: Box<[FieldId]>,
    /// Statement-side scalar fields, logical group coordinates.
    pub logical_scalars: Box<[FieldId]>,
    /// Intern-order scalar fields, used only at index boundaries.
    pub intern_scalars: Box<[FieldId]>,
    /// Statement-side position → intern position (full projection).
    pub forward: Box<[u16]>,
    /// Intern position → statement-side position (full projection).
    pub inverse: Box<[u16]>,
}

impl ProjectionBinding {
    /// Logical group scalars from a full row (statement order, no interval).
    /// Independent of whether this side has a physical index.
    #[must_use]
    pub fn logical_group(&self, row: &[Value]) -> Vec<Value> {
        self.logical_scalars
            .iter()
            .map(|field| row[usize::from(field.0)].clone())
            .collect()
    }

    /// Intern-order scalars from a full row. For index probes only.
    #[must_use]
    pub fn intern_group(&self, row: &[Value]) -> Vec<Value> {
        self.intern_scalars
            .iter()
            .map(|field| row[usize::from(field.0)].clone())
            .collect()
    }

    /// Translate logical group scalars into interned index order.
    /// Identity when this side has no physical index.
    #[must_use]
    pub fn to_index(&self, logical: &[Value]) -> Option<Vec<Value>> {
        permute_by_fields(logical, &self.logical_scalars, &self.intern_scalars)
    }

    /// Translate interned index scalars into logical group order.
    #[must_use]
    pub fn from_index(&self, intern: &[Value]) -> Option<Vec<Value>> {
        permute_by_fields(intern, &self.intern_scalars, &self.logical_scalars)
    }

    /// Reorder statement-side projected values into intern order.
    #[must_use]
    pub fn physical_values(&self, caller: &[Value]) -> Option<Vec<Value>> {
        if caller.len() != self.forward.len() {
            return None;
        }
        let mut out = vec![Value::Bool(false); self.inverse.len()];
        for (caller_i, &physical_i) in self.forward.iter().enumerate() {
            out[usize::from(physical_i)] = caller[caller_i].clone();
        }
        Some(out)
    }

    /// Reorder intern values into statement-side order.
    #[must_use]
    pub fn caller_values(&self, physical: &[Value]) -> Option<Vec<Value>> {
        if physical.len() != self.inverse.len() {
            return None;
        }
        let mut out = vec![Value::Bool(false); self.forward.len()];
        for (physical_i, &caller_i) in self.inverse.iter().enumerate() {
            out[usize::from(caller_i)] = physical[physical_i].clone();
        }
        Some(out)
    }
}

fn permute_by_fields(
    values: &[Value],
    from: &[FieldId],
    to: &[FieldId],
) -> Option<Vec<Value>> {
    if values.len() != from.len() {
        return None;
    }
    let mut out = Vec::with_capacity(to.len());
    for field in to {
        let at = from.iter().position(|candidate| candidate == field)?;
        out.push(values[at].clone());
    }
    Some(out)
}

/// Visitor decision for descriptor-based candidate walks (D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitControl {
    /// Candidate was not a sufficient witness; keep scanning.
    Continue,
    /// Exact sufficient witness. Existence-only accesses may stop.
    Sufficient,
    /// Hard stop: sink refusal or caller-terminated walk.
    Stop,
}

/// Outcome of [`CompiledTheory::consume_visits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitOutcome {
    Exhausted { visited: usize },
    Sufficient { visited: usize },
    Stopped { visited: usize },
}

/// One compiled access path: relation, scalar positions, encoding and
/// optional ordered interval tail (chapter 10 §2).
#[derive(Debug, Clone)]
pub struct CompiledProjection {
    pub id: ProjectionId,
    pub relation: RelationId,
    /// Complete sealed projection in physical intern order (interval tail
    /// included). Shared indexes use this order, not a statement spelling.
    pub projection: Box<[FieldId]>,
    /// Positions into `projection` for scalar determinant fields only.
    pub scalar_positions: Box<[usize]>,
    pub scalar_fields: Box<[FieldDescriptor]>,
    pub encoding: KeyEncoding,
    /// Optional interval tail position within `projection`.
    pub interval_position: Option<usize>,
    pub interval_type: Option<ValueType>,
    /// Encoded interval-tail width in the complete physical key (0 if none).
    /// Separate from the 16-byte scalar grouping crossover.
    pub interval_tail_width: u8,
}

impl CompiledProjection {
    /// Scalar determinant values in physical intern order.
    /// Index-boundary coordinate only; grouping uses
    /// [`CompiledTheory::group_key`].
    pub fn scalar_values(&self, row: &[Value]) -> Vec<Value> {
        self.scalar_positions
            .iter()
            .map(|&position| row[usize::from(self.projection[position].0)].clone())
            .collect()
    }

    #[must_use]
    pub fn interval_field(&self) -> Option<FieldId> {
        self.interval_position.map(|position| self.projection[position])
    }

    /// Complete physical determinant key width: tag + projection id +
    /// routing + interval tail + row surrogate.
    #[must_use]
    pub fn complete_key_width(&self) -> usize {
        DETERMINANT_KEY_OVERHEAD
            + self.encoding.routing_width()
            + usize::from(self.interval_tail_width)
    }

    /// Order-preserving interval-tail bytes, or `None` when this access
    /// has no ordered tail or the row is missing that field.
    #[must_use]
    pub fn interval_tail_bytes(&self, row: &[Value]) -> Option<Vec<u8>> {
        let position = self.interval_position?;
        let field = self.projection[position];
        let value = row.get(usize::from(field.0))?;
        encode_interval_tail(value, self.interval_type.as_ref()?)
    }
}

#[derive(Debug)]
struct StatementAccess {
    key: Option<ProjectionId>,
    source: Option<ProjectionBinding>,
    target: Option<ProjectionBinding>,
    key_witness: Option<DistinctnessWitness>,
    source_witness: Option<DistinctnessWitness>,
    target_witness: Option<DistinctnessWitness>,
}

/// Law adjacency compiled once from the sealed schema (chapter 10 §3).
#[derive(Debug, Clone, Default)]
pub struct LawAdjacency {
    /// Per relation: key statement ids declared over it.
    pub keys: BTreeMap<RelationId, Vec<StatementId>>,
    /// Per relation: outgoing containment statements whose source is the relation.
    pub outgoing_containment: BTreeMap<RelationId, Vec<StatementId>>,
    /// Per relation: capacity statements whose source is the relation.
    pub outgoing_capacity: BTreeMap<RelationId, Vec<StatementId>>,
    /// Per relation: containment statements whose target is the relation.
    pub incoming_containment: BTreeMap<RelationId, Vec<StatementId>>,
    /// Per relation: capacity statements whose target is the relation.
    pub incoming_capacity: BTreeMap<RelationId, Vec<StatementId>>,
}

impl LawAdjacency {
    /// Net delta shape over one relation, or default when untouched.
    #[must_use]
    pub fn shape_of(delta: &[(RelationId, DeltaShape)], relation: RelationId) -> DeltaShape {
        delta
            .binary_search_by_key(&relation, |&(id, _)| id)
            .map_or_else(|_| DeltaShape::default(), |at| delta[at].1)
    }

    /// Whether delta-local judgment may skip this statement under the
    /// lawful-parent premise (chapter 10 §4).
    #[must_use]
    pub fn delta_local_skippable(
        &self,
        view: StatementView<'_>,
        delta: &[(RelationId, DeltaShape)],
    ) -> bool {
        match view {
            StatementView::Key(_, statement) => !Self::shape_of(delta, statement.relation).adds,
            StatementView::Containment(_, statement) => {
                let source = Self::shape_of(delta, statement.source.relation);
                let target = Self::shape_of(delta, statement.target.relation);
                !source.adds && !target.removes
            }
            StatementView::Capacity(_, statement) => {
                let source = Self::shape_of(delta, statement.source.relation);
                let target = Self::shape_of(delta, statement.target.relation);
                !source.touched() && !target.adds
            }
        }
    }
}

/// The sealed schema's compiled machine (chapter 61). Owned by the store
/// and query layers; compiled once at open / seal.
#[derive(Debug)]
pub struct CompiledTheory {
    /// Indexed by `ProjectionId.0`.
    projections: Box<[CompiledProjection]>,
    by_statement: BTreeMap<StatementId, StatementAccess>,
    by_relation: BTreeMap<RelationId, Vec<ProjectionId>>,
    key_by_relation: BTreeMap<RelationId, Vec<ProjectionId>>,
    witnesses: Box<[DistinctnessWitness]>,
    /// Full-row field descriptors per relation id.
    fields: Box<[Box<[FieldDescriptor]>]>,
    pub adjacency: LawAdjacency,
    /// Maximum complete physical determinant key width (prefix + routing +
    /// interval tail + row surrogate), for schema validation.
    pub max_determinant_key_width: usize,
}

/// LMDB's key limit for the pinned build (chapter 40). One place — not
/// scattered 400/496/511 axioms.
pub const LMDB_KEY_LIMIT: usize = 511;

/// Prefix + projection id + row surrogate before routing / interval tail.
const DETERMINANT_KEY_OVERHEAD: usize = 1 + 2 + 8;

/// Maximum encoded scalar bytes for the exact-bounded crossover (chapter 40).
pub const MAX_EXACT_SCALAR_BYTES: usize = 16;

impl CompiledTheory {
    /// Compile the sealed schema into one reusable projection/law table.
    ///
    /// # Errors
    /// [`CompileError::ProjectionIdExhausted`] when more than `u16::MAX + 1`
    /// distinct physical projections would be interned.
    pub fn compile(schema: &Schema) -> Result<Self, CompileError> {
        let relation_count = schema.relations().len();
        let mut fields: Vec<Box<[FieldDescriptor]>> = Vec::with_capacity(relation_count);
        for relation in schema.relations() {
            fields.push(relation.fields().to_vec().into_boxed_slice());
        }

        let mut intern = Interning {
            fields: &fields,
            intern: BTreeMap::new(),
            projections: Vec::new(),
            by_relation: BTreeMap::new(),
            key_by_relation: BTreeMap::new(),
            witnesses: Vec::new(),
            max_key: 0,
        };
        let mut by_statement = BTreeMap::new();
        let mut adjacency = LawAdjacency::default();

        for view in schema.statements() {
            match view {
                StatementView::Key(_, statement) => {
                    compile_key(schema, statement, &mut intern, &mut by_statement, &mut adjacency)?;
                }
                StatementView::Containment(_, statement) => {
                    compile_containment(
                        schema,
                        statement,
                        &mut intern,
                        &mut by_statement,
                        &mut adjacency,
                    )?;
                }
                StatementView::Capacity(_, statement) => {
                    compile_capacity(
                        schema,
                        statement,
                        &mut intern,
                        &mut by_statement,
                        &mut adjacency,
                    )?;
                }
            }
        }

        Ok(Self {
            projections: intern.projections.into_boxed_slice(),
            by_statement,
            by_relation: intern.by_relation,
            key_by_relation: intern.key_by_relation,
            witnesses: intern.witnesses.into_boxed_slice(),
            fields: fields.into_boxed_slice(),
            adjacency,
            max_determinant_key_width: intern.max_key,
        })
    }

    #[must_use]
    pub fn projection(&self, id: ProjectionId) -> Option<&CompiledProjection> {
        self.projections.get(id.0 as usize)
    }

    /// Key-statement projection. Containment/capacity use
    /// [`Self::source_projection`] / [`Self::target_projection`].
    #[must_use]
    pub fn projection_of_statement(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.key)
            .and_then(|id| self.projection(id))
    }

    #[must_use]
    pub fn projections_of_relation(&self, relation: RelationId) -> &[ProjectionId] {
        self.by_relation
            .get(&relation)
            .map_or(&[], Vec::as_slice)
    }

    /// Interned key-law projections of one relation (not containment/capacity
    /// group indexes unless they share that physical key).
    #[must_use]
    pub fn key_projections_of(&self, relation: RelationId) -> &[ProjectionId] {
        self.key_by_relation
            .get(&relation)
            .map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn key_for(
        &self,
        relation: RelationId,
        projection: &[FieldId],
    ) -> Option<&CompiledProjection> {
        self.key_by_relation.get(&relation).and_then(|ids| {
            ids.iter().find_map(|id| {
                let compiled = self.projection(*id)?;
                (compiled.projection.as_ref() == projection).then_some(compiled)
            })
        })
    }

    #[must_use]
    pub fn fields_of(&self, relation: RelationId) -> Option<&[FieldDescriptor]> {
        self.fields.get(relation.0 as usize).map(AsRef::as_ref)
    }

    #[must_use]
    pub fn projections(&self) -> &[CompiledProjection] {
        &self.projections
    }

    /// Intern key for a compiled projection (C1 descriptor identity).
    #[must_use]
    pub fn intern_key(projection: &CompiledProjection) -> ProjectionInternKey {
        ProjectionInternKey {
            relation: projection.relation,
            projection: projection.projection.clone(),
            encoding: projection.encoding,
        }
    }

    /// Containment/capacity source reverse or group projection (C1).
    /// `None` when the side has no physical index (closed data).
    #[must_use]
    pub fn source_projection(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.source_binding(statement)
            .and_then(|binding| binding.projection)
            .and_then(|id| self.projection(id))
    }

    /// Containment/capacity target lookup projection (C1).
    /// `None` when the side has no physical index (closed data).
    #[must_use]
    pub fn target_projection(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.target_binding(statement)
            .and_then(|binding| binding.projection)
            .and_then(|id| self.projection(id))
    }

    /// Logical group scalars for one bound side. One coordinate system
    /// whether or not that side has a physical index. L02 replaces local
    /// `group_key` / closed-vs-indexed order splits with this.
    #[must_use]
    pub fn group_key(binding: &ProjectionBinding, row: &[Value]) -> Vec<Value> {
        binding.logical_group(row)
    }

    /// Translate a logical group into interned index order. Identity when
    /// the side has no physical index. Call only at indexed-access
    /// boundaries (`visit_compiled_group` / determinant seeks).
    #[must_use]
    pub fn index_key(binding: &ProjectionBinding, logical: &[Value]) -> Option<Vec<Value>> {
        binding.to_index(logical)
    }

    #[must_use]
    pub fn source_binding(&self, statement: StatementId) -> Option<&ProjectionBinding> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.source.as_ref())
    }

    #[must_use]
    pub fn target_binding(&self, statement: StatementId) -> Option<&ProjectionBinding> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.target.as_ref())
    }

    /// Strongest lawful witness attached to the interned projection itself.
    #[must_use]
    pub fn distinctness_witness(&self, id: ProjectionId) -> Option<DistinctnessWitness> {
        self.witnesses.get(id.0 as usize).copied()
    }

    #[must_use]
    pub fn source_witness(&self, statement: StatementId) -> Option<DistinctnessWitness> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.source_witness)
    }

    #[must_use]
    pub fn target_witness(&self, statement: StatementId) -> Option<DistinctnessWitness> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.target_witness)
    }

    #[must_use]
    pub fn key_witness(&self, statement: StatementId) -> Option<DistinctnessWitness> {
        self.by_statement
            .get(&statement)
            .and_then(|access| access.key_witness)
    }

    /// Full-row collision check is never sacrificed for a compact key (C1).
    #[must_use]
    pub const fn full_row_witness() -> DistinctnessWitness {
        DistinctnessWitness::FullRowEquality
    }

    /// Descriptor-based candidate walk. Existence-only stops after the first
    /// sufficient exact witness. `Stop` and source `Err` prevent later visits.
    pub fn consume_visits<T, E>(
        witness: DistinctnessWitness,
        candidates: impl IntoIterator<Item = T>,
        visit: &mut dyn FnMut(T) -> Result<VisitControl, E>,
    ) -> Result<VisitOutcome, E> {
        let existence_only = matches!(witness, DistinctnessWitness::ExistenceOnly { .. });
        let mut visited = 0usize;
        for item in candidates {
            visited = visited.saturating_add(1);
            match visit(item)? {
                VisitControl::Continue => {}
                VisitControl::Sufficient if existence_only => {
                    return Ok(VisitOutcome::Sufficient { visited });
                }
                VisitControl::Sufficient => {}
                VisitControl::Stop => return Ok(VisitOutcome::Stopped { visited }),
            }
        }
        Ok(VisitOutcome::Exhausted { visited })
    }

    /// Statement ids whose law family the delta can affect (chapter 10 §4).
    #[must_use]
    pub fn delta_local_statements<'a>(
        &self,
        schema: &'a Schema,
        delta: &[(RelationId, DeltaShape)],
    ) -> Vec<StatementView<'a>> {
        schema
            .statements()
            .filter(|view| {
                !schema.closed_constant(*view) && !self.adjacency.delta_local_skippable(*view, delta)
            })
            .collect()
    }

    /// Kind of one statement for downstream witnesses (planner distinctness).
    #[must_use]
    pub fn statement_kind(view: StatementView<'_>) -> StatementKind {
        match view {
            StatementView::Key(..) => StatementKind::Functionality,
            StatementView::Containment(..) => StatementKind::Containment,
            StatementView::Capacity(..) => StatementKind::Capacity,
        }
    }
}

/// Shared compiled theory handle — one compile per sealed schema.
pub(crate) type SharedCompiledTheory = Arc<CompiledTheory>;

/// # Errors
/// [`CompileError::ProjectionIdExhausted`] when interned projection ids run out.
pub(crate) fn shared_compile(schema: &Schema) -> Result<SharedCompiledTheory, CompileError> {
    Ok(Arc::new(CompiledTheory::compile(schema)?))
}

struct Interning<'a> {
    fields: &'a [Box<[FieldDescriptor]>],
    intern: BTreeMap<ProjectionInternKey, ProjectionId>,
    projections: Vec<CompiledProjection>,
    by_relation: BTreeMap<RelationId, Vec<ProjectionId>>,
    key_by_relation: BTreeMap<RelationId, Vec<ProjectionId>>,
    witnesses: Vec<DistinctnessWitness>,
    max_key: usize,
}

impl Interning<'_> {
    fn intern(
        &mut self,
        relation: RelationId,
        projection: &[FieldId],
        witness: DistinctnessWitness,
        as_key: bool,
    ) -> Result<ProjectionId, CompileError> {
        let descriptors = &self.fields[relation.0 as usize];
        let compiled = compile_projection(ProjectionId(0), relation, projection, descriptors);
        let key = ProjectionInternKey {
            relation: compiled.relation,
            projection: compiled.projection.clone(),
            encoding: compiled.encoding,
        };
        if let Some(&existing) = self.intern.get(&key) {
            self.merge_witness(existing, witness);
            if as_key {
                remember_key(&mut self.key_by_relation, relation, existing);
            }
            return Ok(existing);
        }
        let id = assign_projection_id(self.projections.len())?;
        let mut compiled = compiled;
        compiled.id = id;
        self.max_key = self.max_key.max(compiled.complete_key_width());
        self.by_relation.entry(relation).or_default().push(id);
        if as_key {
            remember_key(&mut self.key_by_relation, relation, id);
        }
        self.intern.insert(key, id);
        self.projections.push(compiled);
        self.witnesses.push(placed_witness(id, witness));
        Ok(id)
    }

    fn merge_witness(&mut self, id: ProjectionId, incoming: DistinctnessWitness) {
        let slot = &mut self.witnesses[id.0 as usize];
        *slot = stronger_witness(*slot, placed_witness(id, incoming));
    }
}

fn compile_key(
    schema: &Schema,
    statement: &KeyStatement,
    intern: &mut Interning<'_>,
    by_statement: &mut BTreeMap<StatementId, StatementAccess>,
    adjacency: &mut LawAdjacency,
) -> Result<(), CompileError> {
    adjacency
        .keys
        .entry(statement.relation)
        .or_default()
        .push(statement.id);
    if schema.relation(statement.relation).body().closed_rows().is_some() {
        return Ok(());
    }
    let witness = match statement.form() {
        KeyForm::Scalar => DistinctnessWitness::ScalarKeyUnique {
            projection: ProjectionId(0),
        },
        KeyForm::Pointwise { .. } => DistinctnessWitness::FullRowEquality,
    };
    let id = intern.intern(statement.relation, &statement.projection, witness, true)?;
    let access = by_statement
        .entry(statement.id)
        .or_insert_with(empty_access);
    access.key = Some(id);
    access.key_witness = Some(placed_witness(id, witness));
    Ok(())
}

fn compile_containment(
    schema: &Schema,
    statement: &super::ContainmentStatement,
    intern: &mut Interning<'_>,
    by_statement: &mut BTreeMap<StatementId, StatementAccess>,
    adjacency: &mut LawAdjacency,
) -> Result<(), CompileError> {
    adjacency
        .outgoing_containment
        .entry(statement.source.relation)
        .or_default()
        .push(statement.id);
    adjacency
        .incoming_containment
        .entry(statement.target.relation)
        .or_default()
        .push(statement.id);

        let access = by_statement
            .entry(statement.id)
            .or_insert_with(empty_access);
        match &statement.enforcement {
            Enforcement::Closed { .. } => {
                bind_coordinates(
                    intern.fields,
                    access,
                    statement.source.relation,
                    &statement.source.projection,
                    statement.target.relation,
                    &statement.target.projection,
                );
                Ok(())
            }
            Enforcement::ScalarProbe {
                target_key,
                key_projection,
            }
            | Enforcement::IntervalCoverage {
                target_key,
                key_projection,
                ..
            } => {
                bind_sides(
                    schema,
                    intern,
                    access,
                    statement.source.relation,
                    &statement.source.projection,
                    statement.target.relation,
                    &statement.target.projection,
                    key_projection,
                    *target_key,
                    DistinctnessWitness::FullRowEquality,
                    DistinctnessWitness::ExistenceOnly {
                        projection: ProjectionId(0),
                    },
                )
            }
        }
}

fn compile_capacity(
    schema: &Schema,
    statement: &super::CapacityStatement,
    intern: &mut Interning<'_>,
    by_statement: &mut BTreeMap<StatementId, StatementAccess>,
    adjacency: &mut LawAdjacency,
) -> Result<(), CompileError> {
    adjacency
        .outgoing_capacity
        .entry(statement.source.relation)
        .or_default()
        .push(statement.id);
    adjacency
        .incoming_capacity
        .entry(statement.target.relation)
        .or_default()
        .push(statement.id);

    let access = by_statement
        .entry(statement.id)
        .or_insert_with(empty_access);
        match &statement.enforcement {
            CapacityEnforcement::Closed { .. } => {
                bind_coordinates(
                    intern.fields,
                    access,
                    statement.source.relation,
                    &statement.source.projection,
                    statement.target.relation,
                    &statement.target.projection,
                );
                Ok(())
            }
            CapacityEnforcement::ScalarProbe {
                target_key,
                key_projection,
            } => bind_sides(
            schema,
            intern,
            access,
            statement.source.relation,
            &statement.source.projection,
            statement.target.relation,
            &statement.target.projection,
            key_projection,
            *target_key,
            DistinctnessWitness::FullRowEquality,
            DistinctnessWitness::ExistenceOnly {
                projection: ProjectionId(0),
            },
        ),
    }
}

fn bind_coordinates(
    fields: &[Box<[FieldDescriptor]>],
    access: &mut StatementAccess,
    source_relation: RelationId,
    source_caller: &[FieldId],
    target_relation: RelationId,
    target_caller: &[FieldId],
) {
    access.source = Some(bind_projection(
        None,
        source_caller,
        source_caller,
        &fields[source_relation.0 as usize],
    ));
    access.target = Some(bind_projection(
        None,
        target_caller,
        target_caller,
        &fields[target_relation.0 as usize],
    ));
}

fn bind_sides(
    schema: &Schema,
    intern: &mut Interning<'_>,
    access: &mut StatementAccess,
    source_relation: RelationId,
    source_caller: &[FieldId],
    target_relation: RelationId,
    target_caller: &[FieldId],
    source_physical: &[FieldId],
    target_key: KeyId,
    source_witness: DistinctnessWitness,
    target_witness: DistinctnessWitness,
) -> Result<(), CompileError> {
    attach_side(
        intern,
        access,
        true,
        source_relation,
        source_caller,
        source_physical,
        schema
            .relation(source_relation)
            .body()
            .closed_rows()
            .is_some(),
        source_witness,
    )?;
    let target_physical = schema.key(target_key).projection.as_ref();
    attach_side(
        intern,
        access,
        false,
        target_relation,
        target_caller,
        target_physical,
        schema
            .relation(target_relation)
            .body()
            .closed_rows()
            .is_some(),
        target_witness,
    )
}

fn attach_side(
    intern: &mut Interning<'_>,
    access: &mut StatementAccess,
    is_source: bool,
    relation: RelationId,
    caller: &[FieldId],
    physical: &[FieldId],
    closed: bool,
    witness: DistinctnessWitness,
) -> Result<(), CompileError> {
    let id = if closed {
        None
    } else {
        Some(intern.intern(relation, physical, witness, false)?)
    };
    let intern_order = if closed { caller } else { physical };
    let binding = bind_projection(
        id,
        caller,
        intern_order,
        &intern.fields[relation.0 as usize],
    );
    let placed = id.map(|id| placed_witness(id, witness));
    if is_source {
        access.source = Some(binding);
        access.source_witness = placed;
    } else {
        access.target = Some(binding);
        access.target_witness = placed;
    }
    Ok(())
}

fn empty_access() -> StatementAccess {
    StatementAccess {
        key: None,
        source: None,
        target: None,
        key_witness: None,
        source_witness: None,
        target_witness: None,
    }
}

fn remember_key(
    key_by_relation: &mut BTreeMap<RelationId, Vec<ProjectionId>>,
    relation: RelationId,
    id: ProjectionId,
) {
    let keys = key_by_relation.entry(relation).or_default();
    if !keys.contains(&id) {
        keys.push(id);
    }
}

fn bind_projection(
    id: Option<ProjectionId>,
    caller: &[FieldId],
    physical: &[FieldId],
    descriptors: &[FieldDescriptor],
) -> ProjectionBinding {
    let mut forward = vec![0u16; caller.len()];
    let mut inverse = vec![0u16; physical.len()];
    for (caller_i, field) in caller.iter().enumerate() {
        let physical_i = physical
            .iter()
            .position(|candidate| candidate == field)
            .expect("validated sides are set-equal to the interned projection");
        forward[caller_i] = u16::try_from(physical_i).expect("projection width fits u16");
        inverse[physical_i] = u16::try_from(caller_i).expect("projection width fits u16");
    }
    ProjectionBinding {
        projection: id,
        logical: caller.to_vec().into_boxed_slice(),
        intern_order: physical.to_vec().into_boxed_slice(),
        logical_scalars: scalar_fields_of(caller, descriptors),
        intern_scalars: scalar_fields_of(physical, descriptors),
        forward: forward.into_boxed_slice(),
        inverse: inverse.into_boxed_slice(),
    }
}

fn scalar_fields_of(projection: &[FieldId], descriptors: &[FieldDescriptor]) -> Box<[FieldId]> {
    projection
        .iter()
        .copied()
        .filter(|field| !descriptors[usize::from(field.0)].value_type.is_interval())
        .collect()
}

fn assign_projection_id(count: usize) -> Result<ProjectionId, CompileError> {
    u16::try_from(count)
        .map(ProjectionId)
        .map_err(|_| CompileError::ProjectionIdExhausted)
}

fn placed_witness(id: ProjectionId, witness: DistinctnessWitness) -> DistinctnessWitness {
    match witness {
        DistinctnessWitness::ScalarKeyUnique { .. } => {
            DistinctnessWitness::ScalarKeyUnique { projection: id }
        }
        DistinctnessWitness::ExistenceOnly { .. } => {
            DistinctnessWitness::ExistenceOnly { projection: id }
        }
        DistinctnessWitness::FullRowEquality => DistinctnessWitness::FullRowEquality,
    }
}

fn stronger_witness(left: DistinctnessWitness, right: DistinctnessWitness) -> DistinctnessWitness {
    match (left, right) {
        (DistinctnessWitness::ScalarKeyUnique { projection }, _)
        | (_, DistinctnessWitness::ScalarKeyUnique { projection }) => {
            DistinctnessWitness::ScalarKeyUnique { projection }
        }
        (DistinctnessWitness::FullRowEquality, _) | (_, DistinctnessWitness::FullRowEquality) => {
            DistinctnessWitness::FullRowEquality
        }
        (DistinctnessWitness::ExistenceOnly { projection }, DistinctnessWitness::ExistenceOnly { .. }) => {
            DistinctnessWitness::ExistenceOnly { projection }
        }
    }
}

fn compile_projection(
    id: ProjectionId,
    relation: RelationId,
    projection: &[FieldId],
    descriptors: &[FieldDescriptor],
) -> CompiledProjection {
    let mut scalar_positions = Vec::new();
    let mut scalar_fields = Vec::new();
    let mut interval_position = None;
    let mut interval_type = None;
    let mut interval_tail_width = 0u8;
    for (position, field) in projection.iter().enumerate() {
        let descriptor = &descriptors[usize::from(field.0)];
        if descriptor.value_type.is_interval() {
            interval_position = Some(position);
            interval_type = Some(descriptor.value_type);
            interval_tail_width = u8::try_from(interval_tail_encoded_width(&descriptor.value_type))
                .unwrap_or(u8::MAX);
        } else {
            scalar_positions.push(position);
            scalar_fields.push(descriptor.clone());
        }
    }
    let encoding = select_encoding_with_tail(&scalar_fields, usize::from(interval_tail_width));
    CompiledProjection {
        id,
        relation,
        projection: projection.to_vec().into_boxed_slice(),
        scalar_positions: scalar_positions.into_boxed_slice(),
        scalar_fields: scalar_fields.into_boxed_slice(),
        encoding,
        interval_position,
        interval_type,
        interval_tail_width,
    }
}

fn interval_tail_encoded_width(value_type: &ValueType) -> usize {
    match value_type {
        ValueType::Interval { .. } => 16,
        ValueType::FixedInterval { .. } => 8,
        _ => 0,
    }
}

/// Routing-byte width for schema validation (exact scalar or fingerprint).
pub fn select_key_encoding_width(scalar_fields: &[FieldDescriptor]) -> usize {
    match select_encoding(scalar_fields) {
        KeyEncoding::ExactBounded { scalar_width } => scalar_width as usize,
        KeyEncoding::FingerprintBucket => crate::storage::store::fingerprint::FP_LEN,
    }
}

pub(crate) fn select_encoding(scalar_fields: &[FieldDescriptor]) -> KeyEncoding {
    select_encoding_with_tail(scalar_fields, 0)
}

fn select_encoding_with_tail(
    scalar_fields: &[FieldDescriptor],
    interval_tail_width: usize,
) -> KeyEncoding {
    let mut width = 0usize;
    for field in scalar_fields {
        let Some(exact) = exact_scalar_width(&field.value_type) else {
            return KeyEncoding::FingerprintBucket;
        };
        width = width.saturating_add(exact);
        if width > MAX_EXACT_SCALAR_BYTES {
            return KeyEncoding::FingerprintBucket;
        }
    }
    let complete = DETERMINANT_KEY_OVERHEAD
        .saturating_add(width)
        .saturating_add(interval_tail_width);
    if complete > LMDB_KEY_LIMIT {
        return KeyEncoding::FingerprintBucket;
    }
    KeyEncoding::ExactBounded {
        scalar_width: u8::try_from(width).unwrap_or(255),
    }
}

/// Exact order-preserving width for one scalar type, or `None` for the
/// fingerprint arm (text, variable width).
fn exact_scalar_width(value_type: &ValueType) -> Option<usize> {
    match value_type {
        ValueType::Bool => Some(1),
        ValueType::U64 | ValueType::I64 | ValueType::F64 => Some(8),
        ValueType::Id128 => Some(16),
        ValueType::FixedBytes { len } => Some(usize::from(*len)),
        ValueType::FixedInterval { .. } | ValueType::String | ValueType::Interval { .. } => None,
    }
}

/// Encode scalar determinant values as compact order-preserving bytes
/// (chapter 40). Used for exact-bounded index routing and exact confirmation.
/// Schema types are already known: no per-field tags are written.
pub fn encode_scalar_group(values: &[Value], fields: &[FieldDescriptor]) -> Option<Vec<u8>> {
    if values.len() != fields.len() {
        return None;
    }
    let mut out = Vec::new();
    for (value, field) in values.iter().zip(fields) {
        append_exact_scalar(value, &field.value_type, &mut out)?;
    }
    Some(out)
}

fn append_exact_scalar(
    value: &Value,
    value_type: &ValueType,
    out: &mut Vec<u8>,
) -> Option<()> {
    use crate::encoding::{encode_bool, encode_f64, encode_i64, encode_u64};
    match (value, value_type) {
        (Value::Bool(v), ValueType::Bool) => out.push(encode_bool(*v)),
        (Value::U64(v), ValueType::U64) => out.extend_from_slice(&encode_u64(*v)),
        (Value::I64(v), ValueType::I64) => out.extend_from_slice(&encode_i64(*v)),
        (Value::F64(v), ValueType::F64) => out.extend_from_slice(&encode_f64(*v)),
        (Value::Id128(v), ValueType::Id128) => out.extend_from_slice(v.as_bytes()),
        (Value::FixedBytes(bytes), ValueType::FixedBytes { len }) => {
            if bytes.len() != usize::from(*len) {
                return None;
            }
            out.extend_from_slice(bytes);
        }
        _ => return None,
    }
    Some(())
}

fn encode_interval_tail(value: &Value, value_type: &ValueType) -> Option<Vec<u8>> {
    use crate::encoding::{
        encode_i64, encode_interval_f64, encode_interval_i64, encode_interval_u64, encode_u64,
    };
    match (value, value_type) {
        (
            Value::IntervalU64(interval),
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        ) => Some(encode_interval_u64(*interval).to_vec()),
        (
            Value::IntervalI64(interval),
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ) => Some(encode_interval_i64(*interval).to_vec()),
        (
            Value::IntervalF64(interval),
            ValueType::Interval {
                element: IntervalElement::F64,
            },
        ) => Some(encode_interval_f64(*interval).to_vec()),
        (
            Value::IntervalU64(interval),
            ValueType::FixedInterval {
                element: FixedIntervalElement::U64,
                ..
            },
        ) => Some(encode_u64(interval.start()).to_vec()),
        (
            Value::IntervalI64(interval),
            ValueType::FixedInterval {
                element: FixedIntervalElement::I64,
                ..
            },
        ) => Some(encode_i64(interval.start()).to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::encode_u64;
    use crate::schema::tests::{capacity, closed, containment, fd, field, row, side, side_where};
    use crate::schema::{RelationDescriptor, SchemaDescriptor, ValidateDescriptor as _};

    fn compile(schema: &Schema) -> CompiledTheory {
        CompiledTheory::compile(schema).expect("projection ids")
    }

    #[test]
    fn u64_key_compiles_exact_bounded() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("id", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let proj = theory.projection(ProjectionId(0)).expect("one key");
        assert!(matches!(
            proj.encoding,
            KeyEncoding::ExactBounded { scalar_width: 8 }
        ));
        let routing = encode_scalar_group(&[Value::U64(42)], &proj.scalar_fields).expect("exact");
        assert_eq!(routing, encode_u64(42), "compact u64 key bytes, not width arithmetic");
        assert!(
            !routing.contains(&3),
            "exact keys do not serialize schema tags"
        );
    }

    #[test]
    fn text_key_compiles_fingerprint_bucket() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("text", ValueType::String)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let proj = theory.projection(ProjectionId(0)).expect("one key");
        assert_eq!(proj.encoding, KeyEncoding::FingerprintBucket);
        assert_eq!(
            theory.distinctness_witness(proj.id),
            Some(DistinctnessWitness::ScalarKeyUnique { projection: proj.id })
        );
    }

    #[test]
    fn d04_key_and_containment_target_share_one_physical_index() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![field("id", ValueType::U64), field("parent", ValueType::U64)],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                fd(RelationId(1), &[FieldId(0)]),
                containment(
                    side(RelationId(1), &[FieldId(1)]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let key = theory
            .projection_of_statement(StatementId(0))
            .expect("T key");
        let target = theory
            .target_projection(StatementId(2))
            .expect("containment target");
        assert_eq!(key.id, target.id, "shared interned projection identity");
        assert_eq!(
            CompiledTheory::intern_key(key),
            CompiledTheory::intern_key(target)
        );
        let source = theory
            .source_projection(StatementId(2))
            .expect("containment reverse");
        assert_eq!(source.relation, RelationId(1));
        assert_eq!(&*source.projection, &[FieldId(1)]);
        assert_ne!(source.id, key.id, "source reverse is a distinct physical index");
        assert_eq!(
            theory.distinctness_witness(target.id),
            Some(DistinctnessWitness::ScalarKeyUnique {
                projection: target.id
            })
        );
        assert_eq!(
            theory.target_witness(StatementId(2)),
            Some(DistinctnessWitness::ExistenceOnly {
                projection: target.id
            })
        );
        assert_eq!(
            theory.source_witness(StatementId(2)),
            Some(DistinctnessWitness::FullRowEquality)
        );
    }

    #[test]
    fn d04_reordered_cross_relation_columns_preserve_inverse() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("a", ValueType::U64), field("b", ValueType::I64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![field("x", ValueType::I64), field("y", ValueType::U64)],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0), FieldId(1)]),
                fd(RelationId(1), &[FieldId(0), FieldId(1)]),
                containment(
                    side(RelationId(1), &[FieldId(0), FieldId(1)]),
                    side(RelationId(0), &[FieldId(1), FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let source = theory.source_binding(StatementId(2)).expect("source map");
        let target = theory.target_binding(StatementId(2)).expect("target map");
        assert_eq!(&*source.forward, &[1, 0]);
        assert_eq!(&*source.inverse, &[1, 0]);
        assert_eq!(&*target.forward, &[1, 0]);
        assert_eq!(&*target.inverse, &[1, 0]);
        let physical = source
            .physical_values(&[Value::I64(7), Value::U64(3)])
            .expect("permute");
        assert_eq!(physical, vec![Value::U64(3), Value::I64(7)]);
        let target_proj = theory
            .target_projection(StatementId(2))
            .expect("target intern");
        assert_eq!(&*target_proj.projection, &[FieldId(0), FieldId(1)]);
        assert_eq!(
            theory
                .projection_of_statement(StatementId(0))
                .expect("T key")
                .id,
            target_proj.id
        );
    }

    #[test]
    fn d04_selected_predicates_share_one_unfiltered_index() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![
                        field("id", ValueType::U64),
                        field("parent", ValueType::U64),
                        field("kind", ValueType::U64),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                fd(RelationId(1), &[FieldId(0)]),
                containment(
                    side_where(RelationId(1), &[FieldId(1)], vec![(FieldId(2), Value::U64(1))]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
                containment(
                    side_where(RelationId(1), &[FieldId(1)], vec![(FieldId(2), Value::U64(2))]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let a = theory.source_projection(StatementId(2)).expect("sel 1");
        let b = theory.source_projection(StatementId(3)).expect("sel 2");
        assert_eq!(a.id, b.id, "predicates do not mint duplicate physical ids");
        assert_eq!(&*a.projection, &[FieldId(1)]);
    }

    #[test]
    fn d04_capacity_source_and_target_are_real_descriptors() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("id", ValueType::U64), field("cap", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![field("id", ValueType::U64), field("parent", ValueType::U64)],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                fd(RelationId(1), &[FieldId(0)]),
                capacity(
                    side(RelationId(1), &[FieldId(1)]),
                    0,
                    Some(3),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let source = theory
            .source_projection(StatementId(2))
            .expect("capacity source group");
        let target = theory
            .target_projection(StatementId(2))
            .expect("capacity target");
        assert_eq!(&*source.projection, &[FieldId(1)]);
        assert_eq!(source.relation, RelationId(1));
        assert_eq!(
            target.id,
            theory
                .projection_of_statement(StatementId(0))
                .expect("T key")
                .id
        );
        assert!(
            theory.projection_of_statement(StatementId(2)).is_none(),
            "capacity is not a key alias"
        );
    }

    #[test]
    fn d04_pointwise_interval_tail_is_not_scalar_uniqueness() {
        let iv = ValueType::Interval {
            element: IntervalElement::I64,
        };
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Booking".into(),
                    fields: vec![field("room", ValueType::U64), field("during", iv)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Request".into(),
                    fields: vec![field("room", ValueType::U64), field("span", iv)],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0), FieldId(1)]),
                containment(
                    side(RelationId(1), &[FieldId(0), FieldId(1)]),
                    side(RelationId(0), &[FieldId(0), FieldId(1)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let key = theory
            .projection_of_statement(StatementId(0))
            .expect("pointwise key");
        assert!(matches!(
            key.encoding,
            KeyEncoding::ExactBounded { scalar_width: 8 }
        ));
        assert_eq!(key.interval_tail_width, 16);
        assert_eq!(
            key.complete_key_width(),
            DETERMINANT_KEY_OVERHEAD + 8 + 16
        );
        assert_eq!(
            theory.distinctness_witness(key.id),
            Some(DistinctnessWitness::FullRowEquality),
            "pointwise match is not scalar uniqueness"
        );
        let source = theory
            .source_projection(StatementId(1))
            .expect("coverage source");
        assert_eq!(source.interval_tail_width, 16);
        assert_eq!(source.interval_position, Some(1));
    }

    #[test]
    fn d04_logical_group_survives_closed_source_permutation() {
        let schema = SchemaDescriptor {
            relations: vec![
                closed(
                    "Src",
                    vec![field("a", ValueType::U64), field("b", ValueType::I64)],
                    vec![row("s", vec![Value::U64(1), Value::I64(2)])],
                ),
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("a", ValueType::I64), field("b", ValueType::U64)],
                },
            ],
            statements: vec![
                fd(RelationId(1), &[FieldId(0), FieldId(1)]),
                containment(
                    side(RelationId(0), &[FieldId(1), FieldId(2)]),
                    side(RelationId(1), &[FieldId(1), FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("closed Source[a,b] ⊆ Target[b,a]");
        let theory = compile(&schema);
        assert!(
            theory.source_projection(StatementId(1)).is_none(),
            "closed source must not mint a physical index"
        );
        let source = theory.source_binding(StatementId(1)).expect("closed coordinate");
        let target = theory.target_binding(StatementId(1)).expect("target coordinate");
        assert!(source.projection.is_none());
        assert!(target.projection.is_some());
        let source_row = [Value::U64(0), Value::U64(1), Value::I64(2)];
        let target_row = [Value::I64(2), Value::U64(1)];
        let source_group = CompiledTheory::group_key(source, &source_row);
        let target_group = CompiledTheory::group_key(target, &target_row);
        assert_eq!(
            source_group,
            vec![Value::U64(1), Value::I64(2)],
            "logical group is statement order"
        );
        assert_eq!(
            source_group, target_group,
            "closed and indexed sides share one logical coordinate"
        );
        let probe = CompiledTheory::index_key(target, &target_group).expect("index probe");
        let compiled = theory
            .target_projection(StatementId(1))
            .expect("ordinary target index");
        assert_eq!(
            probe,
            vec![Value::I64(2), Value::U64(1)],
            "index translation is key order (a,b)"
        );
        assert_eq!(probe, compiled.scalar_values(&target_row));
        assert_eq!(probe, target.intern_group(&target_row));
        assert_ne!(
            compiled.scalar_values(&target_row),
            source_group,
            "physical index order is not the logical group"
        );
        assert_eq!(
            CompiledTheory::index_key(source, &source_group).expect("closed identity"),
            source_group,
            "unindexed side does not permute at a fake index boundary"
        );
    }

    #[test]
    fn d04_closed_target_keeps_identity_coordinates() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Child".into(),
                    fields: vec![field("parent", ValueType::U64)],
                },
                closed(
                    "Parent",
                    vec![],
                    vec![row("p", vec![])],
                ),
            ],
            statements: vec![containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            )],
        }
        .validate()
        .expect("ordinary ⊆ closed handle");
        let theory = compile(&schema);
        assert!(theory.source_projection(StatementId(0)).is_none());
        assert!(theory.target_projection(StatementId(0)).is_none());
        let source = theory.source_binding(StatementId(0)).expect("source coord");
        let target = theory.target_binding(StatementId(0)).expect("target coord");
        assert!(source.projection.is_none());
        assert!(target.projection.is_none());
        let child = [Value::U64(0)];
        let parent = [Value::U64(0)];
        assert_eq!(
            CompiledTheory::group_key(source, &child),
            CompiledTheory::group_key(target, &parent)
        );
        assert_eq!(
            theory.projections().len(),
            0,
            "closed enforcement must not mint pointless indexes"
        );
    }

    #[test]
    fn d04_conflicting_tentative_rows_keep_candidate_multiplicity() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("k", ValueType::U64), field("payload", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let proj = theory.projection(ProjectionId(0)).expect("key");
        let a = proj.scalar_values(&[Value::U64(1), Value::U64(10)]);
        let b = proj.scalar_values(&[Value::U64(1), Value::U64(11)]);
        let ra = encode_scalar_group(&a, &proj.scalar_fields).expect("a");
        let rb = encode_scalar_group(&b, &proj.scalar_fields).expect("b");
        assert_eq!(ra, rb, "same group key");
        assert_ne!(
            [Value::U64(1), Value::U64(10)],
            [Value::U64(1), Value::U64(11)]
        );
        assert_eq!(
            theory.full_row_witness(),
            DistinctnessWitness::FullRowEquality
        );
    }

    #[test]
    fn d04_forced_fingerprint_collision_still_requires_exact_bytes() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("text", ValueType::String)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let theory = compile(&schema);
        let proj = theory.projection(ProjectionId(0)).expect("text key");
        assert_eq!(proj.encoding, KeyEncoding::FingerprintBucket);
        let left = crate::canonical::CanonicalRow::encode(
            &proj.scalar_fields,
            &[Value::String("alpha".into())],
            &work(),
        )
        .expect("left")
        .as_bytes()
        .to_vec();
        let right = crate::canonical::CanonicalRow::encode(
            &proj.scalar_fields,
            &[Value::String("beta".into())],
            &work(),
        )
        .expect("right")
        .as_bytes()
        .to_vec();
        assert_ne!(left, right, "exact confirmation bytes remain distinct");
        let outcome = CompiledTheory::consume_visits(
            DistinctnessWitness::FullRowEquality,
            [left.as_slice(), right.as_slice()],
            &mut |_| Ok::<_, ()>(VisitControl::Sufficient),
        )
        .expect("walk");
        assert_eq!(
            outcome,
            VisitOutcome::Exhausted { visited: 2 },
            "full-row checks are not dropped for a compact/fingerprint path"
        );
    }

    #[test]
    fn d10_existence_only_stops_after_first_sufficient() {
        let witness = DistinctnessWitness::ExistenceOnly {
            projection: ProjectionId(0),
        };
        let mut seen = Vec::new();
        let outcome = CompiledTheory::consume_visits(witness, [10, 20, 30], &mut |item| {
            seen.push(item);
            Ok::<_, ()>(VisitControl::Sufficient)
        })
        .expect("walk");
        assert_eq!(outcome, VisitOutcome::Sufficient { visited: 1 });
        assert_eq!(seen, vec![10]);
    }

    #[test]
    fn d10_sink_stop_and_source_error_forbid_later_visits() {
        let witness = DistinctnessWitness::ScalarKeyUnique {
            projection: ProjectionId(0),
        };
        let mut seen = Vec::new();
        let stopped = CompiledTheory::consume_visits(witness, [1, 2, 3], &mut |item| {
            seen.push(item);
            Ok::<_, ()>(VisitControl::Stop)
        })
        .expect("stop");
        assert_eq!(stopped, VisitOutcome::Stopped { visited: 1 });
        assert_eq!(seen, vec![1]);

        seen.clear();
        let err = CompiledTheory::consume_visits(witness, [4, 5, 6], &mut |item| {
            seen.push(item);
            Err("source")
        });
        assert_eq!(err, Err("source"));
        assert_eq!(seen, vec![4]);
    }

    #[test]
    fn assign_projection_id_is_explicitly_exhausted() {
        assert_eq!(
            assign_projection_id(usize::from(u16::MAX) + 1),
            Err(CompileError::ProjectionIdExhausted)
        );
        assert_eq!(assign_projection_id(0), Ok(ProjectionId(0)));
        assert_eq!(
            assign_projection_id(usize::from(u16::MAX)),
            Ok(ProjectionId(u16::MAX))
        );
    }

    fn work() -> crate::WorkContext {
        crate::work::ExecutionPolicy {
            input_bytes: 1_000_000,
            working_bytes: 1_000_000,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 1000,
            work_units: 1_000_000,
            timeout: std::time::Duration::from_secs(60),
        }
        .start()
        .expect("work")
    }
}
