//! Leases-adjacent recording: the host writes into a `Batch` and the
//! driver never re-invokes the body (10 §3).

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Mutex;

use bumbledb::Value;
use bumbledb::schema::{FieldId, RelationId, SchemaDescriptor, ValueType};

use crate::codec::{Op, OpKind};
use crate::lease::{Leased, Leases};
use crate::replica::Fault;
use crate::store::ObjectStore;

use super::{Error, Result, lock};

/// The recorded transaction: typed inserts and deletes, plus one
/// unsigned lease draw (`OverWidth | Exhausted | Drawn`). Recording is
/// pure — the engine judges at apply, and the driver never re-invokes
/// the body.
pub struct Batch<'w, S: ObjectStore> {
    pub(crate) ops: Vec<Op>,
    pub(crate) store: &'w S,
    pub(crate) prefix: &'w str,
    pub(crate) leases: &'w Mutex<Leases>,
}

impl<S: ObjectStore> Batch<'_, S> {
    pub fn insert<I: IntoIterator<Item = Box<[Value]>>>(&mut self, relation: RelationId, rows: I) {
        self.ops.push(Op {
            kind: OpKind::Insert,
            relation,
            rows: rows.into_iter().collect(),
        });
    }

    pub fn delete<I: IntoIterator<Item = Box<[Value]>>>(&mut self, relation: RelationId, rows: I) {
        self.ops.push(Op {
            kind: OpKind::Delete,
            relation,
            rows: rows.into_iter().collect(),
        });
    }

    /// One draw of `count` (unsigned) for `(relation, field)`:
    /// `OverWidth | Exhausted | Drawn`. The body is not re-run; the
    /// resulting inserts carry the concrete values, and id
    /// reservations never appear in the log.
    ///
    /// # Errors
    pub fn reserve(
        &mut self,
        relation: RelationId,
        field: FieldId,
        count: u64,
    ) -> Result<Range<u64>> {
        match lock(self.leases)
            .draw(self.store, self.prefix, relation, field, count)
            .map_err(|err| Error::Fault(Fault::Store(err)))?
        {
            Leased::Drawn { range, .. } => Ok(range),
            Leased::Refused(refusal) => Err(Error::Lease(refusal)),
        }
    }
}

/// Descriptor-derived view the writer reads outside the codec: raw
/// layouts for the drain's packing measure.
pub(crate) struct SchemaMaps {
    pub(crate) layouts: BTreeMap<RelationId, Box<[ValueType]>>,
}

pub(crate) fn schema_maps(descriptor: &SchemaDescriptor) -> SchemaMaps {
    let mut layouts: BTreeMap<RelationId, Box<[ValueType]>> = BTreeMap::new();
    for (index, relation) in descriptor.relations.iter().enumerate() {
        let id = RelationId(u32::try_from(index).expect("relation count fits u32"));
        layouts.insert(
            id,
            relation
                .fields
                .iter()
                .map(|field| field.value_type)
                .collect(),
        );
    }
    SchemaMaps { layouts }
}
