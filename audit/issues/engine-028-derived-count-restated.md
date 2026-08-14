# engine-028: `derived_count` / rec-id restated instead of stored

- **Severity:** medium
- **Tree:** engine
- **Status:** DUPLICATE(engine-003)
- **Source:** audit/engine.md F28

CONTRACT §C2 treats F3 (the Option + `len()` pun) and F28 (the same arithmetic restated, overflow re-`expect`ed) as one representation change. engine-003 already lists the sites (`reach.rs:303`, `build.rs:373-375`, `render.rs:153`, naive, translate) and owns storing `rec_id` / `derived_count` once on the witness. No separate fix lands under this id.
