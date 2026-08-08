# src/

The public C++ SDK modules (`bumbledb.*`, TODO_CPP §4): the relation model,
schema algebra, query surface, database/session types, and answers. Dialect
code only — named modules, `import std`, no headers, no preprocessor. Every
target links `bumbledb_language_profile`. Reflection-using module units live
in `meta/` (lint-graph zoning), never here.
