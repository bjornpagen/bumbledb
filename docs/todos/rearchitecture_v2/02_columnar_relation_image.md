# 02: Columnar RelationImage

**Goal**
- Represent each relation as encoded columnar data with row-id addressability and zero logical row materialization in hot paths.

**RelationImage Type**
```rust
pub struct RelationImage {
    pub id: RelationId,
    pub name: String,
    pub row_count: usize,
    pub fields: Vec<FieldImage>,
    pub columns: Vec<ColumnImage>,
    pub sorted_indexes: Vec<SortedTrieIndex>,
    pub hash_indexes: Vec<HashTrieIndex>,
    pub stats: RelationStats,
}

pub struct FieldImage {
    pub id: FieldId,
    pub name: String,
    pub value_type: ValueType,
    pub width: usize,
}
```

**ColumnImage Type**
```rust
pub enum ColumnImage {
    Fixed8(FixedColumn<[u8; 8]>),
    Fixed16(FixedColumn<[u8; 16]>),
    Bool(FixedColumn<[u8; 1]>),
}

pub struct FixedColumn<T> {
    field: FieldId,
    values: Vec<T>,
}

impl<T: Copy> FixedColumn<T> {
    #[inline]
    pub fn get(&self, row: RowId) -> T {
        self.values[row.0 as usize]
    }
}
```

**Encoded References**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedRef<'a> {
    One(&'a [u8; 1]),
    Eight(&'a [u8; 8]),
    Sixteen(&'a [u8; 16]),
}

impl<'a> EncodedRef<'a> {
    #[inline]
    pub fn as_bytes(self) -> &'a [u8] {
        match self {
            EncodedRef::One(bytes) => &bytes[..],
            EncodedRef::Eight(bytes) => &bytes[..],
            EncodedRef::Sixteen(bytes) => &bytes[..],
        }
    }
}
```

**Relation Access**
```rust
impl RelationImage {
    #[inline]
    pub fn encoded<'a>(&'a self, row: RowId, field: FieldId) -> EncodedRef<'a> {
        match &self.columns[field.0 as usize] {
            ColumnImage::Bool(col) => EncodedRef::One(col.get_ref(row)),
            ColumnImage::Fixed8(col) => EncodedRef::Eight(col.get_ref(row)),
            ColumnImage::Fixed16(col) => EncodedRef::Sixteen(col.get_ref(row)),
        }
    }
}
```

**Row Sets**
```rust
#[derive(Clone, Copy, Debug)]
pub struct RowRange {
    pub start: RowId,
    pub end: RowId,
}

pub enum RowSetRef<'a> {
    Empty,
    One(RowId),
    Range(RowRange),
    Slice(&'a [RowId]),
}
```

**Build Path**
- Decode durable current rows only into encoded field bytes, not logical `Value`.
- Append each encoded field to the corresponding column vector.
- Assign dense `RowId` in relation-image order.
- Keep durable primary key to `RowId` mapping only if needed for exact lookup indexes.

**Column Demand Rules**
- All columns are available in QueryImage v1.
- Later stages may build partial column images for query-specific images.
- Query execution asks for fields by `FieldId`, not by name.

**Tests**
- Column widths match schema widths.
- Every encoded column value decodes back to original inserted value.
- String/bytes columns contain intern IDs, not raw strings/bytes.
- `RowId` order is stable for the same snapshot.
- Public row decode can be implemented by reading column values for a row.

**Passing Criteria**
- No `Row { BTreeMap<String, Value> }` construction occurs while building indexes from `RelationImage`.
- RelationImage has O(1) encoded value access by `(RowId, FieldId)`.
- RelationImage memory accounting reports encoded column bytes.
- Existing storage/query tests pass through unchanged public behavior.

**Non-Goals**
- Do not introduce compression yet.
- Do not add vectorized execution yet.
- Do not persist column chunks yet.
