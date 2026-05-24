use bumbledb_core::schema::ValueType;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    relation: String,
    values: Vec<(String, Value)>,
}

impl Fact {
    pub fn new(
        relation: impl Into<String>,
        values: impl IntoIterator<Item = (impl Into<String>, Value)>,
    ) -> Self {
        let mut values = values
            .into_iter()
            .fold(Vec::new(), |mut out, (field, value)| {
                let field = field.into();
                if let Some((_, existing)) = out.iter_mut().find(|(name, _)| *name == field) {
                    *existing = value;
                } else {
                    out.push((field, value));
                }
                out
            });
        values.sort_by(|left, right| left.0.cmp(&right.0));
        Self {
            relation: relation.into(),
            values,
        }
    }

    pub fn relation(&self) -> &str {
        &self.relation
    }

    pub fn value(&self, field: &str) -> Option<&Value> {
        self.values
            .iter()
            .find_map(|(name, value)| (name == field).then_some(value))
    }

    pub fn values(&self) -> &[(String, Value)] {
        &self.values
    }
}

pub(crate) trait FactView {
    fn relation(&self) -> &str;
    fn value_ref(&self, field: &str) -> Option<ValueRef<'_>>;
    fn for_each_field(&self, f: impl FnMut(&str));
}

impl FactView for Fact {
    fn relation(&self) -> &str {
        self.relation()
    }

    fn value_ref(&self, field: &str) -> Option<ValueRef<'_>> {
        self.value(field).map(ValueRef::from)
    }

    fn for_each_field(&self, mut f: impl FnMut(&str)) {
        for (field, _) in &self.values {
            f(field);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactRef<'a> {
    relation: &'a str,
    values: &'a [(&'a str, ValueRef<'a>)],
}

impl<'a> FactRef<'a> {
    pub fn new(relation: &'a str, values: &'a [(&'a str, ValueRef<'a>)]) -> Self {
        Self { relation, values }
    }
}

impl FactView for FactRef<'_> {
    fn relation(&self) -> &str {
        self.relation
    }

    fn value_ref(&self, field: &str) -> Option<ValueRef<'_>> {
        self.values
            .iter()
            .find_map(|(name, value)| (*name == field).then_some(*value))
    }

    fn for_each_field(&self, mut f: impl FnMut(&str)) {
        for (field, _) in self.values {
            f(field);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    Serial(u64),
    Enum(u8),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueRef<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    Serial(u64),
    Enum(u8),
    String(&'a str),
    Bytes(&'a [u8]),
}

impl<'a> From<&'a Value> for ValueRef<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Bool(value) => Self::Bool(*value),
            Value::U64(value) => Self::U64(*value),
            Value::I64(value) => Self::I64(*value),
            Value::Serial(value) => Self::Serial(*value),
            Value::Enum(value) => Self::Enum(*value),
            Value::String(value) => Self::String(value),
            Value::Bytes(value) => Self::Bytes(value),
        }
    }
}

impl ValueRef<'_> {
    pub fn matches_type(self, value_type: &ValueType) -> bool {
        matches!(
            (self, value_type),
            (ValueRef::Bool(_), ValueType::Bool)
                | (ValueRef::U64(_), ValueType::U64)
                | (ValueRef::I64(_), ValueType::I64)
                | (ValueRef::Serial(_), ValueType::Serial { .. })
                | (ValueRef::Enum(_), ValueType::Enum { .. })
                | (ValueRef::String(_), ValueType::String)
                | (ValueRef::Bytes(_), ValueType::Bytes)
        )
    }
}

impl Value {
    pub fn matches_type(&self, value_type: &ValueType) -> bool {
        ValueRef::from(self).matches_type(value_type)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    AlreadyPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    Absent,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputBindings {
    values: Vec<(String, Value)>,
}

impl InputBindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_values(values: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        let mut values = values
            .into_iter()
            .fold(Vec::new(), |mut out, (name, value)| {
                let name = name.into();
                if let Some((_, existing)) = out.iter_mut().find(|(existing, _)| *existing == name)
                {
                    *existing = value;
                } else {
                    out.push((name, value));
                }
                out
            });
        values.sort_by(|left, right| left.0.cmp(&right.0));
        Self { values }
    }

    pub fn value(&self, name: &str) -> Option<&Value> {
        self.values
            .iter()
            .find_map(|(candidate, value)| (candidate == name).then_some(value))
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultColumn {
    Variable(String),
}

pub type ResultFact = Vec<Value>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryResultSet {
    pub columns: Vec<ResultColumn>,
    pub facts: Vec<ResultFact>,
}

impl QueryResultSet {
    pub fn new(columns: Vec<ResultColumn>, mut facts: Vec<ResultFact>) -> Self {
        facts.sort();
        facts.dedup();
        Self { columns, facts }
    }

    pub(crate) fn from_canonical(columns: Vec<ResultColumn>, facts: Vec<ResultFact>) -> Self {
        Self { columns, facts }
    }

    pub fn cardinality(&self) -> usize {
        self.facts.len()
    }
}
