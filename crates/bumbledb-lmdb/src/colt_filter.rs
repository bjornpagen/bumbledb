use crate::base_image::RelationBaseImage;
use crate::colt::KeyOwned;

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceFilterOp { Eq, NotEq, Lt, Lte, Gt, Gte }

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceFilter { Compare { field_id: usize, op: SourceFilterOp, value: KeyOwned }, False }

impl SourceFilter {
    pub(crate) fn field_id(&self) -> Option<usize> {
        match self {
            SourceFilter::Compare { field_id, .. } => Some(*field_id),
            SourceFilter::False => None,
        }
    }
}

pub(crate) fn source_filter_matches(
    base: &RelationBaseImage,
    offset: usize,
    filter: &SourceFilter,
) -> bool {
    match filter {
        SourceFilter::False => false,
        SourceFilter::Compare {
            field_id,
            op,
            value,
        } => base
            .columns
            .get(field_id)
            .and_then(|column| column.value_at(offset))
            .is_some_and(|candidate| compare_encoded(candidate, *op, value.bytes())),
    }
}

fn compare_encoded(candidate: &[u8], op: SourceFilterOp, value: &[u8]) -> bool {
    match op {
        SourceFilterOp::Eq => candidate == value,
        SourceFilterOp::NotEq => candidate != value,
        SourceFilterOp::Lt => candidate < value,
        SourceFilterOp::Lte => candidate <= value,
        SourceFilterOp::Gt => candidate > value,
        SourceFilterOp::Gte => candidate >= value,
    }
}
