// bumbledb — the umbrella module: `import bumbledb;` is the ONE import an
// application needs (TODO_CPP §31). It re-exports the public SDK surface:
// the value vocabulary, the failure vocabulary, the untyped result lane,
// the runtime resource layer, the relation reflector's public face
// (bdb::relation / bdb::coord / bdb::fixed_string), the statement
// algebra + schema elaborator (bdb::schema / on / key / contained /
// mirrors / capacity / weigh / within / ref / duration), and the query
// builder (bdb::query / point_in / allen / eq / ... / row_of /
// params_of). Deliberately NOT re-exported: bumbledb.foreign /
// bumbledb.foreign.raii / bumbledb.foreign.program (the quarantine — the
// pre-schema spec lane imports them explicitly and dies with it) and the
// meta internals (bumbledb.meta.row is the marshalling machinery behind
// WriteTx, not API).
//
// GCC-only, like bumbledb.db (it re-exports reflective modules).
export module bumbledb;

export import bumbledb.types;
export import bumbledb.error;
export import bumbledb.answers;
export import bumbledb.db;
export import bumbledb.meta.relation;
export import bumbledb.meta.schema;
export import bumbledb.meta.query;
