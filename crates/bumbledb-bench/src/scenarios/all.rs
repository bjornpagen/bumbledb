use super::{Scenario, graph, joins, olap, points, rings};

/// The registry, in report order.
#[must_use]
pub fn all() -> Vec<Scenario> {
    vec![
        joins::scenario(),
        graph::scenario(),
        olap::scenario(),
        points::scenario(),
        rings::scenario(),
    ]
}
