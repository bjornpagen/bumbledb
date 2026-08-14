# docs-018: "stays a runtime check (`ForeignPreparedQuery`)" — a guard documented as the last word

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F18
- **Depends on:** none (prose; same file as docs-015/016/017)

## The bug

`docs/architecture/70-api.md:754` — "same-schema/different-environment confusion stays a runtime check (`ForeignPreparedQuery`)."

## Why it's wrong

Insight 5 (a guard is a representation that hasn't been found yet): cross-SCHEMA confusion is already unrepresentable (`Db<S>`); cross-ENVIRONMENT is the same class of error left as an execute-time check, and the doc presents that as settled rather than as either (a) a representation to build or (b) essential complexity with a named horizon.

## The fix

Per `audit/CONTRACT.md §C7` (the F18 ruling): document it as ESSENTIAL runtime identity with the horizon representation named — "a prepared query is bound to the preparing environment, a process-runtime fact no static type can carry across processes; the horizon representation is branding `PreparedQuery` with an environment/generation witness so a foreign snapshot fails at the CALL type where the host language can express it. Today the engine detects it at execute (`ForeignPreparedQuery`)." The sentence "stays a runtime check" as the last word deletes. (This issue does NOT mandate the engine change — it mandates the doc stop presenting the check as the end state.)

## Acceptance criteria

- [ ] Gone: `rg -n 'stays a runtime check' docs/architecture/70-api.md` → no matches.
- [ ] `ForeignPreparedQuery` (the error) still documented; no engine code changed by this issue.

## Constraints

- Prose only. If a future engine issue brands `PreparedQuery`, this section updates again then.
