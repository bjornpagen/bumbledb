# engine-040: `ReachScratch` "Size 1" comment vs `[TransientImage; 2]` ping-pong

- **Severity:** low
- **Tree:** engine
- **Status:** DUPLICATE(engine-013)
- **Source:** audit/engine.md F40

engine-013's `PingPong { a, b, flip }` (or two named working fields) is the layout; the "Size 1" comment dies with `ReachScratch`. No separate fix lands under this id.
