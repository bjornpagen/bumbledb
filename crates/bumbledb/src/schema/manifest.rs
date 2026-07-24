//! The theory's manifest: name → id, as plain data
//! (`docs/architecture/70-api.md` § the manifest). The macro's id
//! constants give the *Rust* host its numbers at compile time
//! (`Calendar::BUSY`, `Calendar::BUSY_PERSON`); the manifest gives a
//! *foreign* host the same numbers as a runtime value — a plain Rust
//! struct straight off the descriptor, no serde, no derive machinery
//! (the dependency law: a downstream binding serializes it however it
//! likes; the engine never learns the wire format).

use super::{FieldId, RelationId, SchemaDescriptor, StatementId, StatementKind, ValueType};
use bumbledb_theory::Value;

/// Every name → id pairing of one theory, in declaration order — named
/// data, not ergonomics. A closed relation's handles ride as its
/// [`RowManifest`] list: the row id is the index, by the
/// declaration-order law. Closed relations carry their extension — the
/// vocabulary as data, so a foreign surface (render, future bindings)
/// sees every ground axiom without touching Rust. Statements ride in
/// materialized order with their canonical spellings, so a foreign host
/// can cite any statement id — a rejection's, a diagnostic's — without a
/// Rust renderer in reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// One entry per relation; `RelationId` = the index, stated
    /// explicitly on each entry so a reader never re-derives it.
    pub relations: Vec<RelationManifest>,
    /// One entry per MATERIALIZED statement (fresh auto-keys, closed
    /// auto-keys, then declared statements —
    /// [`SchemaDescriptor::materialized_statements`] owns the order);
    /// `StatementId` = the index, stated explicitly.
    pub statements: Vec<StatementManifest>,
}

/// One statement's identity, form tag, and canonical spelling
/// ([`super::render::render_declared`] — the one renderer, a bijection
/// on legal statements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementManifest {
    pub id: StatementId,
    pub kind: StatementKind,
    pub spelling: String,
}

/// One relation's names and ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationManifest {
    pub name: Box<str>,
    pub id: RelationId,
    /// One entry per field, in declaration order; `FieldId` = the index.
    pub fields: Vec<FieldManifest>,
    /// A closed relation's ground axioms in declaration order (`None` =
    /// ordinary): handle → id → intrinsic values, plain data off the
    /// descriptor.
    pub extension: Option<Vec<RowManifest>>,
}

/// One ground axiom as manifest data: the handle, its declaration-order
/// row id, and each intrinsic (column, value) pair in field-declaration
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowManifest {
    pub handle: Box<str>,
    pub id: u64,
    pub values: Vec<(Box<str>, Value)>,
}

/// One field's name, id, and structural type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldManifest {
    pub name: Box<str>,
    pub id: FieldId,
    pub value_type: ValueType,
}

/// The manifest rendering as an extension trait: [`SchemaDescriptor`] is
/// theory data (hosted in `bumbledb-theory`), and the manifest needs the
/// engine-side renderer, so the method hangs off it here.
pub trait ManifestDescriptor {
    /// Renders the manifest off the descriptor — the ids are the
    /// declaration-order indices, made explicit.
    fn manifest(&self) -> Manifest;
}

impl ManifestDescriptor for SchemaDescriptor {
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space (`u32`/`u16`)
    /// — impossible for a descriptor the declaration boundary admits.
    fn manifest(&self) -> Manifest {
        // Materialized once, `==` links once — every statement's spelling
        // reads the same list through the threaded renderer (per-entry
        // `render_declared` re-materialized the whole roster per
        // statement: O(n²) clones).
        let materialized = self.materialized_statements();
        let mirrors = super::validate::mirror_links(&materialized);
        Manifest {
            statements: materialized
                .iter()
                .enumerate()
                .map(|(idx, statement)| {
                    let id = StatementId(u16::try_from(idx).expect("statement count fits u16"));
                    StatementManifest {
                        id,
                        kind: statement.kind(),
                        spelling: super::render::render_materialized(
                            self,
                            &materialized,
                            &mirrors,
                            id,
                        ),
                    }
                })
                .collect(),
            relations: self
                .relations
                .iter()
                .enumerate()
                .map(|(rel_idx, relation)| {
                    // The extension table: handle → declaration-order id →
                    // (column, value) pairs — the vocabulary as data.
                    let extension = relation.extension.as_ref().map(|rows| {
                        rows.iter()
                            .enumerate()
                            .map(|(row_idx, row)| RowManifest {
                                handle: row.handle.clone(),
                                id: u64::try_from(row_idx).expect("row count fits u64"),
                                values: relation
                                    .fields
                                    .iter()
                                    .map(|field| field.name.clone())
                                    .zip(row.values.iter().cloned())
                                    .collect(),
                            })
                            .collect()
                    });
                    RelationManifest {
                        name: relation.name.clone(),
                        id: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
                        // The SEALED roster — synthetic (`id`, U64) first
                        // for a closed relation — through THE one owner of
                        // the synthetic-id law
                        // (`RelationDescriptor::sealed_fields`); the
                        // manifest reports the ids the sealed schema
                        // answers to (handle-name entries are the emission
                        // PRD's).
                        fields: relation
                            .sealed_fields()
                            .enumerate()
                            .map(|(field_idx, slot)| FieldManifest {
                                name: slot.name.into(),
                                id: FieldId(
                                    u16::try_from(field_idx).expect("field count fits u16"),
                                ),
                                value_type: slot.value_type.clone(),
                            })
                            .collect(),
                        extension,
                    }
                })
                .collect(),
        }
    }
}
