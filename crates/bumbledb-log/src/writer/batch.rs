//! Leases-adjacent recording: the host writes into a `Batch` and the
//! driver never re-invokes the body. Reservation sugar is an ordinary
//! insert whose shape was derived at open.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Mutex;

use bumbledb::Value;
use bumbledb::schema::{
    FieldId, RelationId, SchemaDescriptor, StatementDescriptor, StatementId, ValueType, Weight,
};

use crate::codec::{Op, OpKind};
use crate::lease::{Leased, Leases};
use crate::replica::Fault;
use crate::store::ObjectStore;

use super::{Error, Result, lock};

/// The recorded transaction: typed inserts and deletes, id draws from
/// the lease, and the reservation sugar. Recording is pure — the engine
/// judges at apply, and the driver never re-invokes the body.
pub struct Batch<'w, S: ObjectStore> {
    pub(crate) ops: Vec<Op>,
    pub(crate) store: &'w S,
    pub(crate) prefix: &'w str,
    pub(crate) leases: &'w Mutex<Leases>,
    pub(crate) maps: &'w SchemaMaps,
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

    /// Draws `count` fresh ids for `(relation, field)` from the lease;
    /// the resulting inserts carry the concrete values, and id
    /// reservations never appear in the log.
    pub fn reserve(
        &mut self,
        relation: RelationId,
        field: FieldId,
        count: u64,
    ) -> Result<Range<u64>> {
        let drawn = lock(self.leases)
            .draw(self.store, self.prefix, relation, field, count)
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        match drawn {
            Leased::Range(range) => Ok(range),
            Leased::Refused(refusal) => Err(Error::Lease(refusal)),
        }
    }

    /// Sugar over an ordinary insert into the declared reservation
    /// relation of `statement`: parent projection values, `units` into
    /// the weight field, `expiry` into the one leftover u64 field.
    /// Nothing here is special-cased — the row rides the log like any
    /// other, and the spend is an ordinary commit.
    pub fn reserve_capacity(
        &mut self,
        statement: StatementId,
        parent: &[Value],
        units: u64,
        expiry: u64,
    ) -> Result<()> {
        let Some(shape) = self.maps.reservations.get(&statement) else {
            return Err(Error::ReservationShape { statement });
        };
        if parent.len() != shape.projection.len() {
            return Err(Error::ReservationShape { statement });
        }
        let layout = &self.maps.layouts[&shape.relation];
        let mut row: Vec<Option<Value>> = vec![None; layout.len()];
        for (position, field) in shape.projection.iter().enumerate() {
            row[usize::from(*field)] = Some(parent[position].clone());
        }
        row[usize::from(shape.weight_field)] = Some(Value::U64(units));
        row[usize::from(shape.expiry_field)] = Some(Value::U64(expiry));
        let row: Box<[Value]> = row
            .into_iter()
            .map(|value| value.expect("the reservation shape covers every field"))
            .collect();
        self.insert(shape.relation, [row]);
        Ok(())
    }
}

/// One capacity statement's reservation shape, derived at open: the
/// source relation whose layout is exactly parent projection + weight
/// field + one leftover u64 field (the expiry).
pub(crate) struct ReservationShape {
    pub(crate) relation: RelationId,
    pub(crate) projection: Box<[u16]>,
    pub(crate) weight_field: u16,
    pub(crate) expiry_field: u16,
}

/// Descriptor-derived views the writer reads outside the codec: raw
/// layouts for the drain's packing measure, and the reservation shapes.
pub(crate) struct SchemaMaps {
    pub(crate) layouts: BTreeMap<RelationId, Box<[ValueType]>>,
    pub(crate) reservations: BTreeMap<StatementId, ReservationShape>,
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

    let mut reservations: BTreeMap<StatementId, ReservationShape> = BTreeMap::new();
    let fields =
        |projection: &[FieldId]| -> Box<[u16]> { projection.iter().map(|field| field.0).collect() };
    for (index, statement) in descriptor.materialized_statements().iter().enumerate() {
        let id = StatementId(u16::try_from(index).expect("statement count fits u16"));
        let StatementDescriptor::Capacity { weight, source, .. } = statement else {
            continue;
        };
        let Weight::Field(weight_field) = weight else {
            continue;
        };
        let Some(layout) = layouts.get(&source.relation) else {
            continue;
        };
        let projection = fields(&source.projection);
        let named: Vec<u16> = projection.iter().copied().chain([weight_field.0]).collect();
        let leftovers: Vec<u16> = (0..u16::try_from(layout.len()).expect("field count"))
            .filter(|field| !named.contains(field))
            .collect();
        if let [expiry_field] = leftovers.as_slice()
            && layout[usize::from(*expiry_field)] == ValueType::U64
        {
            reservations.insert(
                id,
                ReservationShape {
                    relation: source.relation,
                    projection,
                    weight_field: weight_field.0,
                    expiry_field: *expiry_field,
                },
            );
        }
    }
    SchemaMaps {
        layouts,
        reservations,
    }
}
