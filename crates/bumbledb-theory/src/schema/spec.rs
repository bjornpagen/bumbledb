//! `SchemaSpec` — the bindings contract (`docs/architecture/70-api.md`
//! § the `SchemaSpec` bindings contract): a schema as **named plain data**,
//! the runtime peer of the `schema!` grammar. A foreign host (the Node
//! bindings, ETL tooling, any language that can build owned strings,
//! vectors, and integers) describes its theory here and lowers it to the
//! [`SchemaDescriptor`] the engine already takes — the macro and the spec
//! produce indistinguishable descriptors, so the same theory built either
//! way carries the same fingerprint.
//!
//! The division of labor mirrors the macro's exactly:
//!
//! - [`SchemaSpec::descriptor`] does what macro EXPANSION does — name→id
//!   resolution (relations, fields, closed-relation handles), the
//!   canonical-utterance ban table over window spellings and literal
//!   sets, and the newtype-coherence check over every statement's paired
//!   faces (`docs/architecture/30-dependencies.md` § the taxonomy is
//!   checked; authoring-time only — newtypes never reach the descriptor)
//!   — surfacing every failure as the typed [`SchemaSpecError`],
//!   which enumerates ALL unresolvable names and banned spellings rather
//!   than stopping at the first (a foreign host gets one round trip).
//! - Everything semantic beyond names stays where the macro defers it:
//!   the engine's `SchemaDescriptor::validate` inside `Db::create` /
//!   `Db::open`, as the typed `SchemaError`.
//!
//! No serde, no wire format: the spec is owned Rust data
//! (`String`/`Vec`/integers); a bindings crate marshals it however it
//! likes and the engine never learns the encoding.

use std::collections::BTreeMap;

use super::{
    FieldDescriptor, FieldId, Generation, LiteralSet, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
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
    /// Closedness as one sum: `Some` = closed, `None` = ordinary
    /// (ruled 2026-07-23, R7).
    pub closed: Option<ClosedSpec>,
}

/// A closed relation's closed half, fused: the handle newtype and the
/// ground axioms travel together, so the two states the grammar forbids
/// — an ordinary relation carrying a handle newtype, a closed relation
/// without one — are unrepresentable (`docs/architecture/70-api.md`
/// § the `SchemaSpec` bindings contract; ruled 2026-07-23, R7), exactly
/// as the macro's mandatory `as NewType` makes them unspellable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSpec {
    /// The handle newtype name (the macro's mandatory `as NewType`) —
    /// host-side nominal vocabulary, never a fingerprint input: it
    /// exists so [`LiteralSpec::Handle`] literals can resolve through a
    /// referencing field's [`FieldSpec::newtype`], and it is dropped at
    /// lowering (the descriptor never carries names of host types).
    pub newtype: Box<str>,
    /// The ground axioms in declaration order (row id = index).
    pub rows: Vec<RowSpec>,
}

/// One field: name, structural type, host newtype name, and the `fresh`
/// mark. [`ValueType`] is the one structural-type vocabulary — `bool`,
/// `u64`, `i64`, `str` ([`ValueType::String`]), `bytes<N>`
/// ([`ValueType::FixedBytes`]), and the interval family
/// ([`ValueType::Interval`], general or fixed-width) — so the spec can
/// state every type the grammar can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: Box<str>,
    pub value_type: ValueType,
    /// The host newtype name (the macro's `as NewType`) — carried for
    /// handle resolution only ([`LiteralSpec::Handle`] resolves through
    /// the selected field's newtype to its closed relation, the macro's
    /// own rule) and dropped at lowering: two specs differing only in
    /// newtype names lower to identical descriptors, exactly as two
    /// `schema!` invocations differing only in `as` names do.
    pub newtype: Option<Box<str>>,
    /// `fresh` — the mint mark, legal on `u64` (validated at the
    /// engine's `SchemaDescriptor::validate`, as the macro defers it).
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

/// One side of a containment or window: `R(fields… | field == literal…)`,
/// all names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideSpec {
    pub relation: Box<str>,
    /// π — ordered, the statement's written order.
    pub projection: Vec<Box<str>>,
    /// σ — (field, literal-or-set) bindings, read conjunctively.
    pub selection: Vec<(Box<str>, LiteralSetSpec)>,
}

/// A cardinality window's bounds as spelled — the macro's surviving
/// spellings, each otherwise unrepresentable: `{n}` exact (`{0}` the
/// exclusion), `{lo..hi}` with lo < hi, `{lo..*}` floors (lo ≥ 2),
/// `{0..hi}` ceilings. The banned spellings are representable here (a
/// wire crossing carries what it carries) and rejected by
/// [`SchemaSpec::descriptor`] with the canonical form named — the same
/// ban table the macro enforces at expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSpec {
    /// `{n}` — THE exact-count spelling; `{0}` is the exclusion.
    Exact(u64),
    /// `{lo..hi}` — both bounds explicit, lo < hi.
    Range { lo: u64, hi: u64 },
    /// `{lo..*}` — a floor, no ceiling (lo ≥ 2; `{1..*}` is the bare
    /// containment respelled and `{0..*}` says nothing).
    Floor(u64),
}

/// One dependency statement, tagged by form. `==` is not a variant:
/// exactly as in the grammar, a bidirectional containment is the
/// `Containment { bidirectional: true }` spelling, lowered to the two
/// adjacent containment descriptors (`source <= target` first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementSpec {
    /// `R(X) -> R` — the key form. No selection exists on this variant:
    /// the FD-with-selection shape is unrepresentable, as in the
    /// descriptor.
    Fd {
        relation: Box<str>,
        projection: Vec<Box<str>>,
    },
    /// `source(X | φ) <= target(Y | ψ)`; `bidirectional: true` is the
    /// `==` spelling.
    Containment {
        source: SideSpec,
        target: SideSpec,
        bidirectional: bool,
    },
    /// `target(Y | ψ) <={window} source(X | φ)` — B-family, target-left:
    /// the target is the per-group parent, the source is counted.
    Cardinality {
        target: SideSpec,
        window: WindowSpec,
        source: SideSpec,
    },
}

/// Which side of a containment or window a selection binding rides —
/// half of [`LiteralAt::Selection`]'s address. FDs carry no selection
/// (the shape is unrepresentable), so two sides name every binding site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatementSide {
    Source,
    Target,
}

/// The structural address of one literal in a [`SchemaSpec`] — the two
/// provenances a literal can have (a statement side's σ binding, a
/// closed relation's extension row), with no third. Carried by the
/// handle-shaped issues so a holder of the spec's source tokens (the
/// `schema!` macro's span table) can mark the offending token itself,
/// never the whole invocation. `Ord` because it is a map key there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralAt {
    /// `statements[statement]`, the `side` side's `selection[binding]`,
    /// literal `literal` within the binding's set (`0` for the bare
    /// [`LiteralSetSpec::One`] spelling).
    Selection {
        statement: usize,
        side: StatementSide,
        binding: usize,
        literal: usize,
    },
    /// `relations[relation].closed.rows[row].values[column]` — the column
    /// in declared-intrinsic order (the synthetic `id` is no column).
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
    /// The face as an error names it: `` `Rel.field` (`NewType`) `` or
    /// `` `Rel.field` (no newtype) `` — public so the macro's teaching
    /// message and the spec path's `Display` speak one citation.
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
///
/// Every window and literal-set variant's `Display` names the canonical
/// form verbatim as the ban table does (`docs/architecture/70-api.md`
/// § the canonical-utterance law) — an error is a paste-back instruction,
/// not a shrug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecIssue {
    /// A statement names a relation the spec never declares.
    UnknownRelation {
        statement: usize,
        relation: Box<str>,
    },
    /// A projection or selection names a field its relation never
    /// declares (a closed relation's sealed shape includes `id`).
    UnknownField {
        statement: usize,
        relation: Box<str>,
        field: Box<str>,
    },
    /// A handle literal on a field whose newtype names no closed
    /// relation — the handle namespace is per-closed-relation, entered
    /// only through a referencing field's newtype (the macro's rule).
    NotAHandleField {
        /// The offending literal's structural address.
        at: LiteralAt,
        relation: Box<str>,
        field: Box<str>,
        handle: Box<str>,
    },
    /// A handle the named closed relation's extension never declares.
    UnknownHandle {
        /// The offending literal's structural address.
        at: LiteralAt,
        closed: Box<str>,
        handle: Box<str>,
    },
    /// An extension row supplies more values than its relation declares
    /// columns — the excess is unrepresentable in the descriptor (the
    /// column zip has nowhere to put it), so the lowering rejects here
    /// rather than truncate silently. The fewer-values case survives
    /// lowering intact and stays the engine's
    /// `SchemaError::ExtensionArityMismatch` (the two-boundary split).
    RowArityExcess {
        /// The offending row's declaration indices into
        /// [`SchemaSpec::relations`] and its extension.
        relation: usize,
        row: usize,
        name: Box<str>,
        declared: usize,
        supplied: usize,
    },
    /// Two closed relations claim one handle newtype — a handle newtype
    /// names exactly one closed relation.
    DuplicateHandleNewtype {
        newtype: Box<str>,
        /// The claimants' declaration indices into
        /// [`SchemaSpec::relations`]; `second_relation` is the later
        /// claimant — the declaration a caller marks.
        first_relation: usize,
        second_relation: usize,
        first: Box<str>,
        second: Box<str>,
    },
    /// `{hi..lo}` with hi > lo — inverted, unsatisfiable.
    WindowInverted { statement: usize, lo: u64, hi: u64 },
    /// `{n..n}` — an exact count is written `{n}`.
    WindowExactRespelled { statement: usize, count: u64 },
    /// `{0..0}` — the exclusion is written `{0}`.
    WindowExclusionRespelled { statement: usize },
    /// `{0..*}` — vacuous; provably says nothing.
    WindowVacuous { statement: usize },
    /// `{1..*}` — says only what the bare containment says.
    WindowContainmentRespelled { statement: usize },
    /// A `Many` literal set with fewer than two literals — `{L}` is the
    /// bare literal and `{}` selects nothing.
    DegenerateLiteralSet {
        statement: usize,
        field: Box<str>,
        len: usize,
    },
    /// A paired-face statement (containment, `==`, or a cardinality
    /// window) puts two columns whose newtype labels disagree at one
    /// projection position — the coherence check
    /// (`docs/architecture/30-dependencies.md` § the taxonomy is
    /// checked): the faces of a dependency agree on their newtype, or
    /// neither carries one (labeled pairs only with the SAME label,
    /// bare pairs only with bare — the TS wall's own law, adopted so
    /// the two hosts judge identically). Authoring-time only: newtypes
    /// are dropped at lowering, so descriptors, fingerprints, and
    /// stores never see this law.
    StatementNewtypeMismatch {
        statement: usize,
        /// The projection position (0-based) where the faces disagree.
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
            Self::WindowInverted { statement, lo, hi } => write!(
                f,
                "statement {statement}: the window `{{{lo}..{hi}}}` is inverted — no \
                 count satisfies it; bounds are `{{lo..hi}}` with lo < hi (an exact \
                 count is `{{n}}`)"
            ),
            Self::WindowExactRespelled { statement, count } => write!(
                f,
                "statement {statement}: `{{{count}..{count}}}` — an exact count is \
                 written `{{{count}}}`"
            ),
            Self::WindowExclusionRespelled { statement } => write!(
                f,
                "statement {statement}: `{{0..0}}` — the exclusion is written `{{0}}`"
            ),
            Self::WindowVacuous { statement } => write!(
                f,
                "statement {statement}: the `{{0..*}}` window is vacuous — it provably \
                 says nothing; delete the statement"
            ),
            Self::WindowContainmentRespelled { statement } => write!(
                f,
                "statement {statement}: `{{1..*}}` says only what the bare containment \
                 says — drop the annotation and write the containment"
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
    /// Every issue, in spec order.
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

/// The resolution pass's working state: the spec, the handle namespace
/// (newtype → closed relation index), and the issue collector.
struct Resolver<'spec> {
    spec: &'spec SchemaSpec,
    /// Handle newtype → the owning closed relation's index.
    handles: BTreeMap<&'spec str, usize>,
    issues: Vec<SpecIssue>,
}

impl<'spec> Resolver<'spec> {
    /// The relation's declaration index, or an issue.
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

    /// A field's SEALED id within relation `rel_idx` — for a closed
    /// relation, `id` is the synthetic handle field at 0 and declared
    /// columns sit at index + 1, the numbering every sealed statement
    /// addresses (the macro materializes the same shape at parse).
    fn field(&mut self, statement: usize, rel_idx: usize, name: &str) -> Option<FieldId> {
        let relation = &self.spec.relations[rel_idx];
        let closed = relation.closed.is_some();
        if closed && name == "id" {
            return Some(FieldId(0));
        }
        let found = relation.fields.iter().position(|f| &*f.name == name);
        let Some(index) = found else {
            self.issues.push(SpecIssue::UnknownField {
                statement,
                relation: relation.name.clone(),
                field: name.into(),
            });
            return None;
        };
        let sealed = index + usize::from(closed);
        Some(FieldId(
            u16::try_from(sealed).expect("field count fits u16"),
        ))
    }

    /// The newtype of a sealed field position: the synthetic `id` field
    /// of a closed relation carries the relation's handle newtype.
    fn field_newtype(&self, rel_idx: usize, name: &str) -> Option<&'spec str> {
        let relation = &self.spec.relations[rel_idx];
        if let (Some(closed), "id") = (&relation.closed, name) {
            return Some(&closed.newtype);
        }
        relation
            .fields
            .iter()
            .find(|f| &*f.name == name)
            .and_then(|f| f.newtype.as_deref())
    }

    /// Whether relation `rel_idx`'s SEALED shape declares `name` — the
    /// silent twin of [`Resolver::field`], for the checks that run
    /// where resolution already reports the unknown name.
    fn declares(&self, rel_idx: usize, name: &str) -> bool {
        let relation = &self.spec.relations[rel_idx];
        (relation.closed.is_some() && name == "id")
            || relation.fields.iter().any(|f| &*f.name == name)
    }

    /// The coherence check — the newtype law over one statement's
    /// paired faces (`docs/architecture/30-dependencies.md` § the
    /// taxonomy is checked): positionwise over the two projections, the
    /// paired columns' newtype labels must agree — labeled with the
    /// SAME label, bare with bare; a labeled↔bare pairing is the
    /// mismatch too. σ selections never change the pairing (a
    /// ψ-selected face pairs by its projection exactly as a bare one),
    /// and a column in no paired-face statement is untouched — a
    /// deliberately-bare pointer stays legal. Runs on the spec's names,
    /// BEFORE newtype-dropping (authoring-time only — descriptors and
    /// fingerprints carry no newtypes); unresolvable names are skipped
    /// (resolution reports them), and unequal projection arities pair
    /// the common prefix (the engine's `ContainmentArityMismatch` owns
    /// the rest).
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
            if !(self.declares(source_rel, source_field) && self.declares(target_rel, target_field))
            {
                continue;
            }
            let source_newtype = self.field_newtype(source_rel, source_field);
            let target_newtype = self.field_newtype(target_rel, target_field);
            if source_newtype == target_newtype {
                continue;
            }
            let face = |rel_idx: usize, field: &str, newtype: Option<&str>| FaceNewtype {
                relation: self.spec.relations[rel_idx].name.clone(),
                field: field.into(),
                newtype: newtype.map(Into::into),
            };
            let source_face = face(source_rel, source_field, source_newtype);
            let target_face = face(target_rel, target_field, target_newtype);
            self.issues.push(SpecIssue::StatementNewtypeMismatch {
                statement,
                position,
                source: source_face,
                target: target_face,
            });
        }
    }

    /// One literal at its field position — a [`LiteralSpec::Handle`]
    /// resolves through the field's newtype to its closed relation's
    /// declaration-order row id, the macro's own resolution. `at` is the
    /// literal's structural address, carried by the handle-shaped issues.
    /// On an issue the placeholder `Value::U64(0)` stands in;
    /// placeholders never escape (a nonempty issue list fails the whole
    /// construction).
    fn literal(
        &mut self,
        at: LiteralAt,
        rel_idx: usize,
        field: &str,
        literal: &LiteralSpec,
    ) -> Value {
        match literal {
            LiteralSpec::Value(value) => value.clone(),
            LiteralSpec::Handle(handle) => {
                let owner = self
                    .field_newtype(rel_idx, field)
                    .and_then(|newtype| self.handles.get(newtype).copied());
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

    /// One side lowered: names to ids, literal sets through the
    /// degenerate-set ban (the canonical-utterance law — `{L}` is the
    /// bare literal, `{}` is no binding), handles through the namespace.
    /// `which` tags the side for the handle-shaped issues' addresses.
    fn side(&mut self, statement: usize, which: StatementSide, side: &SideSpec) -> Option<Side> {
        let rel_idx = self.relation(statement, &side.relation)?;
        let mut projection = Vec::with_capacity(side.projection.len());
        for field in &side.projection {
            if let Some(id) = self.field(statement, rel_idx, field) {
                projection.push(id);
            }
        }
        let mut selection = Vec::with_capacity(side.selection.len());
        for (binding, (field, literals)) in side.selection.iter().enumerate() {
            let Some(field_id) = self.field(statement, rel_idx, field) else {
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
                    LiteralSet::One(self.literal(at(0), rel_idx, field, literal))
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
                        .map(|(index, literal)| self.literal(at(index), rel_idx, field, literal))
                        .collect(),
                ),
            };
            selection.push((field_id, set));
        }
        (self.issues.is_empty()).then(|| Side {
            relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
            projection: projection.into_boxed_slice(),
            selection: selection.into_boxed_slice(),
        })
    }

    /// The canonical-utterance ban table over window spellings — the
    /// macro's `admit_window`, as data: survivors lower to the
    /// descriptor's `(lo, hi)`; every banned spelling is an issue whose
    /// `Display` names the canonical form.
    fn window(&mut self, statement: usize, window: WindowSpec) -> (u64, Option<u64>) {
        match window {
            WindowSpec::Exact(n) => (n, Some(n)),
            WindowSpec::Range { lo, hi } if hi < lo => {
                self.issues
                    .push(SpecIssue::WindowInverted { statement, lo, hi });
                (lo, None)
            }
            WindowSpec::Range { lo: 0, hi: 0 } => {
                self.issues
                    .push(SpecIssue::WindowExclusionRespelled { statement });
                (0, Some(0))
            }
            WindowSpec::Range { lo, hi } if lo == hi => {
                self.issues.push(SpecIssue::WindowExactRespelled {
                    statement,
                    count: lo,
                });
                (lo, Some(hi))
            }
            WindowSpec::Range { lo, hi } => (lo, Some(hi)),
            WindowSpec::Floor(0) => {
                self.issues.push(SpecIssue::WindowVacuous { statement });
                (0, None)
            }
            WindowSpec::Floor(1) => {
                self.issues
                    .push(SpecIssue::WindowContainmentRespelled { statement });
                (1, None)
            }
            WindowSpec::Floor(lo) => (lo, None),
        }
    }
}

impl SchemaSpec {
    /// Lowers the spec to the [`SchemaDescriptor`] the engine takes —
    /// exactly what `schema!` expansion does: name→id resolution
    /// (declaration order mints every id), handle resolution through
    /// field newtypes, `==` lowering to two adjacent containments
    /// (`source <= target` first), and the canonical-utterance ban table
    /// over window spellings and literal sets. Nothing else is judged
    /// here: semantic validation stays at the engine's
    /// `SchemaDescriptor::validate` inside `Db::create` / `Db::open`
    /// (the typed `SchemaError`), the same two-boundary split the macro
    /// observes.
    ///
    /// # Errors
    ///
    /// [`SchemaSpecError`] carrying EVERY unresolvable name, banned
    /// spelling, and over-wide extension row (the one shape lowering
    /// cannot represent — see [`SpecIssue::RowArityExcess`]), in spec
    /// order — never just the first.
    ///
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space
    /// (`u32`/`u16`) — the [`SchemaDescriptor::materialized_statements`]
    /// precedent; the declaration boundary (the engine's
    /// `SchemaError::RelationTooManyColumns` /
    /// `SchemaError::TooManyStatements`) is where such counts are
    /// rejected typed.
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
        for (idx, relation) in self.relations.iter().enumerate() {
            // The option is the kind: a closed relation carries its
            // handle newtype by construction (R7), so entering the
            // namespace is plain iteration — no silent skip stands in
            // for a typed issue.
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
                        value_type: field.value_type.clone(),
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
                            // The zip below drops any literal past the
                            // declared columns — unrepresentable, so it
                            // is an issue, not a truncation (the
                            // short-row case survives lowering and is
                            // the engine validator's arity check).
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
                                        resolver.literal(at, rel_idx, &field.name, literal)
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
                    let Some(rel_idx) = resolver.relation(index, relation) else {
                        continue;
                    };
                    let mut fields = Vec::with_capacity(projection.len());
                    for field in projection {
                        if let Some(id) = resolver.field(index, rel_idx, field) {
                            fields.push(id);
                        }
                    }
                    statements.push(StatementDescriptor::Functionality {
                        relation: RelationId(
                            u32::try_from(rel_idx).expect("relation count fits u32"),
                        ),
                        projection: fields.into_boxed_slice(),
                    });
                }
                StatementSpec::Containment {
                    source,
                    target,
                    bidirectional,
                } => {
                    resolver.coherent(index, source, target);
                    let source_side = resolver.side(index, StatementSide::Source, source);
                    let target_side = resolver.side(index, StatementSide::Target, target);
                    let (Some(source), Some(target)) = (source_side, target_side) else {
                        continue;
                    };
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
                StatementSpec::Cardinality {
                    target,
                    window,
                    source,
                } => {
                    resolver.coherent(index, source, target);
                    let (lo, hi) = resolver.window(index, *window);
                    let source_side = resolver.side(index, StatementSide::Source, source);
                    let target_side = resolver.side(index, StatementSide::Target, target);
                    let (Some(source), Some(target)) = (source_side, target_side) else {
                        continue;
                    };
                    statements.push(StatementDescriptor::Cardinality {
                        source,
                        lo,
                        hi,
                        target,
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
