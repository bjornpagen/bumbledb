# sdk-019: C++ `derived_tables` re-threads the rec flag

- **Severity:** medium
- **Tree:** sdk (cpp)
- **Status:** DUPLICATE(sdk-001)
- **Source:** audit/sdks.md #19

Same defect as sdk-001 at the lowering coordinate. sdk-001 already requires `bool has_rec` gone from `cpp/src` and `query_view` / lowering to read `if constexpr (HasRec)`. No separate fix lands under this id.
