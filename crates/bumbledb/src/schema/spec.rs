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
//!   resolution (relations, fields, closed-relation handles) and the
//!   canonical-utterance ban table over window spellings and literal
//!   sets — surfacing every failure as the typed [`SchemaSpecError`],
//!   which enumerates ALL unresolvable names and banned spellings rather
//!   than stopping at the first (a foreign host gets one round trip).
//! - Everything semantic beyond names stays where the macro defers it:
//!   [`SchemaDescriptor::validate`] inside [`crate::Db::create`] /
//!   [`crate::Db::open`], as the typed [`crate::error::SchemaError`].
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

/// One relation. `extension: Some(rows)` declares it **closed** (the
/// option is the kind, mirroring [`RelationDescriptor::extension`]); a
/// closed relation's `fields` are its declared intrinsic columns only —
/// the synthetic (`id`, `u64`) handle field is materialized by schema
/// validation, and statement field names address the sealed shape (`id`
/// resolves to [`FieldId`] 0, declared columns shift by one), exactly as
/// the macro resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSpec {
    pub name: Box<str>,
    /// The handle newtype name of a closed relation (the macro's
    /// mandatory `as NewType`) — host-side nominal vocabulary, never a
    /// fingerprint input: it exists so [`LiteralSpec::Handle`] literals
    /// can resolve through a referencing field's [`FieldSpec::newtype`],
    /// and it is dropped at lowering (the descriptor never carries
    /// names of host types). Meaningless on an ordinary relation.
    pub newtype: Option<Box<str>>,
    pub fields: Vec<FieldSpec>,
    /// A closed relation's ground axioms in declaration order (row id =
    /// index); `None` = ordinary.
    pub extension: Option<Vec<RowSpec>>,
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
    /// `fresh` — the mint mark, legal on `u64` (validated at
    /// [`SchemaDescriptor::validate`], as the macro defers it).
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

/// One resolution failure of [`SchemaSpec::descriptor`] — a name the spec
/// used that its own declarations never introduce, or a banned spelling
/// of the canonical-utterance law. `statement` payloads index
/// [`SchemaSpec::statements`] (the spec's own order, before `==`
/// lowering); extension-row payloads name the relation and row.
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
        relation: Box<str>,
        field: Box<str>,
        handle: Box<str>,
    },
    /// A handle the named closed relation's extension never declares.
    UnknownHandle { closed: Box<str>, handle: Box<str> },
    /// Two closed relations claim one handle newtype — a handle newtype
    /// names exactly one closed relation.
    DuplicateHandleNewtype {
        newtype: Box<str>,
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
}

impl std::fmt::Display for SpecIssue {
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
            } => write!(
                f,
                "`{relation}.{field}` is not a closed-relation reference — the handle \
                 literal `{handle}` is legal only on a field whose newtype is a closed \
                 relation's handle newtype"
            ),
            Self::UnknownHandle { closed, handle } => {
                write!(f, "closed relation `{closed}` has no handle `{handle}`")
            }
            Self::DuplicateHandleNewtype {
                newtype,
                first,
                second,
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
        }
    }
}

/// [`SchemaSpec::descriptor`]'s typed failure: the COMPLETE issue list —
/// every unresolvable name and every banned spelling, in spec order —
/// never the first offender alone (the [`crate::error::Violations`]
/// precedent: a foreign host repairs its whole spec in one round trip).
/// Sealed nonempty by the one construction site.
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
        let closed = relation.extension.is_some();
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
        if relation.extension.is_some() && name == "id" {
            return relation.newtype.as_deref();
        }
        relation
            .fields
            .iter()
            .find(|f| &*f.name == name)
            .and_then(|f| f.newtype.as_deref())
    }

    /// One literal at its field position — a [`LiteralSpec::Handle`]
    /// resolves through the field's newtype to its closed relation's
    /// declaration-order row id, the macro's own resolution. On an issue
    /// the placeholder `Value::U64(0)` stands in; placeholders never
    /// escape (a nonempty issue list fails the whole construction).
    fn literal(&mut self, rel_idx: usize, field: &str, literal: &LiteralSpec) -> Value {
        match literal {
            LiteralSpec::Value(value) => value.clone(),
            LiteralSpec::Handle(handle) => {
                let owner = self
                    .field_newtype(rel_idx, field)
                    .and_then(|newtype| self.handles.get(newtype).copied());
                let Some(owner) = owner else {
                    self.issues.push(SpecIssue::NotAHandleField {
                        relation: self.spec.relations[rel_idx].name.clone(),
                        field: field.into(),
                        handle: handle.clone(),
                    });
                    return Value::U64(0);
                };
                let rows = self.spec.relations[owner]
                    .extension
                    .as_ref()
                    .expect("the handle namespace holds closed relations only");
                let Some(row) = rows.iter().position(|row| row.handle == *handle) else {
                    self.issues.push(SpecIssue::UnknownHandle {
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
    fn side(&mut self, statement: usize, side: &SideSpec) -> Option<Side> {
        let rel_idx = self.relation(statement, &side.relation)?;
        let mut projection = Vec::with_capacity(side.projection.len());
        for field in &side.projection {
            if let Some(id) = self.field(statement, rel_idx, field) {
                projection.push(id);
            }
        }
        let mut selection = Vec::with_capacity(side.selection.len());
        for (field, literals) in &side.selection {
            let Some(field_id) = self.field(statement, rel_idx, field) else {
                continue;
            };
            let set = match literals {
                LiteralSetSpec::One(literal) => {
                    LiteralSet::One(self.literal(rel_idx, field, literal))
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
                        .map(|literal| self.literal(rel_idx, field, literal))
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
    /// here: semantic validation stays at [`SchemaDescriptor::validate`]
    /// inside [`crate::Db::create`] / [`crate::Db::open`] (the typed
    /// [`crate::error::SchemaError`]), the same two-boundary split the
    /// macro observes.
    ///
    /// # Errors
    ///
    /// [`SchemaSpecError`] carrying EVERY unresolvable name and banned
    /// spelling, in spec order — never just the first.
    ///
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space
    /// (`u32`/`u16`) — the [`SchemaDescriptor::materialized_statements`]
    /// precedent; the declaration boundary
    /// ([`crate::error::SchemaError::RelationTooManyColumns`] /
    /// [`crate::error::SchemaError::TooManyStatements`]) is where such
    /// counts are rejected typed.
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
            if relation.extension.is_none() {
                continue;
            }
            let Some(newtype) = relation.newtype.as_deref() else {
                continue;
            };
            if let Some(first) = resolver.handles.insert(newtype, idx) {
                resolver.issues.push(SpecIssue::DuplicateHandleNewtype {
                    newtype: newtype.into(),
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
                extension: relation.extension.as_ref().map(|rows| {
                    rows.iter()
                        .map(|row| Row {
                            handle: row.handle.clone(),
                            values: row
                                .values
                                .iter()
                                .zip(&relation.fields)
                                .map(|(literal, field)| {
                                    resolver.literal(rel_idx, &field.name, literal)
                                })
                                .collect(),
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
                    let source_side = resolver.side(index, source);
                    let target_side = resolver.side(index, target);
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
                    let (lo, hi) = resolver.window(index, *window);
                    let source_side = resolver.side(index, source);
                    let target_side = resolver.side(index, target);
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
