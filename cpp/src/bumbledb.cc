// bumbledb — the primary module interface: `import bumbledb;` is the ONE
// import an application needs (TODO_CPP §31). Internals are partitions —
// physically unimportable from outside the module — and every partition
// is re-exported here (a module interface partition must be reachable
// from the primary interface). The exported surface: the value vocabulary
// (:interval/:bytes/:allen/:fresh), the failure vocabulary (:error), the
// untyped result lane (:decode/:answers/:answers_row), host-side ordering
// (:order), the relation reflector (:facade — bdb::relation / bdb::coord
// / bdb::fixed_string), closed relations (:closed_facade / :handle / :id
// / :member), the statement algebra + schema elaborator (:schema — key /
// contained / mirrors / capacity / weigh / within / ref / duration), the
// query builder (:query / :program — point_in / allen_in / eq / ... /
// row_of / params_of), and the runtime resource layer (:db / :snapshot /
// :tx / :write / :prepared).
//
// The raw ABI quarantine stays a SEPARATE module (bumbledb_foreign): the
// pre-schema spec lane imports it explicitly and dies with it. The
// :foreign_program partition (foreign/program.cc) is quarantine code by
// nature but lives in THIS module because it consumes the query IR
// partitions (a partition of bumbledb_foreign could not import them
// without a module cycle).
//
// GCC-only: the module contains reflective partitions, so the whole
// module is outside the Clang lint graph.
export module bumbledb;

export import :interval;
export import :bytes;
export import :allen;
export import :fresh;
export import :error;
export import :order;
export import :version;
export import :name;
export import :classify;
export import :coord;
export import :facade;
export import :row;
export import :handle;
export import :id;
export import :member;
export import :axioms;
export import :closed_facade;
export import :where;
export import :spec;
export import :schema_member;
export import :face;
export import :key;
export import :contained;
export import :mirrors;
export import :capacity;
export import :unionfind;
export import :classes;
export import :schema;
export import :ir;
export import :var;
export import :param;
export import :pattern;
export import :predicate;
export import :aggregate;
export import :head;
export import :rule;
export import :lower;
export import :query;
export import :program;
export import :decode;
export import :answers;
export import :answers_row;
export import :manifest;
export import :wire;
export import :write;
export import :prepared;
export import :snapshot;
export import :tx;
export import :db;
export import :foreign_program;
