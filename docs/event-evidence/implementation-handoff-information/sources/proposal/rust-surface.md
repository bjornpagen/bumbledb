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
alongside [finite relational products and operators](../docs/event-relations.md)
[diagram inspection/reconstruction](../docs/event-inspection.md), and
[finite information operators](../docs/event-information.md).
Constructive query heads, complete descriptor transport, measured sources and the remaining
operators still have the acceptance gates recorded in the
[native ledger](../docs/event-implementation.md). None of this surface is shipped
in public v1.3.1.
