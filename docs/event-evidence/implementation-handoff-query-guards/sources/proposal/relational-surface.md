# Relational surface

The current [event surface](event-surface.md) follows the existing pointwise
FD/IND meanings: keys reject overlap and mirrors prove coverage. A partition of
full has unit probability because its source law is normalized.

The earlier per-row probability-weight and numeric-capacity design is retired.
It could check a numerical total without identifying which worlds the rows
covered, and therefore did not exploit the region denotation. It is not a second
supported surface. The public name is `event` / `Event`.
