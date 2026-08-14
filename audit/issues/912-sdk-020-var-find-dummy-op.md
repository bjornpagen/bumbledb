# sdk-020: variable finds store a dummy `fold_form::sum`

- **Severity:** low
- **Tree:** sdk (cpp)
- **Status:** DUPLICATE(sdk-004)
- **Source:** audit/sdks.md #20

sdk-004's four-case `find_form` is why Var stops carrying an op. The dummy `fold_form::sum` filler and the `BDB_HEAD_OP_SUM` write on Var heads die in that change. No separate fix lands under this id.
