# Rust surface

The current proposal uses `event` / `Event`, with outcome vocabularies supplied
by ordinary relations and containment. The earlier parameterized kernel-field
sketch has been retired.

- [Schema and constructor surface](event-surface.md)
- [Query grammar and stages](query-algebra.md)
- [Complete Coup schema](coup/schema.rs)
- [Complete Coup query templates](coup/queries.rs)
- [Constructive algebra query templates](coup/algebra-queries.rs)
- [World faces, relational programs, and finite closure](world-relations.md)
- [Resident layout and owned values](representation.md)

These documents define the complete proposed surface. Native Event fields,
unmeasured values/bindings, field/full dependencies and the
[checked map host API](../docs/event-maps.md) are now implemented on the branch,
alongside [finite relational products and operators](../docs/event-relations.md),
[diagram inspection/reconstruction](../docs/event-inspection.md),
[finite information operators](../docs/event-information.md), and
[typed programs and finite fixed points](../docs/event-fixed-points.md).
The [indexed partition helper](../docs/event-partitions.md) now supplies finite
value/count rosters, grouping, refinement and conversion into checked readouts;
it does not introduce a parameterized observable database field.
The [action/strategy host API](../docs/event-actions.md) now supplies checked
controlled predecessors, inhabited uniform permissions and fully observed
strategies with retained rank/safety witnesses. It does not infer actor memory.
Constructive query heads, complete descriptor transport, measured sources and the remaining
operators still have the acceptance gates recorded in the
[native ledger](../docs/event-implementation.md). None of this surface is shipped
in public v1.3.1.
