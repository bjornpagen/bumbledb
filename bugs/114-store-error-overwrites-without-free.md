# store_error overwrites a live bdb_error* without freeing it
- id: 114
- severity: low
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/lib.rs, cpp/foreign/raii.cc
- status: open (do not fix)

## Summary
On `BDB_STATUS_ERROR`, `store_error` does `*out_error = Box::into_raw(...)`. It never reads or destroys a previously stored pointer. A C caller that reuses the same `bdb_error**` slot without `bdb_error_destroy` leaks the first error (message `String`, violation spellings). C++ wrappers always start from `bdb_error* error = nullptr`, so the dialect is fine.

## Evidence
- `store_error` (`cpp/bridge/src/lib.rs`): null out-param drops the new error; else `*out_error = raw` with no `from_raw` of the old value.
- Header: caller owns the written `bdb_error*` and must `bdb_error_destroy`.
- raii: `bdb_error* error = nullptr` on every call (`cpp/foreign/raii.cc`).

## Why this is a bug
Easy leak on the raw ABI: two failing `bdb_db_open`s with one `bdb_error* err` variable, no destroy in between. Not a UAF unless the caller also keeps views into the first error and then (after a correct destroy of only the second) … they still leak the first. Combined with 111-style views, overwriting without destroy also leaves dangling `bdb_string_view`s into the leaked (still-alive) first error — confusing but the leak is the defect.

## How to trigger / repro sketch
```
bdb_error *err = NULL;
bdb_db_open(bad1, spec, &db, &err);
bdb_db_open(bad2, spec, &db, &err); // first err leaked
bdb_error_destroy(err);            // only frees the second
```

## Spec / docs notes
Standard C out-param ownership; the header does not say “slot must be null.” C++ RAII does the right thing.

## Related
- 104 (other leak-on-misuse paths)
