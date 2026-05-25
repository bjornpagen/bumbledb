use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::rc::Rc;

use bumbledb_core::encoding::{InternId, encode_enum, encode_intern_id, encode_u64};

use super::{
    EncodedTuple, EncodedTupleRef, GhtSource, KeyCountEstimate, TupleBatch, TupleCursor,
    TupleError, TupleField, TupleSchema,
};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::query::model::AtomOccurrenceId;

#[test]
fn encoded_tuple_single_field_equality_and_hash() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![TupleField::new(0, None, 8)?]);
    let tuple_a = EncodedTuple::new(&schema, encode_u64(10).to_vec())?;
    let tuple_b = EncodedTuple::new(&schema, encode_u64(10).to_vec())?;

    assert_eq!(tuple_a, tuple_b);
    assert_eq!(tuple_a.as_ref(), tuple_b.as_ref());
    Ok(())
}

#[test]
fn encoded_tuple_multi_field_equality_and_order() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![
        TupleField::new(0, None, 8)?,
        TupleField::new(1, None, 1)?,
    ]);
    let mut bytes = encode_u64(7).to_vec();
    bytes.extend_from_slice(&encode_enum(2));
    let tuple = EncodedTuple::new(&schema, bytes.clone())?;

    assert_eq!(tuple.bytes(), bytes.as_slice());
    assert_eq!(schema.vars(), vec![0, 1]);
    Ok(())
}

#[test]
fn encoded_tuple_supports_mixed_width_values() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![
        TupleField::new(0, None, 1)?,
        TupleField::new(1, None, 8)?,
        TupleField::new(2, None, 8)?,
        TupleField::new(3, None, 8)?,
        TupleField::new(4, None, 16)?,
    ]);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&encode_enum(3));
    bytes.extend_from_slice(&encode_u64(9));
    bytes.extend_from_slice(&encode_intern_id(InternId(1)));
    bytes.extend_from_slice(&encode_intern_id(InternId(2)));
    bytes.extend_from_slice(&[4; 16]);

    assert_eq!(EncodedTuple::new(&schema, bytes)?.bytes().len(), 41);
    Ok(())
}

#[test]
fn encoded_tuple_rejects_width_mismatch() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![TupleField::new(0, None, 8)?]);

    assert!(matches!(
        EncodedTuple::new(&schema, vec![1]),
        Err(TupleError::TupleWidthMismatch { .. })
    ));
    assert!(matches!(
        TupleField::new(0, None, 4),
        Err(TupleError::UnsupportedWidth { .. })
    ));
    Ok(())
}

#[test]
fn encoded_tuple_builds_from_bindings() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![TupleField::new(1, None, 8)?]);
    let bindings = BTreeMap::from([(1, encode_u64(42).to_vec())]);

    assert_eq!(
        schema.tuple_from_bindings(&bindings)?.bytes(),
        &encode_u64(42)
    );
    Ok(())
}

#[test]
fn encoded_tuple_builds_from_base_image_offset() -> Result<(), TupleError> {
    let schema = TupleSchema::new(vec![
        TupleField::new(0, Some(0), 8)?,
        TupleField::new(1, Some(1), 1)?,
    ]);
    let image = base_image();

    let tuple = schema.tuple_from_base_offset(&image, 1)?;

    let mut expected = encode_u64(2).to_vec();
    expected.extend_from_slice(&encode_enum(8));
    assert_eq!(tuple.bytes(), expected.as_slice());
    Ok(())
}

#[test]
fn ght_mock_iterates_and_gets_tuple_children() -> Result<(), TupleError> {
    let key = EncodedTuple::new(
        &TupleSchema::new(vec![TupleField::new(0, None, 8)?]),
        encode_u64(1).to_vec(),
    )?;
    let child = MockGht::leaf(vec![1]);
    let source = MockGht {
        atom: Some(AtomOccurrenceId(0)),
        vars: vec![0],
        keys: vec![key.clone()],
        children: BTreeMap::from([(key.clone(), child)]),
        count: KeyCountEstimate::Exact(1),
    };

    assert_eq!(collect_tuples(&source), vec![key.clone()]);
    assert_eq!(
        source
            .fill_batch(&mut TupleCursor::default(), 10)
            .iter()
            .map(EncodedTupleRef::to_owned_tuple)
            .collect::<Vec<_>>(),
        vec![key.clone()]
    );
    assert!(source.get(key.as_ref()).is_some());
    assert_eq!(source.key_count(), KeyCountEstimate::Exact(1));
    Ok(())
}

#[derive(Clone)]
struct MockGht {
    atom: Option<AtomOccurrenceId>,
    vars: Vec<usize>,
    keys: Vec<EncodedTuple>,
    children: BTreeMap<EncodedTuple, MockGht>,
    count: KeyCountEstimate,
}

impl MockGht {
    fn leaf(vars: Vec<usize>) -> Self {
        Self {
            atom: None,
            vars,
            keys: Vec::new(),
            children: BTreeMap::new(),
            count: KeyCountEstimate::Estimate(0),
        }
    }
}

impl GhtSource for MockGht {
    type Child<'a> = &'a MockGht;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        self.atom
    }

    fn vars(&self) -> &[usize] {
        &self.vars
    }

    fn try_for_each_tuple<E, F>(&self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        for key in &self.keys {
            if f(key.as_ref())?.is_break() {
                break;
            }
        }
        Ok(())
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        let batch_size = batch_size.max(1);
        let mut batch = TupleBatch::new();
        while cursor.position < self.keys.len() && batch.len() < batch_size {
            let _ = batch.push(self.keys[cursor.position].bytes());
            cursor.position += 1;
        }
        batch.exhausted = cursor.position >= self.keys.len();
        batch
    }

    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>> {
        self.children.get(tuple.bytes())
    }

    fn key_count(&self) -> KeyCountEstimate {
        self.count
    }
}

impl GhtSource for &MockGht {
    type Child<'a>
        = &'a MockGht
    where
        Self: 'a;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        (*self).atom()
    }

    fn vars(&self) -> &[usize] {
        (*self).vars()
    }

    fn try_for_each_tuple<E, F>(&self, f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        (*self).try_for_each_tuple(f)
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        (*self).fill_batch(cursor, batch_size)
    }

    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>> {
        (*self).get(tuple)
    }

    fn key_count(&self) -> KeyCountEstimate {
        (*self).key_count()
    }
}

fn collect_tuples(source: &impl GhtSource) -> Vec<EncodedTuple> {
    let mut tuples = Vec::new();
    let result = source.try_for_each_tuple::<(), _>(|tuple| {
        tuples.push(tuple.to_owned_tuple());
        Ok(ControlFlow::Continue(()))
    });
    assert!(result.is_ok());
    tuples
}

fn base_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 0,
        name: "R".to_owned(),
        row_handles: Rc::new(Vec::new()),
        columns: BTreeMap::from([(0, column_u64(0, [1, 2])), (1, column_enum(1, [7, 8]))]),
        stats: RelationStats { row_count: 2 },
    }
}

fn column_u64<const N: usize>(field_id: usize, values: [u64; N]) -> ColumnImage {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&encode_u64(value));
    }
    ColumnImage {
        field_id,
        width: 8,
        values: Rc::new(bytes),
        row_offsets: None,
    }
}

fn column_enum<const N: usize>(field_id: usize, values: [u8; N]) -> ColumnImage {
    let mut bytes = Vec::with_capacity(values.len());
    for value in values {
        bytes.extend_from_slice(&encode_enum(value));
    }
    ColumnImage {
        field_id,
        width: 1,
        values: Rc::new(bytes),
        row_offsets: None,
    }
}
