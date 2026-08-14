# engine-036: `_either_sink_marker` dead-import hush

- **Severity:** low
- **Tree:** engine
- **Status:** DUPLICATE(engine-026)
- **Source:** audit/engine.md F36

engine-026 makes interiors/rec projection-typed and drops the `EitherSink` import from reach.rs; the `#[allow(dead_code)]` marker goes with it. No separate fix lands under this id.
