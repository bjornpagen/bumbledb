# C++ cell() returns a copyable Value/bdb_value that borrows the carrier with no lifetime
- id: 111
- severity: medium
- confidence: confirmed
- area: ffi
- components: cpp/foreign/raii.cc, cpp/src/answers/decode.cc, cpp/src/answers/answers.cc, cpp/src/answers/row.cc
- status: open (do not fix)

## Summary
Outbound string/bytes payloads are views into `bdb_answers` / `bdb_row_set`. The C++ raii `cell()` copies a `bdb_value` POD (raw pointers) out of the FFI call; `decode_value` wraps those in `std::string_view` / `std::span<byte const>` inside a copyable `std::variant`. Destroying, moving, `clear()`ing, or re-executing the carrier leaves those copies dangling. Error messages were explicitly copied to `std::string` to avoid this; answers were not.

## Evidence
- `answers_handle::cell` / `row_set_handle::cell` return `optional<bdb_value>` (`cpp/foreign/raii.cc`). Comment: payloads BORROW this carrier.
- `decode_value` → `Value{text_of(cell.string_value)}` (`cpp/src/answers/decode.cc`). `Value` is a `variant` including `string_view` and `span`.
- `AnswersRaw::cell` / `RowSet::cell` return `optional<Value>` (`cpp/src/answers/answers.cc`).
- `RowAnswers::row` builds a `Row` product that can contain those views (`cpp/src/answers/row.cc`); `rows()` is a lazy range that yields `Row` by value.
- Contrast: `error_handle::message()` and `violation()` copy spelling into `std::string` because “the borrowed view dies with the error.”

## Why this is a bug
This is a classic FFI view-escape. The type system allows `auto v = answers.cell({0,0}); answers.clear(); use(v);` — use-after-free of the Rust `Box<str>` / byte heap. Snapshot/WriteTx were made non-copyable specifically to prevent this class of stash; `Value` was not. ASan on a cookbook that stores a decoded name then `clear()`s will fire.

## How to trigger / repro sketch
```
auto answers = … execute …;
auto v = answers.cell({.row=0, .column=0}); // string column
answers.clear(); // or destroy / re-execute
std::string copy{std::get<std::string_view>(*v)}; // UAF
```
Same with `RowSet` after `row_set_handle` destructor, and with `RowAnswers::row()` stored past `clear()`.

## Spec / docs notes
Comments document the borrow. C++ AGENTS.md: owning structs do not contain borrows unless named `*View`; `Value` is not a view type but holds views. Failure to copy at the dialect boundary is inconsistent with `error_handle`.

## Related
- 101 (another stash-the-borrow)
- 110 (clear on error makes this easier to hit)
