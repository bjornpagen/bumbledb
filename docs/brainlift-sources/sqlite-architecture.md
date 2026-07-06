# SQLite — Architecture (arch.html)

Source: https://www.sqlite.org/arch.html — fetched 2026-07-06

## Compile-then-execute model
- SQL text → bytecode via `sqlite3_prepare_v2()`; bytecode runs on a virtual
  machine via `sqlite3_step()`. "SQLite works by compiling SQL text into
  bytecode, then running that bytecode using a virtual machine."
- `sqlite3_stmt` (prepared statement) is "a container for a single bytecode
  program that implements a single SQL statement."

## Components (bottom-up)
- OS interface (VFS): os_unix.c/os_win.c — portability layer.
- Pager (pager.c, wal.c, pcache*.c): fixed-size pages, default 4096 bytes
  (512–65536 configurable); rollback/atomic-commit abstraction; locking.
- B-tree (btree.c): one B-tree per table and per index, all in one file.
- VDBE (vdbe.c): the bytecode virtual machine; values in "Mem" objects;
  built-in SQL functions are C callbacks (func.c, date.c).
- Code generator (select.c, where*.c, expr.c...): parse tree → bytecode.
  "The query planner is an AI that strives to select the best algorithm"
  from "hundreds, thousands, or millions of different algorithms."
- Parser: Lemon-generated (parse.y); tokenizer calls the parser (threadsafe,
  faster than the YACC direction).

## Load-bearing contrasts for bumbledb
- Interpreted bytecode dispatch per row vs bumbledb's monomorphized executor.
- The base structure IS the query structure (B-tree pages) — no derived
  columnar form, hence no rebuild-after-write cost and no scan-speed
  amortization.
