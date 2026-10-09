//! Grouped judgment state: determinant membership, capacity group totals,
//! pointwise span tables and coverage runs, held in exact in-memory ordered
//! maps keyed by injective value encodings. Ordered iteration is byte order,
//! which the fixed-width span and run keys rely on.

use std::collections::BTreeMap;
use std::ops::Bound;

use crate::canonical::{DecodeScratch, RowError};
use crate::ir::Value;
use crate::schema::{FieldDescriptor, FieldId};
use crate::work::WorkContext;

use super::JudgeError;

pub(super) const FLAG_OK: u8 = 0;
pub(super) const FLAG_RAY: u8 = 1;
pub(super) const FLAG_OVERFLOW: u8 = 2;

/// One scalar projection workspace, reused between projections.
pub(super) struct ScalarKeyScratch {
    values: Vec<Value>,
    encoded: Vec<u8>,
}

impl ScalarKeyScratch {
    pub(super) fn new(fields: usize) -> Self {
        Self {
            values: Vec::with_capacity(fields),
            encoded: Vec::new(),
        }
    }

    pub(super) fn project(&mut self, row: &[Value], fields: &[usize]) {
        self.values.clear();
        self.values.extend(fields.iter().map(|&at| row[at].clone()));
    }

    pub(super) fn values(&self) -> &[Value] {
        &self.values
    }

    pub(super) fn key(&self) -> &[u8] {
        &self.encoded
    }

    pub(super) fn encode(&mut self) -> &[u8] {
        self.encoded.clear();
        for value in &self.values {
            encode_value(value, &mut self.encoded);
        }
        &self.encoded
    }

    /// Encode borrowed logical fields without cloning their payloads.
    pub(super) fn encode_projection(&mut self, row: &[Value], fields: &[FieldId]) -> &[u8] {
        self.encoded.clear();
        for field in fields {
            encode_value(&row[usize::from(field.0)], &mut self.encoded);
        }
        &self.encoded
    }
}

/// One exact grouped map for a single statement's judgment. Keys are exact
/// encoded value tuples or fixed-width words; no hash decides identity.
#[derive(Default)]
pub(super) struct GroupedMap {
    entries: BTreeMap<Box<[u8]>, Box<[u8]>>,
}

impl GroupedMap {
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Insert-if-absent set membership; true iff new.
    pub(super) fn insert_if_absent(&mut self, key: &[u8]) -> bool {
        if self.entries.contains_key(key) {
            return false;
        }
        self.entries.insert(key.into(), Box::default());
        true
    }

    pub(super) fn put(&mut self, key: &[u8], value: &[u8]) {
        self.entries.insert(key.into(), value.into());
    }

    pub(super) fn contains(&self, key: &[u8]) -> bool {
        self.entries.contains_key(key)
    }

    /// The dense token of `key`, minted in first-encounter order.
    pub(super) fn token_of(&mut self, key: &[u8]) -> u64 {
        if let Some(token) = self.lookup_token(key) {
            return token;
        }
        let token = self.entries.len() as u64;
        self.put(key, &token.to_be_bytes());
        token
    }

    pub(super) fn lookup_token(&self, key: &[u8]) -> Option<u64> {
        self.entries.get(key).map(|value| word(value))
    }

    /// One capacity group's widened running total and its sticky
    /// first-failure flag; `(0, FLAG_OK)` for an unseen group.
    pub(super) fn group_total(&self, key: &[u8]) -> (u128, u8) {
        self.entries.get(key).map_or((0, FLAG_OK), |value| {
            let (total, flag) = value.split_at(16);
            (
                u128::from_be_bytes(total.try_into().expect("16-byte total")),
                flag[0],
            )
        })
    }

    pub(super) fn put_group_total(&mut self, key: &[u8], total: u128, flag: u8) {
        let mut value = [0u8; 17];
        value[..16].copy_from_slice(&total.to_be_bytes());
        value[16] = flag;
        self.put(key, &value);
    }

    /// Ordered walk over every (key, value); `false` stops early.
    pub(super) fn for_each<E>(
        &self,
        mut visit: impl FnMut(&[u8], &[u8]) -> Result<bool, JudgeError<E>>,
    ) -> Result<(), JudgeError<E>> {
        for (key, value) in &self.entries {
            if !visit(key, value)? {
                break;
            }
        }
        Ok(())
    }

    /// Walk determinant keys decoded back to logical values with one reusable
    /// decode workspace. `false` stops early.
    pub(super) fn for_each_determinant<E>(
        &self,
        fields: &[FieldDescriptor],
        projection: &[FieldId],
        work: &WorkContext,
        mut visit: impl FnMut(&[u8], &[Value]) -> Result<bool, JudgeError<E>>,
    ) -> Result<(), JudgeError<E>> {
        let mut decoded = DecodeScratch::new(work);
        for key in self.entries.keys() {
            let keep = decoded
                .with_decoded_payload(
                    projection.iter().map(|field| &fields[usize::from(field.0)]),
                    key,
                    |values| Ok::<_, RowError>(visit(key, values)),
                )
                .map_err(|error| match error {
                    RowError::Work(work) => JudgeError::Work(work),
                    other => {
                        unreachable!("judge keys follow the canonical payload encoder: {other:?}")
                    }
                })??;
            if !keep {
                break;
            }
        }
        Ok(())
    }

    /// The last entry at or before `bound`, borrowed.
    pub(super) fn last_at_or_before(&self, bound: &[u8]) -> Option<(&[u8], &[u8])> {
        self.entries
            .range::<[u8], _>((Bound::Unbounded, Bound::Included(bound)))
            .next_back()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
    }
}

fn word(value: &[u8]) -> u64 {
    u64::from_be_bytes(value.try_into().expect("8-byte grouped word"))
}

/// Append one value's exact, injective, prefix-free byte image: the
/// canonical row payload rules, so byte equality of two encoded tuples is
/// canonical value equality.
pub(super) fn encode_value(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Bool(v) => out.extend_from_slice(&[0, u8::from(*v)]),
        Value::U64(v) => {
            out.push(1);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::I64(v) => {
            out.push(2);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::F64(v) => {
            out.push(3);
            out.extend_from_slice(&v.to_be_bytes());
        }
        Value::String(v) => {
            out.push(4);
            out.extend_from_slice(&(v.len() as u64).to_be_bytes());
            out.extend_from_slice(v.as_bytes());
        }
        Value::FixedBytes(v) => {
            out.push(5);
            out.extend_from_slice(&(v.len() as u64).to_be_bytes());
            out.extend_from_slice(v);
        }
        Value::IntervalU64(v) => {
            out.push(6);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
        Value::IntervalI64(v) => {
            out.push(7);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
        Value::Uuid(v) => {
            out.push(8);
            out.extend_from_slice(v.as_bytes());
        }
        Value::IntervalF64(v) => {
            out.push(9);
            out.extend_from_slice(&v.start().to_be_bytes());
            out.extend_from_slice(&v.end().to_be_bytes());
        }
    }
}
