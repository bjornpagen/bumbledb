# Inbound bdb_string_view::as_str fabricates an unbounded lifetime from a raw pointer
- id: 118
- severity: low
- confidence: possible
- area: ffi
- components: cpp/bridge/src/value.rs, cpp/bridge/src/lib.rs
- status: open (do not fix)

## Summary
`bdb_string_view::as_str<'a>` and `slice_in<'a>` return `&'a T` where `'a` is chosen by the caller and is not tied to the FFI call. Current call sites copy into owned `Value` / `SchemaSpec` / `Program` before returning to C, so the lie does not escape today. Any future change that stores the `&str` (a cache, a lazy intern, a `'static` intern table) would be a silent use-after-free of C++ memory.

## Evidence
- `as_str<'a>(&self, …) -> BridgeResult<&'a str>` calls `slice_in(self.data, self.len)` (`cpp/bridge/src/value.rs`).
- `slice_in<'a, T>(ptr, count) -> &'a [T]` (`cpp/bridge/src/lib.rs`).
- Inbound copies: `value_in` does `text.as_bytes().to_vec()`, `schema_spec_in` / `program_in` own Strings. Outbound views are a different, documented borrow from Rust carriers.

## Why this is a bug
This is the same lifetime-erasure shape as the old Node `&'static Snapshot` (finding 018), just currently defused by copies. The type system will not stop a later patch from keeping the borrow. Confidence is *possible* because there is no current dangling use; the API is the hazard.

## How to trigger / repro sketch
Not triggerable in current code if every inbound path still copies. Confirm by searching for `as_str` / `slice_in` results that are stored in a struct field or returned to C without copying. Today they are not.

## Spec / docs notes
Header: “Views handed IN are copied before the call returns; no caller memory is retained.” That is a convention, not encoded in the Rust signatures.

## Related
- 100, 101 (other lifetime erasures that *do* escape)
