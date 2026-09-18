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

These documents define one proposed surface. The macros do not implement it yet.
