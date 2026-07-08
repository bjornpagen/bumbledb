use super::{graph, joins, olap, points, Scenario};

/// The registry, in report order.
#[must_use]
pub fn all() -> Vec<Scenario> {
    vec![
        joins::scenario(),
        graph::scenario(),
        olap::scenario(),
        points::scenario(),
    ]
}
