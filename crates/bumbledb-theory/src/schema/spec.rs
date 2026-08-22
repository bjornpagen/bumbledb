//! `SchemaSpec` — the bindings contract: a schema as **named plain data**,
//! the runtime peer of the `schema!` grammar. A foreign host (the Node
//! bindings, ETL tooling, any language that can build owned strings,
//! vectors, and integers) describes its theory here and lowers it to the
//! produce indistinguishable descriptors, so the same theory built either
//! way carries the same fingerprint.
use std::collections::BTreeMap;

use super::{
    Bound, FieldDescriptor, FieldId, Generation, LiteralSet, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight,
};
use crate::value::Value;

/// The whole theory as named plain data: relations (ordinary and closed)
/// and dependency statements, each list in declaration order — the same
/// declaration-order law that mints every id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSpec {
    pub relations: Vec<RelationSpec>,
    pub statements: Vec<StatementSpec>,
}

/// One relation. `closed: Some(spec)` declares it **closed** (the option
/// is the kind, mirroring [`RelationDescriptor::extension`]); a closed
/// relation's `fields` are its declared intrinsic columns only — the
/// synthetic (`id`, `u64`) handle field is materialized by schema
/// validation, and statement field names address the sealed shape (`id`
/// resolves to [`FieldId`] 0, declared columns shift by one), exactly as
/// the macro resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSpec {
    pub name: Box<str>,
    pub fields: Vec<FieldSpec>,

    /// (ruled 2026-07-23, R7).
    pub closed: Option<ClosedSpec>,
}

/// A closed relation's closed half, fused: the handle newtype and the
/// ground axioms travel together, so the two states the grammar forbids
/// — an ordinary relation carrying a handle newtype, a closed relation
/// without one — are unrepresentable, exactly
/// as the macro's mandatory `as NewType` makes them unspellable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSpec {
    pub newtype: Box<str>,

    pub rows: Vec<RowSpec>,
}

/// One field: name, structural type, host newtype name, and the `fresh`
/// mark. [`ValueType`] is the one structural-type vocabulary — `bool`,
/// `u64`, `i64`, `str` ([`ValueType::String`]), `bytes<N>`
/// ([`ValueType::FixedBytes`]), and the interval family
/// ([`ValueType::Interval`] / [`ValueType::FixedInterval`]) — so the spec can
/// state every type the grammar can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: Box<str>,
    pub value_type: ValueType,

    pub newtype: Option<Box<str>>,

    pub fresh: bool,
}

/// One ground axiom of a closed relation: the handle plus one literal per
/// declared intrinsic column, in field-declaration order. Column literals
/// ride the same [`LiteralSpec`] machine as statement selections (one
/// machine, same errors — the macro's own rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowSpec {
    pub handle: Box<str>,
    pub values: Vec<LiteralSpec>,
}

/// One literal as spelled: a plain [`Value`], or a closed relation's
/// handle by name — the `| status == Frozen` spelling, resolved through
/// the selected field's newtype to the handle's declaration-order row id
/// (a `u64` word), exactly as the macro resolves it at expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSpec {
    Value(Value),
    Handle(Box<str>),
}

/// One σ binding's right side: a single literal or a literal set (read
/// disjunctively). The degenerate sets are banned exactly as the macro
/// bans them (the canonical-utterance law): a one-element set is the bare
/// literal, and an empty set selects nothing — write no binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSetSpec {
    One(LiteralSpec),
    Many(Vec<LiteralSpec>),
}

/// One side of a containment or capacity statement:
/// `R(fields… | field == literal…)`, all names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideSpec {
    pub relation: Box<str>,

    pub projection: Vec<Box<str>>,

    pub selection: Vec<(Box<str>, LiteralSetSpec)>,
}

/// A capacity statement's weight as spelled: the measure of one source
/// fact — `Unit` the absent bracket (the count instance), `Field` a
/// u64-encoded SOURCE field by name, `Duration` a SOURCE interval
/// position's measure (`[Duration(field)]`). A dotted `Field` name is
/// the path spelling — representable here (a wire crossing carries what
/// it carries) and refused by [`SchemaSpec::descriptor`] as
/// [`SpecIssue::WeightPathRefused`], whose `Display` names the
/// pinned-column composition idiom (ruled 2026-07-24, ruling 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightSpec {
    Unit,

    Field(Box<str>),

    Duration(Box<str>),
}

/// One capacity bound as spelled: an integer literal, a field of
/// TARGET's row by name (the dependent bound — resolved against the
/// target's WHOLE field roster, ruled 2026-07-24, C1), or a TARGET
/// interval position's measure (`Duration(field)`). Dependent bounds
/// are hi-slot only (ruled 2026-07-24, C6): a dependent floor or exact
/// is representable here and refused by [`SchemaSpec::descriptor`] as
/// [`SpecIssue::CapacityDependentFloor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundSpec {
    Lit(u64),

    Field(Box<str>),

    Duration(Box<str>),
}

/// A capacity statement's window as spelled — the macro's surviving
/// spellings, each otherwise unrepresentable: `{n}` exact (`{0}` the
/// exclusion), `{lo..hi}` with lo < hi (hi may be a dependent
/// [`BoundSpec`]), `{lo..*}` floors (unit instance: lo ≥ 2), `{0..hi}`
/// ceilings. The banned spellings are representable here (a wire
/// crossing carries what it carries) and rejected by
/// [`SchemaSpec::descriptor`] with the canonical form named — the same
/// per-aggregate ban table the macro enforces at expansion (a ban is
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapacityWindowSpec {
    Exact(BoundSpec),

    Range { lo: BoundSpec, hi: BoundSpec },

    Floor(BoundSpec),
}

/// One dependency statement, tagged by form. `==` is not a variant:
/// exactly as in the grammar, a bidirectional containment is the
/// `Containment { bidirectional: true }` spelling, lowered to the two
/// adjacent containment descriptors (`source <= target` first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementSpec {
    Fd {
        relation: Box<str>,
        projection: Vec<Box<str>>,
    },

    Containment {
        source: SideSpec,
        target: SideSpec,
        bidirectional: bool,
    },

    /// weight, window, source (ruled 2026-07-24, C2).
    Capacity {
        target: SideSpec,
        weight: WeightSpec,
        window: CapacityWindowSpec,
        source: SideSpec,
    },
}

/// half of [`LiteralAt::Selection`]'s address. FDs carry no selection
/// (the shape is unrepresentable), so two sides name every binding site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatementSide {
    Source,
    Target,
}

/// provenances a literal can have (a statement side's σ binding, a
/// closed relation's extension row), with no third. Carried by the
/// handle-shaped issues so a holder of the spec's source tokens (the
/// `schema!` macro's span table) can mark the offending token itself,
/// never the whole invocation. `Ord` because it is a map key there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralAt {
    Selection {
        statement: usize,
        side: StatementSide,
        binding: usize,
        literal: usize,
    },

    Row {
        relation: usize,
        row: usize,
        column: usize,
    },
}

/// One face of a paired-face statement as the coherence check cites it:
/// the relation and field the projection names at the offending position,
/// plus the newtype label that column carries — a closed relation's
/// synthetic `id` carries the handle newtype; `None` is the bare column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceNewtype {
    pub relation: Box<str>,
    pub field: Box<str>,
    pub newtype: Option<Box<str>>,
}

impl FaceNewtype {
    #[must_use]
    pub fn cite(&self) -> String {
        match &self.newtype {
            Some(newtype) => format!("`{}.{}` (`{newtype}`)", self.relation, self.field),
            None => format!("`{}.{}` (no newtype)", self.relation, self.field),
        }
    }
}

/// One resolution failure of [`SchemaSpec::descriptor`] — a name the spec
/// used that its own declarations never introduce, or a banned spelling
/// of the canonical-utterance law. `statement` payloads index
/// [`SchemaSpec::statements`] (the spec's own order, before `==`
/// lowering); handle-shaped payloads carry [`LiteralAt`], the literal's
/// structural address, alongside the names `Display` speaks.
/// Every capacity-window and literal-set variant's `Display` names the
/// — an
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecIssue {
    UnknownRelation {
        statement: usize,
        relation: Box<str>,
    },

    UnknownField {
        statement: usize,
        relation: Box<str>,
        field: Box<str>,
    },

    NotAHandleField {
        at: LiteralAt,
        relation: Box<str>,
        field: Box<str>,
        handle: Box<str>,
    },

    UnknownHandle {
        at: LiteralAt,
        closed: Box<str>,
        handle: Box<str>,
    },

    /// [`FieldId`] past it can be minted, so the cap runs before any
    RelationTooManyFields {
        relation: usize,
        name: Box<str>,

        fields: usize,
    },

    RowArityExcess {
        relation: usize,
        row: usize,
        name: Box<str>,
        declared: usize,
        supplied: usize,
    },

    DuplicateHandleNewtype {
        newtype: Box<str>,

        first_relation: usize,
        second_relation: usize,
        first: Box<str>,
        second: Box<str>,
    },

    CapacityInverted {
        statement: usize,
        lo: u64,
        hi: u64,
    },

    CapacityExactRespelled {
        statement: usize,
        count: u64,
    },

    CapacityExclusionRespelled {
        statement: usize,
    },

    /// (`lean/Bumbledb/Capacity.lean: capacity_zero_star`).
    CapacityVacuous {
        statement: usize,
    },

    /// 2026-07-24): `<=[w]{1..*}` — "positive total" — is a different,
    CapacityContainmentRespelled {
        statement: usize,
    },

    /// are hi-slot only (ruled 2026-07-24, C6): a dependent floor has no
    CapacityDependentFloor {
        statement: usize,
    },

    /// Unit floors are refused whole (K16): a bare count floor `{N..*}`
    /// has no user; floors are legal only on weighted measures.
    CapacityUnitFloor {
        statement: usize,
    },

    /// vocabulary is closed at the row (ruled 2026-07-24, ruling 6):
    WeightPathRefused {
        statement: usize,
        path: Box<str>,
    },

    /// spelling, one refusal, every authoring wall.
    BoundPathRefused {
        statement: usize,
        path: Box<str>,
    },

    DegenerateLiteralSet {
        statement: usize,
        field: Box<str>,
        len: usize,
    },

    StatementNewtypeMismatch {
        statement: usize,

        position: usize,
        source: FaceNewtype,
        target: FaceNewtype,
    },
}

impl std::fmt::Display for SpecIssue {
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per issue, each a paste-back instruction — \
                  clearer kept together (the `descriptor` precedent)"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRelation {
                statement,
                relation,
            } => write!(
                f,
                "statement {statement}: relation `{relation}` is not declared in this spec"
            ),
            Self::UnknownField {
                statement,
                relation,
                field,
            } => write!(
                f,
                "statement {statement}: relation `{relation}` has no field `{field}`"
            ),
            Self::NotAHandleField {
                relation,
                field,
                handle,
                ..
            } => write!(
                f,
                "`{relation}.{field}` is not a closed-relation reference — the handle \
                 literal `{handle}` is legal only on a field whose newtype is a closed \
                 relation's handle newtype"
            ),
            Self::UnknownHandle { closed, handle, .. } => {
                write!(f, "closed relation `{closed}` has no handle `{handle}`")
            }
            Self::RelationTooManyFields { name, fields, .. } => write!(
                f,
                "relation `{name}` seals {fields} fields — the u16 field-id space \
                 caps a relation at 65,535 sealed fields (a closed relation's \
                 synthetic `id` included)"
            ),
            Self::RowArityExcess {
                row,
                name,
                declared,
                supplied,
                ..
            } => write!(
                f,
                "closed relation `{name}`, row {row}: {supplied} values for {declared} \
                 declared columns — the extra literals have no column to lower into"
            ),
            Self::DuplicateHandleNewtype {
                newtype,
                first,
                second,
                ..
            } => write!(
                f,
                "handle newtype `{newtype}` is declared by two closed relations \
                 (`{first}` and `{second}`) — a handle newtype names exactly one \
                 closed relation"
            ),
            Self::CapacityInverted { statement, lo, hi } => write!(
                f,
                "statement {statement}: the window `{{{lo}..{hi}}}` is inverted — no \
                 measure satisfies it; bounds are `{{lo..hi}}` with lo < hi (an exact \
                 measure is `{{n}}`)"
            ),
            Self::CapacityExactRespelled { statement, count } => write!(
                f,
                "statement {statement}: `{{{count}..{count}}}` — an exact measure is \
                 written `{{{count}}}`"
            ),
            Self::CapacityExclusionRespelled { statement } => write!(
                f,
                "statement {statement}: `{{0..0}}` — the exclusion is written `{{0}}`"
            ),
            Self::CapacityVacuous { statement } => write!(
                f,
                "statement {statement}: the `{{0..*}}` window is vacuous — it provably \
                 says nothing; delete the statement"
            ),
            Self::CapacityContainmentRespelled { statement } => write!(
                f,
                "statement {statement}: unit `{{1..*}}` says only what the bare \
                 containment says — drop the annotation and write the containment"
            ),
            Self::CapacityDependentFloor { statement } => write!(
                f,
                "statement {statement}: a dependent bound in the floor slot — \
                 dependent bounds are hi-slot only (ruled 2026-07-24, C6): a \
                 dependent floor has no use case; write a literal floor"
            ),
            Self::CapacityUnitFloor { statement } => write!(
                f,
                "statement {statement}: `{{N..*}}` on the unit instance — a \
                 bare count floor is refused; weigh the source (`<=[w]{{N..*}}` \
                 stays legal) or drop the bound"
            ),
            Self::WeightPathRefused { statement, path } => write!(
                f,
                "statement {statement}: the weight path `[{path}]` is refused — the \
                 weight vocabulary is closed at the row; state the join as a law and \
                 read the local column (the pinned-column idiom): \
                 `Device(model, watts) <= Model(id, watts); \
                 Pool(id) <=[watts]{{0..supply}} Device(pool);`"
            ),
            Self::BoundPathRefused { statement, path } => write!(
                f,
                "statement {statement}: the bound path `{{..{path}}}` is refused — a \
                 dependent bound names a field of the TARGET's own row, closed at the \
                 row exactly like the weight; state the join as a law and read the \
                 local column (the pinned-column idiom): \
                 `Pool(id, supply) <= Grid(pool, supply); \
                 Pool(id) <=[watts]{{0..supply}} Device(pool);`"
            ),
            Self::DegenerateLiteralSet {
                statement,
                field,
                len: 0,
            } => write!(
                f,
                "statement {statement}: the literal set for `{field}` is empty — an \
                 empty set selects nothing; write no binding"
            ),
            Self::DegenerateLiteralSet {
                statement, field, ..
            } => write!(
                f,
                "statement {statement}: the literal set for `{field}` has one element \
                 — a one-element set is the bare literal: write `{field} == L`, no \
                 braces"
            ),
            Self::StatementNewtypeMismatch {
                statement,
                position,
                source,
                target,
            } => write!(
                f,
                "statement {statement}: position {position} pairs {} with {} — the \
                 faces of a dependency agree on their newtype, or neither carries one",
                source.cite(),
                target.cite()
            ),
        }
    }
}

/// [`SchemaSpec::descriptor`]'s typed failure: the COMPLETE issue list —
/// every unresolvable name and every banned spelling, in spec order —
/// never the first offender alone (the engine `Violations` precedent: a
/// foreign host repairs its whole spec in one round trip). Sealed
/// nonempty by the one construction site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSpecError(Box<[SpecIssue]>);

impl SchemaSpecError {
    #[must_use]
    pub fn issues(&self) -> &[SpecIssue] {
        &self.0
    }
}

impl std::fmt::Display for SchemaSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "schema spec resolution failed:")?;
        for issue in &self.0 {
            write!(f, "\n  - {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SchemaSpecError {}

struct Resolver<'spec> {
    spec: &'spec SchemaSpec,

    handles: BTreeMap<&'spec str, usize>,
    issues: Vec<SpecIssue>,
}

#[derive(Clone, Copy)]
struct SealedSlot<'spec> {
    field: FieldId,
    newtype: Option<&'spec str>,
}

impl<'spec> Resolver<'spec> {
    fn relation(&mut self, statement: usize, name: &str) -> Option<usize> {
        let found = self.spec.relations.iter().position(|r| &*r.name == name);
        if found.is_none() {
            self.issues.push(SpecIssue::UnknownRelation {
                statement,
                relation: name.into(),
            });
        }
        found
    }

    fn slot(&self, rel_idx: usize, name: &str) -> Option<SealedSlot<'spec>> {
        let relation = &self.spec.relations[rel_idx];
        if let (Some(closed), "id") = (&relation.closed, name) {
            return Some(SealedSlot {
                field: FieldId(0),
                newtype: Some(&closed.newtype),
            });
        }
        let index = relation.fields.iter().position(|f| &*f.name == name)?;
        let sealed = index + usize::from(relation.closed.is_some());

        Some(SealedSlot {
            field: FieldId(u16::try_from(sealed).unwrap_or(0)),
            newtype: relation.fields[index].newtype.as_deref(),
        })
    }

    fn field(&mut self, statement: usize, rel_idx: usize, name: &str) -> Option<SealedSlot<'spec>> {
        let slot = self.slot(rel_idx, name);
        if slot.is_none() {
            self.issues.push(SpecIssue::UnknownField {
                statement,
                relation: self.spec.relations[rel_idx].name.clone(),
                field: name.into(),
            });
        }
        slot
    }

    fn coherent(&mut self, statement: usize, source: &SideSpec, target: &SideSpec) {
        let position_of = |name: &str| self.spec.relations.iter().position(|r| &*r.name == name);
        let (Some(source_rel), Some(target_rel)) =
            (position_of(&source.relation), position_of(&target.relation))
        else {
            return;
        };
        for (position, (source_field, target_field)) in
            source.projection.iter().zip(&target.projection).enumerate()
        {
            let (Some(source_slot), Some(target_slot)) = (
                self.slot(source_rel, source_field),
                self.slot(target_rel, target_field),
            ) else {
                continue;
            };
            if source_slot.newtype == target_slot.newtype {
                continue;
            }
            let face = |rel_idx: usize, field: &str, newtype: Option<&str>| FaceNewtype {
                relation: self.spec.relations[rel_idx].name.clone(),
                field: field.into(),
                newtype: newtype.map(Into::into),
            };
            let source_face = face(source_rel, source_field, source_slot.newtype);
            let target_face = face(target_rel, target_field, target_slot.newtype);
            self.issues.push(SpecIssue::StatementNewtypeMismatch {
                statement,
                position,
                source: source_face,
                target: target_face,
            });
        }
    }

    fn literal(
        &mut self,
        at: LiteralAt,
        rel_idx: usize,
        field: &str,
        newtype: Option<&str>,
        literal: &LiteralSpec,
    ) -> Value {
        match literal {
            LiteralSpec::Value(value) => value.clone(),
            LiteralSpec::Handle(handle) => {
                let owner = newtype.and_then(|newtype| self.handles.get(newtype).copied());
                let Some(owner) = owner else {
                    self.issues.push(SpecIssue::NotAHandleField {
                        at,
                        relation: self.spec.relations[rel_idx].name.clone(),
                        field: field.into(),
                        handle: handle.clone(),
                    });
                    return Value::U64(0);
                };
                let rows = &self.spec.relations[owner]
                    .closed
                    .as_ref()
                    .expect("the handle namespace holds closed relations only")
                    .rows;
                let Some(row) = rows.iter().position(|row| row.handle == *handle) else {
                    self.issues.push(SpecIssue::UnknownHandle {
                        at,
                        closed: self.spec.relations[owner].name.clone(),
                        handle: handle.clone(),
                    });
                    return Value::U64(0);
                };
                Value::U64(u64::try_from(row).expect("row count fits u64"))
            }
        }
    }

    fn side(&mut self, statement: usize, which: StatementSide, side: &SideSpec) -> Side {
        let Some(rel_idx) = self.relation(statement, &side.relation) else {
            return Side {
                relation: RelationId(0),
                projection: Box::new([]),
                selection: Box::new([]),
            };
        };
        let mut projection = Vec::with_capacity(side.projection.len());
        for field in &side.projection {
            if let Some(slot) = self.field(statement, rel_idx, field) {
                projection.push(slot.field);
            }
        }
        let mut selection = Vec::with_capacity(side.selection.len());
        for (binding, (field, literals)) in side.selection.iter().enumerate() {
            let Some(slot) = self.field(statement, rel_idx, field) else {
                continue;
            };
            let at = |literal: usize| LiteralAt::Selection {
                statement,
                side: which,
                binding,
                literal,
            };
            let set = match literals {
                LiteralSetSpec::One(literal) => {
                    LiteralSet::One(self.literal(at(0), rel_idx, field, slot.newtype, literal))
                }
                LiteralSetSpec::Many(many) if many.len() < 2 => {
                    self.issues.push(SpecIssue::DegenerateLiteralSet {
                        statement,
                        field: field.clone(),
                        len: many.len(),
                    });
                    continue;
                }
                LiteralSetSpec::Many(many) => LiteralSet::Many(
                    many.iter()
                        .enumerate()
                        .map(|(index, literal)| {
                            self.literal(at(index), rel_idx, field, slot.newtype, literal)
                        })
                        .collect(),
                ),
            };
            selection.push((slot.field, set));
        }
        Side {
            relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
            projection: projection.into_boxed_slice(),
            selection: selection.into_boxed_slice(),
        }
    }

    fn weight(
        &mut self,
        statement: usize,
        source_rel: Option<usize>,
        weight: &WeightSpec,
    ) -> Weight {
        let name = match weight {
            WeightSpec::Unit => return Weight::Unit,
            WeightSpec::Field(name) | WeightSpec::Duration(name) => name,
        };
        if name.contains('.') {
            self.issues.push(SpecIssue::WeightPathRefused {
                statement,
                path: name.clone(),
            });
            return Weight::Unit;
        }
        let Some(rel_idx) = source_rel else {
            return Weight::Unit;
        };
        let Some(slot) = self.field(statement, rel_idx, name) else {
            return Weight::Unit;
        };
        match weight {
            WeightSpec::Unit => unreachable!("Unit returned above"),
            WeightSpec::Field(_) => Weight::Field(slot.field),
            WeightSpec::Duration(_) => Weight::DurationOf(slot.field),
        }
    }

    fn bound(&mut self, statement: usize, target_rel: Option<usize>, bound: &BoundSpec) -> Bound {
        let name = match bound {
            BoundSpec::Lit(n) => return Bound::Lit(*n),
            BoundSpec::Field(name) | BoundSpec::Duration(name) => name,
        };
        if name.contains('.') {
            self.issues.push(SpecIssue::BoundPathRefused {
                statement,
                path: name.clone(),
            });
            return Bound::Lit(0);
        }
        let Some(rel_idx) = target_rel else {
            return Bound::Lit(0);
        };
        let Some(slot) = self.field(statement, rel_idx, name) else {
            return Bound::Lit(0);
        };
        match bound {
            BoundSpec::Lit(_) => unreachable!("Lit returned above"),
            BoundSpec::Field(_) => Bound::TargetField(slot.field),
            BoundSpec::Duration(_) => Bound::TargetDuration(slot.field),
        }
    }

    fn capacity_window(
        &mut self,
        statement: usize,
        unit: bool,
        target_rel: Option<usize>,
        window: &CapacityWindowSpec,
    ) -> (u64, Option<Bound>) {
        let lit = |resolver: &mut Self, bound: &BoundSpec, floor_slot: bool| match bound {
            BoundSpec::Lit(n) => Some(*n),
            BoundSpec::Field(_) | BoundSpec::Duration(_) => {
                if floor_slot {
                    resolver
                        .issues
                        .push(SpecIssue::CapacityDependentFloor { statement });
                }
                None
            }
        };
        match window {
            CapacityWindowSpec::Exact(bound) => {
                let n = lit(self, bound, true).unwrap_or(0);
                (n, Some(Bound::Lit(n)))
            }
            CapacityWindowSpec::Range { lo, hi } => {
                let lo = lit(self, lo, true).unwrap_or(0);
                match lit(self, hi, false) {
                    Some(hi) if hi < lo => {
                        self.issues
                            .push(SpecIssue::CapacityInverted { statement, lo, hi });
                        (lo, None)
                    }
                    Some(0) if lo == 0 => {
                        self.issues
                            .push(SpecIssue::CapacityExclusionRespelled { statement });
                        (0, Some(Bound::Lit(0)))
                    }
                    Some(hi) if lo == hi => {
                        self.issues.push(SpecIssue::CapacityExactRespelled {
                            statement,
                            count: lo,
                        });
                        (lo, Some(Bound::Lit(hi)))
                    }
                    Some(hi) => (lo, Some(Bound::Lit(hi))),

                    None => (lo, Some(self.bound(statement, target_rel, hi))),
                }
            }
            CapacityWindowSpec::Floor(bound) => match lit(self, bound, true) {
                Some(0) => {
                    self.issues.push(SpecIssue::CapacityVacuous { statement });
                    (0, None)
                }
                Some(1) if unit => {
                    self.issues
                        .push(SpecIssue::CapacityContainmentRespelled { statement });
                    (1, None)
                }
                Some(lo) if unit => {
                    self.issues.push(SpecIssue::CapacityUnitFloor { statement });
                    (lo, None)
                }
                Some(lo) => (lo, None),
                None => (0, None),
            },
        }
    }
}

impl SchemaSpec {
    /// # Errors
    ///
    /// [`SchemaSpecError`] carrying every unresolvable name, banned spelling,
    /// over-wide extension row, and past-u16 field roster, in spec order.
    ///
    /// # Panics
    ///
    /// Only on one programmer-invariant violation: more than 2³²
    /// relations — unreachable (the spec's own relations vector exceeds
    /// memory first; the engine's `validate` states the same bound).
    #[expect(
        clippy::too_many_lines,
        reason = "the one lowering pass — one arm per statement form, \
                  clearer kept together (the `validate` precedent)"
    )]
    pub fn descriptor(&self) -> Result<SchemaDescriptor, SchemaSpecError> {
        let mut resolver = Resolver {
            spec: self,
            handles: BTreeMap::new(),
            issues: Vec::new(),
        };
        // The sealed-field cap runs FIRST — before any statement

        for (idx, relation) in self.relations.iter().enumerate() {
            let sealed = relation.fields.len() + usize::from(relation.closed.is_some());
            if sealed > usize::from(u16::MAX) {
                resolver.issues.push(SpecIssue::RelationTooManyFields {
                    relation: idx,
                    name: relation.name.clone(),
                    fields: sealed,
                });
            }
        }
        for (idx, relation) in self.relations.iter().enumerate() {
            let Some(closed) = &relation.closed else {
                continue;
            };
            if let Some(first) = resolver.handles.insert(&closed.newtype, idx) {
                resolver.issues.push(SpecIssue::DuplicateHandleNewtype {
                    newtype: closed.newtype.clone(),
                    first_relation: first,
                    second_relation: idx,
                    first: self.relations[first].name.clone(),
                    second: relation.name.clone(),
                });
            }
        }

        let relations = self
            .relations
            .iter()
            .enumerate()
            .map(|(rel_idx, relation)| RelationDescriptor {
                name: relation.name.clone(),
                fields: relation
                    .fields
                    .iter()
                    .map(|field| FieldDescriptor {
                        name: field.name.clone(),
                        value_type: field.value_type,
                        generation: if field.fresh {
                            Generation::Fresh
                        } else {
                            Generation::None
                        },
                    })
                    .collect(),
                extension: relation.closed.as_ref().map(|closed| {
                    closed
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(row_idx, row)| {
                            if row.values.len() > relation.fields.len() {
                                resolver.issues.push(SpecIssue::RowArityExcess {
                                    relation: rel_idx,
                                    row: row_idx,
                                    name: relation.name.clone(),
                                    declared: relation.fields.len(),
                                    supplied: row.values.len(),
                                });
                            }
                            Row {
                                handle: row.handle.clone(),
                                values: row
                                    .values
                                    .iter()
                                    .zip(&relation.fields)
                                    .enumerate()
                                    .map(|(column, (literal, field))| {
                                        let at = LiteralAt::Row {
                                            relation: rel_idx,
                                            row: row_idx,
                                            column,
                                        };
                                        resolver.literal(
                                            at,
                                            rel_idx,
                                            &field.name,
                                            field.newtype.as_deref(),
                                            literal,
                                        )
                                    })
                                    .collect(),
                            }
                        })
                        .collect()
                }),
            })
            .collect();

        let mut statements = Vec::with_capacity(self.statements.len());
        for (index, statement) in self.statements.iter().enumerate() {
            match statement {
                StatementSpec::Fd {
                    relation,
                    projection,
                } => {
                    let mut fields = Vec::with_capacity(projection.len());
                    let relation = match resolver.relation(index, relation) {
                        Some(rel_idx) => {
                            for field in projection {
                                if let Some(slot) = resolver.field(index, rel_idx, field) {
                                    fields.push(slot.field);
                                }
                            }
                            RelationId(u32::try_from(rel_idx).expect("relation count fits u32"))
                        }

                        None => RelationId(0),
                    };
                    statements.push(StatementDescriptor::Functionality {
                        relation,
                        projection: fields.into_boxed_slice(),
                    });
                }
                StatementSpec::Containment {
                    source,
                    target,
                    bidirectional,
                } => {
                    resolver.coherent(index, source, target);
                    let source = resolver.side(index, StatementSide::Source, source);
                    let target = resolver.side(index, StatementSide::Target, target);
                    if *bidirectional {
                        statements.push(StatementDescriptor::Containment {
                            source: source.clone(),
                            target: target.clone(),
                        });
                        statements.push(StatementDescriptor::Containment {
                            source: target,
                            target: source,
                        });
                    } else {
                        statements.push(StatementDescriptor::Containment { source, target });
                    }
                }
                StatementSpec::Capacity {
                    target,
                    weight,
                    window,
                    source,
                } => {
                    resolver.coherent(index, source, target);

                    let position_of =
                        |name: &str| self.relations.iter().position(|r| &*r.name == name);
                    let source_rel = position_of(&source.relation);
                    let target_rel = position_of(&target.relation);

                    // placeholder must not widen the ban table).
                    let unit = matches!(weight, WeightSpec::Unit);
                    let weight = resolver.weight(index, source_rel, weight);
                    let (lo, hi) = resolver.capacity_window(index, unit, target_rel, window);
                    let source = resolver.side(index, StatementSide::Source, source);
                    let target = resolver.side(index, StatementSide::Target, target);
                    statements.push(StatementDescriptor::Capacity {
                        target,
                        weight,
                        lo,
                        hi,
                        source,
                    });
                }
            }
        }

        if resolver.issues.is_empty() {
            Ok(SchemaDescriptor {
                relations,
                statements,
            })
        } else {
            Err(SchemaSpecError(resolver.issues.into_boxed_slice()))
        }
    }
}
