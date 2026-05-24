use std::collections::BTreeMap;

use bumbledb_core::encoding::{InternId, encode_enum, encode_intern_id, encode_u64};

use super::{EncodedTuple, GhtSource, KeyCountEstimate, TupleError, TupleField, TupleSchema};
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

    assert_eq!(source.iter(), vec![key.clone()]);
    assert!(source.get(&key).is_some());
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

    fn iter(&self) -> Vec<EncodedTuple> {
        self.keys.clone()
    }

    fn get(&self, tuple: &EncodedTuple) -> Option<Self::Child<'_>> {
        self.children.get(tuple)
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

    fn iter(&self) -> Vec<EncodedTuple> {
        (*self).iter()
    }

    fn get(&self, tuple: &EncodedTuple) -> Option<Self::Child<'_>> {
        (*self).get(tuple)
    }

    fn key_count(&self) -> KeyCountEstimate {
        (*self).key_count()
    }
}

fn base_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 0,
        name: "R".to_owned(),
        row_handles: Vec::new(),
        columns: BTreeMap::from([
            (
                0,
                ColumnImage {
                    field_id: 0,
                    field: "x".to_owned(),
                    values: vec![encode_u64(1).to_vec(), encode_u64(2).to_vec()],
                },
            ),
            (
                1,
                ColumnImage {
                    field_id: 1,
                    field: "e".to_owned(),
                    values: vec![encode_enum(7).to_vec(), encode_enum(8).to_vec()],
                },
            ),
        ]),
        stats: RelationStats { row_count: 2 },
    }
}
