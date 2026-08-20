/* bumbledb_c.h — the generated C ABI of libbumbledb_c.
 *
 * GENERATED FILE — DO NOT EDIT. Regenerate from crates/bumbledb-c with:
 *     cbindgen --config cbindgen.toml --crate bumbledb-c \
 *         --output include/bumbledb_c.h
 * (pinned cbindgen 0.29.4; see crates/bumbledb-c/cbindgen.toml)
 *
 * Boundary protocol: every fallible function returns a bdb_status and
 * takes a trailing bdb_error** out-param — BDB_STATUS_OK (no error),
 * BDB_STATUS_ABORTED (the caller's callback aborted; no error),
 * BDB_STATUS_ERROR (a bdb_error* is written; the caller owns it and
 * frees it with bdb_error_destroy), BDB_STATUS_MISUSE (a contract
 * violation — null required pointer, stale instance/tx ref, index out
 * of range, unknown enum tag, bool payload other than 0/1; no error is
 * allocated).
 *
 * Lexical capabilities: each callback mints its own bdb_instance_ref
 * (and, for store reads, a borrowed bdb_witness). When the callback
 * returns the slot is invalidated. A stashed pointer answers
 * BDB_STATUS_MISUSE rather than use-after-free. bdb_db_write_from takes
 * a witness (borrowed or retained via bdb_witness_retain). Concurrent
 * MVCC reads on one handle are allowed. Theory rejection and a moved
 * generation fill an admission union under BDB_STATUS_OK.
 *
 * Callbacks: an extern "C" function pointer invoked from Rust. A C++
 * exception thrown through a callback is unsupported (it would unwind
 * through Rust). Unknown callback-control tags are MISUSE.
 *
 * View lifetimes: bdb_value string/bytes payloads handed OUT borrow the
 * carrier named at the accessor (bdb_row_set, bdb_answers, bdb_error)
 * and die with it (or on the carrier's next clear/execute). Views handed
 * IN are copied before the call returns; no caller memory is retained.
 */

#ifndef BUMBLEDB_C_H
#define BUMBLEDB_C_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

// The status every fallible export returns (module doc: the boundary
// protocol).
typedef enum bdb_status {
  BDB_STATUS_OK = 0,
  BDB_STATUS_ERROR = 1,
  BDB_STATUS_ABORTED = 2,
  BDB_STATUS_MISUSE = 3,
} bdb_status;

// Admission discriminant. Zero is the documented empty/uninitialized
// state and is never returned with `BDB_STATUS_OK`.
typedef enum bdb_admission_tag {
  BDB_ADMISSION_TAG_EMPTY = 0,
  BDB_ADMISSION_TAG_ACCEPTED = 1,
  BDB_ADMISSION_TAG_REJECTED = 2,
  BDB_ADMISSION_TAG_MOVED = 3,
} bdb_admission_tag;

// Tagged fresh-id range. Empty is the tag, never `{0, 0}`.
typedef enum bdb_fresh_range_tag {
  BDB_FRESH_RANGE_TAG_EMPTY = 0,
  BDB_FRESH_RANGE_TAG_NON_EMPTY = 1,
} bdb_fresh_range_tag;

// Origin of a [`bdb_error`]: engine taxonomy vs bridge marshal/busy.
typedef enum bdb_error_origin {
  BDB_ERROR_ORIGIN_ENGINE = 0,
  BDB_ERROR_ORIGIN_BRIDGE = 1,
} bdb_error_origin;

// The C error kind — one constant per engine error family, plus the
// bridge-synthesized `Panic`, `BusyHandle`, and `Marshal`. Proved write
// outcomes are admission-union arms, not kinds.
typedef enum bdb_error_kind {
  BDB_ERROR_KIND_SCHEMA,
  BDB_ERROR_KIND_SCHEMA_MISMATCH,
  BDB_ERROR_KIND_FORMAT_MISMATCH,
  BDB_ERROR_KIND_ALREADY_INITIALIZED,
  BDB_ERROR_KIND_DESTINATION_EXISTS,
  BDB_ERROR_KIND_PUBLISHED_BUT_UNSYNCED,
  BDB_ERROR_KIND_ENVIRONMENT_LOCKED,
  BDB_ERROR_KIND_READERS_FULL,
  BDB_ERROR_KIND_VALIDATION,
  BDB_ERROR_KIND_COMMIT_SYNC,
  BDB_ERROR_KIND_FOREIGN_WITNESS,
  BDB_ERROR_KIND_FOREIGN_PREPARED,
  BDB_ERROR_KIND_FACT_SHAPE,
  BDB_ERROR_KIND_CLOSED_RELATION_WRITE,
  BDB_ERROR_KIND_FRESH_EXHAUSTED,
  BDB_ERROR_KIND_TRANSACTION_POISONED,
  BDB_ERROR_KIND_PARAM,
  BDB_ERROR_KIND_MEASURE_OF_RAY,
  BDB_ERROR_KIND_CAPACITY_RAY_MEASURE,
  BDB_ERROR_KIND_DERIVED_BUDGET_EXCEEDED,
  BDB_ERROR_KIND_OVERFLOW,
  BDB_ERROR_KIND_RESULT_BYTES_OVERFLOW,
  BDB_ERROR_KIND_CORRUPTION,
  BDB_ERROR_KIND_IO,
  BDB_ERROR_KIND_LMDB,
  BDB_ERROR_KIND_PANIC,
  BDB_ERROR_KIND_BUSY_HANDLE,
  BDB_ERROR_KIND_MARSHAL,
} bdb_error_kind;

// A violated statement's form tag (`bumbledb::StatementKind`, spelled C).
typedef enum bdb_statement_kind {
  BDB_STATEMENT_KIND_FUNCTIONALITY,
  BDB_STATEMENT_KIND_CONTAINMENT,
  BDB_STATEMENT_KIND_CAPACITY,
} bdb_statement_kind;

// A containment citation's violated side. Live only on the containment
// payload arm.
typedef enum bdb_violation_direction {
  BDB_VIOLATION_DIRECTION_SOURCE_UNSATISFIED,
  BDB_VIOLATION_DIRECTION_TARGET_REQUIRED,
} bdb_violation_direction;

// The value tag — one constant per `bumbledb::Value` variant.
typedef enum bdb_value_kind {
  BDB_VALUE_KIND_BOOL,
  BDB_VALUE_KIND_U64,
  BDB_VALUE_KIND_I64,
  BDB_VALUE_KIND_STRING,
  BDB_VALUE_KIND_FIXED_BYTES,
  BDB_VALUE_KIND_INTERVAL_U64,
  BDB_VALUE_KIND_INTERVAL_I64,
} bdb_value_kind;

// The execute-parameter tag: a scalar (a [`bdb_value`]) or a param set
// (a value array — points only; the engine types the elements).
typedef enum bdb_param_kind {
  BDB_PARAM_KIND_SCALAR,
  BDB_PARAM_KIND_SET,
} bdb_param_kind;

// The structural value-type tag (`bumbledb::schema::ValueType`, spelled
// C).
typedef enum bdb_value_type_kind {
  BDB_VALUE_TYPE_KIND_BOOL,
  BDB_VALUE_TYPE_KIND_U64,
  BDB_VALUE_TYPE_KIND_I64,
  BDB_VALUE_TYPE_KIND_STRING,
  BDB_VALUE_TYPE_KIND_FIXED_BYTES,
  BDB_VALUE_TYPE_KIND_INTERVAL,
} bdb_value_type_kind;

// An interval's element domain.
typedef enum bdb_interval_element {
  BDB_INTERVAL_ELEMENT_U64,
  BDB_INTERVAL_ELEMENT_I64,
} bdb_interval_element;

// A literal's tag: a plain tagged value, or a closed relation's handle
// by name.
typedef enum bdb_literal_kind {
  BDB_LITERAL_KIND_VALUE,
  BDB_LITERAL_KIND_HANDLE,
} bdb_literal_kind;

// A statement's form tag.
typedef enum bdb_statement_spec_kind {
  BDB_STATEMENT_SPEC_KIND_FD,
  BDB_STATEMENT_SPEC_KIND_CONTAINMENT,
  BDB_STATEMENT_SPEC_KIND_CAPACITY,
} bdb_statement_spec_kind;

// A σ binding's right side: one literal or a literal set (read
// disjunctively).
typedef enum bdb_literal_set_kind {
  BDB_LITERAL_SET_KIND_ONE,
  BDB_LITERAL_SET_KIND_MANY,
} bdb_literal_set_kind;

// A capacity weight's tag.
typedef enum bdb_weight_kind {
  BDB_WEIGHT_KIND_UNIT,
  BDB_WEIGHT_KIND_FIELD,
  BDB_WEIGHT_KIND_DURATION_FIELD,
} bdb_weight_kind;

// A capacity window's tag.
typedef enum bdb_capacity_window_kind {
  BDB_CAPACITY_WINDOW_KIND_EXACT,
  BDB_CAPACITY_WINDOW_KIND_RANGE,
  BDB_CAPACITY_WINDOW_KIND_FLOOR,
} bdb_capacity_window_kind;

// A capacity bound's tag.
typedef enum bdb_bound_kind {
  BDB_BOUND_KIND_LIT,
  BDB_BOUND_KIND_FIELD,
  BDB_BOUND_KIND_DURATION_FIELD,
} bdb_bound_kind;

// A callback's control return: `Ok` commits (write) / completes (read);
// `Abort` abandons — the write delta drops, LMDB untouched, and the outer
// call returns `BDB_STATUS_ABORTED` (the ts bridge's abort sentinel,
// spelled as control flow).
typedef enum bdb_callback_control {
  BDB_CALLBACK_CONTROL_OK = 0,
  BDB_CALLBACK_CONTROL_ABORT = 1,
} bdb_callback_control;

// A head position's tag.
typedef enum bdb_head_term_kind {
  BDB_HEAD_TERM_KIND_VAR,
  BDB_HEAD_TERM_KIND_AGGREGATE,
} bdb_head_term_kind;

// The var-free aggregate-op kind at a head position
// (`bumbledb::ir::HeadOp`).
typedef enum bdb_head_op {
  BDB_HEAD_OP_SUM,
  BDB_HEAD_OP_MIN,
  BDB_HEAD_OP_MAX,
  BDB_HEAD_OP_COUNT,
  BDB_HEAD_OP_PACK,
} bdb_head_op;

// A find term's tag (`bumbledb::ir::FindTerm`).
typedef enum bdb_find_term_kind {
  BDB_FIND_TERM_KIND_VAR,
  BDB_FIND_TERM_KIND_MEASURE,
  BDB_FIND_TERM_KIND_AGGREGATE,
  BDB_FIND_TERM_KIND_AGGREGATE_MEASURE,
  BDB_FIND_TERM_KIND_COUNT,
} bdb_find_term_kind;

// An atom source's tag: a stored relation (`Edb`) or a derived table of
// the same query (`Interior` — an interior or the rec).
typedef enum bdb_atom_source_kind {
  BDB_ATOM_SOURCE_KIND_EDB,
  BDB_ATOM_SOURCE_KIND_INTERIOR,
} bdb_atom_source_kind;

// A term's tag (`bumbledb::ir::Term`).
typedef enum bdb_term_kind {
  BDB_TERM_KIND_VAR,
  BDB_TERM_KIND_PARAM,
  BDB_TERM_KIND_PARAM_SET,
  BDB_TERM_KIND_LITERAL,
  BDB_TERM_KIND_MEASURE,
} bdb_term_kind;

// A condition node's tag.
typedef enum bdb_condition_kind {
  BDB_CONDITION_KIND_LEAF,
  BDB_CONDITION_KIND_AND,
  BDB_CONDITION_KIND_OR,
} bdb_condition_kind;

// A comparison operator's tag (`bumbledb::ir::CmpOp`). For `PointIn`
// the lhs is the INTERVAL term and the rhs the point term (the engine's
// ordered lowering; the notation reads point-first).
typedef enum bdb_cmp_op_kind {
  BDB_CMP_OP_KIND_EQ,
  BDB_CMP_OP_KIND_NE,
  BDB_CMP_OP_KIND_LT,
  BDB_CMP_OP_KIND_LE,
  BDB_CMP_OP_KIND_GT,
  BDB_CMP_OP_KIND_GE,
  BDB_CMP_OP_KIND_ALLEN,
  BDB_CMP_OP_KIND_POINT_IN,
} bdb_cmp_op_kind;

// Q1 discriminant: a CQ (no rec) or a Reach (rec by value).
typedef enum bdb_query_kind {
  BDB_QUERY_KIND_CQ,
  BDB_QUERY_KIND_REACH,
} bdb_query_kind;

// The opaque, reusable answers carrier.
typedef struct bdb_answers bdb_answers;

// Opaque database handle.
typedef struct bdb_db bdb_db;

// The opaque error handle: origin, kind, rendered message. Owned by the
// caller after a `BDB_STATUS_ERROR` return; freed by [`bdb_error_destroy`].
typedef struct bdb_error bdb_error;

// Opaque heap builder. Spent by [`bdb_instance_builder_admit`].
typedef struct bdb_instance_builder bdb_instance_builder;

// Borrowed query surface, valid only during the callback that minted it.
typedef struct bdb_instance_ref bdb_instance_ref;

// Opaque admitted heap instance.
typedef struct bdb_owned_instance bdb_owned_instance;

// The opaque prepared-query handle. Field order is load-bearing: the
// prepared value drops before the optional store `Arc`. Heap-prepared
// queries hold `None`.
typedef struct bdb_prepared bdb_prepared;

// Owned row carrier for scans and point reads.
typedef struct bdb_row_set bdb_row_set;

// Borrowed write-transaction capability, valid only inside the write
// callback.
typedef struct bdb_tx_ref bdb_tx_ref;

// Owning handle for a rejected admission's violation set. Destroy with
// [`bdb_violations_destroy`].
typedef struct bdb_violations bdb_violations;

// Generation witness: cloneable evidence. A callback argument is borrowed
// and invalidated when the callback returns. [`bdb_witness_retain`] clones
// an owning handle.
typedef struct bdb_witness bdb_witness;

// A borrowed UTF-8 text view (NOT NUL-terminated; the length is the
// contract). A null `data` with `len == 0` is the empty string; a null
// `data` under a nonzero `len` is misuse. In optional positions
// (`bdb_field_spec.newtype`) a null `data` means ABSENT.
typedef struct bdb_string_view {
  const uint8_t *data;
  size_t len;
} bdb_string_view;

// A borrowed raw-byte view (`bytes<N>` payloads). Null/len rules as
// [`bdb_string_view`].
typedef struct bdb_bytes_view {
  const uint8_t *data;
  size_t len;
} bdb_bytes_view;

// One tagged value. Only the fields the `kind` names are read; the rest
// are ignored inbound and zeroed outbound. Boring and flat by design —
// no union, no packing.
typedef struct bdb_value {
  // Integer tag; valid values are [`bdb_value_kind`]. Unknown tags are
  // `BDB_STATUS_MISUSE` (the field is `u32` so an out-of-range C enum
  // is not UB).
  uint32_t kind;
  // 0 or 1 when `kind` is Bool; any other byte is misuse.
  uint8_t bool_value;
  uint64_t u64_value;
  int64_t i64_value;
  // `String`: UTF-8 text (checked at the boundary).
  struct bdb_string_view string_value;
  // `FixedBytes`: exactly the field's N bytes (the engine checks N).
  struct bdb_bytes_view bytes_value;
  // `IntervalU64`: half-open `[start, end)`, `start < end` checked.
  uint64_t interval_u64_start;
  uint64_t interval_u64_end;
  // `IntervalI64`: half-open `[start, end)`, `start < end` checked.
  int64_t interval_i64_start;
  int64_t interval_i64_end;
} bdb_value;

// One positional execution argument — the C mirror of the engine's
// public `ParamArg` shape (Scalar | Set).
typedef struct bdb_param {
  uint32_t kind;
  // `Scalar`: the value.
  struct bdb_value scalar;
  // `Set`: `set_len` tagged values.
  const struct bdb_value *set;
  size_t set_len;
} bdb_param;

// One structural value type. `fixed_len` is read for `FixedBytes`;
// `element` / `has_width` / `width` for `Interval` (`has_width == false`
// is the general 16-byte interval; `true` the fixed-width family).
typedef struct bdb_value_type {
  uint32_t kind;
  uint16_t fixed_len;
  uint32_t element;
  uint8_t has_width;
  uint64_t width;
} bdb_value_type;

// One field: name, structural type, optional host newtype label (null
// `data` = absent; carried for closed-handle resolution only, dropped at
// descriptor lowering), and the `fresh` mark.
typedef struct bdb_field_spec {
  struct bdb_string_view name;
  struct bdb_value_type value_type;
  struct bdb_string_view newtype;
  uint8_t fresh;
} bdb_field_spec;

// One literal as spelled.
typedef struct bdb_literal {
  uint32_t kind;
  struct bdb_value value;
  struct bdb_string_view handle;
} bdb_literal;

// One ground axiom of a closed relation: the handle plus one literal per
// declared intrinsic column, in field-declaration order.
typedef struct bdb_closed_row {
  struct bdb_string_view handle;
  const struct bdb_literal *values;
  size_t value_count;
} bdb_closed_row;

// A closed relation's closed half: the handle newtype and the ground
// axioms, fused (absent `closed` on the relation = ordinary relation).
typedef struct bdb_closed_spec {
  struct bdb_string_view newtype;
  const struct bdb_closed_row *rows;
  size_t row_count;
} bdb_closed_spec;

// One relation: name, declared fields, and closedness (`closed` null =
// ordinary).
typedef struct bdb_relation_spec {
  struct bdb_string_view name;
  const struct bdb_field_spec *fields;
  size_t field_count;
  const struct bdb_closed_spec *closed;
} bdb_relation_spec;

// One literal set. `One` reads `literals[0]` (`literal_count` must be
// 1); `Many` reads all `literal_count` entries.
typedef struct bdb_literal_set {
  uint32_t kind;
  const struct bdb_literal *literals;
  size_t literal_count;
} bdb_literal_set;

// One σ binding: `field == literal-or-set`.
typedef struct bdb_selection_binding {
  struct bdb_string_view field;
  struct bdb_literal_set set;
} bdb_selection_binding;

// One side of a containment/capacity statement:
// `Relation(projection… | selection…)`, all names.
typedef struct bdb_side {
  struct bdb_string_view relation;
  const struct bdb_string_view *projection;
  size_t projection_count;
  const struct bdb_selection_binding *selection;
  size_t selection_count;
} bdb_side;

// A capacity weight; `field` is read for `Field`/`DurationField`.
typedef struct bdb_weight {
  uint32_t kind;
  struct bdb_string_view field;
} bdb_weight;

// One capacity bound; `lit` for `Lit`, `field` for
// `Field`/`DurationField`.
typedef struct bdb_bound {
  uint32_t kind;
  uint64_t lit;
  struct bdb_string_view field;
} bdb_bound;

// One capacity window: `Exact` reads `lo` as the exact bound; `Floor`
// reads `lo`; `Range` reads `lo` and `hi`.
typedef struct bdb_capacity_window {
  uint32_t kind;
  struct bdb_bound lo;
  struct bdb_bound hi;
} bdb_capacity_window;

// One dependency statement. `Fd` reads `fd_relation` +
// `fd_projection`; `Containment` reads `source`/`target`/`bidirectional`;
// `Capacity` reads `target`/`weight`/`window`/`source` (the operator's
// read order).
typedef struct bdb_statement_spec {
  uint32_t kind;
  struct bdb_string_view fd_relation;
  const struct bdb_string_view *fd_projection;
  size_t fd_projection_count;
  struct bdb_side source;
  struct bdb_side target;
  uint8_t bidirectional;
  struct bdb_weight weight;
  struct bdb_capacity_window window;
} bdb_statement_spec;

// The whole schema spec: relations then statements, declaration order —
// the order IS the id mint.
typedef struct bdb_schema_spec {
  const struct bdb_relation_spec *relations;
  size_t relation_count;
  const struct bdb_statement_spec *statements;
  size_t statement_count;
} bdb_schema_spec;

typedef union bdb_db_admission_value {
  struct bdb_db *accepted;
  struct bdb_violations *rejected;
} bdb_db_admission_value;

typedef struct bdb_db_admission {
  enum bdb_admission_tag tag;
  union bdb_db_admission_value value;
} bdb_db_admission;

// 64 lowercase hex chars of the store's schema fingerprint.
typedef struct bdb_fingerprint {
  uint8_t hex[64];
} bdb_fingerprint;

// Facts consumed vs facts that changed the in-memory final-state view.
typedef struct bdb_mutation_report {
  uint64_t submitted;
  uint64_t changed;
} bdb_mutation_report;

typedef struct bdb_fresh_range {
  enum bdb_fresh_range_tag tag;
  uint64_t start;
  uint64_t end_exclusive;
} bdb_fresh_range;

typedef union bdb_instance_admission_value {
  struct bdb_owned_instance *accepted;
  struct bdb_violations *rejected;
} bdb_instance_admission_value;

typedef struct bdb_instance_admission {
  enum bdb_admission_tag tag;
  union bdb_instance_admission_value value;
} bdb_instance_admission;

// Heap-instance callback: the same query surface, no witness.
typedef uint32_t (*bdb_owned_instance_read_callback)(void *context,
                                                     const struct bdb_instance_ref *instance);

// Store-read callback: instance + borrowed witness.
typedef uint32_t (*bdb_db_read_callback)(void *context,
                                         const struct bdb_instance_ref *instance,
                                         const struct bdb_witness *witness);

typedef uint32_t (*bdb_write_callback)(void *context, struct bdb_tx_ref *transaction);

typedef struct bdb_moved_generations {
  uint64_t witnessed;
  uint64_t current;
} bdb_moved_generations;

typedef union bdb_write_admission_value {
  uint64_t accepted_generation;
  struct bdb_violations *rejected;
  struct bdb_moved_generations moved;
} bdb_write_admission_value;

typedef struct bdb_write_admission {
  enum bdb_admission_tag tag;
  union bdb_write_admission_value value;
} bdb_write_admission;

// One head position; `op` is read for `Aggregate`.
typedef struct bdb_head_term {
  uint32_t kind;
  uint32_t op;
} bdb_head_term;

// One rule-scoped aggregate op.
typedef struct bdb_agg_op {
  uint32_t kind;
} bdb_agg_op;

// One find term. `var` is read for `Var`/`Measure`; `op` plus `over` for
// `Aggregate`/`AggregateMeasure` (folds always carry `over`); `Count` is
// nullary and does not read `over`.
typedef struct bdb_find_term {
  uint32_t kind;
  uint16_t var;
  struct bdb_agg_op op;
  uint16_t over;
} bdb_find_term;

// One term. `var` is read for `Var`/`Measure`, `param` for
// `Param`/`ParamSet`, `literal` for `Literal`.
typedef struct bdb_term {
  uint32_t kind;
  uint16_t var;
  uint16_t param;
  struct bdb_value literal;
} bdb_term;

// One atom binding: `(field, term)`. Absence of a field is the
// wildcard — bind only what the rule constrains.
typedef struct bdb_binding {
  uint16_t field;
  struct bdb_term term;
} bdb_binding;

// One atom. `relation` is read for `Edb`, `interior` for `Interior`.
typedef struct bdb_atom {
  uint32_t source_kind;
  uint32_t relation;
  uint32_t interior;
  const struct bdb_binding *bindings;
  size_t binding_count;
} bdb_atom;

// One comparison operator; `mask` is the literal 13-bit Allen mask,
// read for `Allen` only.
typedef struct bdb_cmp_op {
  uint32_t kind;
  uint16_t mask;
} bdb_cmp_op;

// One comparison condition.
typedef struct bdb_comparison {
  struct bdb_cmp_op op;
  struct bdb_term lhs;
  struct bdb_term rhs;
} bdb_comparison;

// One condition-tree node. `cmp` is read for `Leaf`;
// `children`/`child_count` for `And`/`Or`. Nesting past the engine's
// `MAX_CONDITION_DEPTH` is refused at marshal (the engine's own bound,
// re-checked here so the recursion is stack-safe on hostile input).
typedef struct bdb_condition {
  uint32_t kind;
  struct bdb_comparison cmp;
  const struct bdb_condition *children;
  size_t child_count;
} bdb_condition;

// One rule: finds against the head, positive atoms, negated atoms,
// conditions (conjoined).
typedef struct bdb_rule {
  const struct bdb_find_term *finds;
  size_t find_count;
  const struct bdb_atom *atoms;
  size_t atom_count;
  const struct bdb_atom *negated;
  size_t negated_count;
  const struct bdb_condition *conditions;
  size_t condition_count;
} bdb_rule;

// One named interior: a finite CQ (union of conjunctive rules).
typedef struct bdb_interior {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_interior;

// CQ payload: named interiors, then the main answer. No rec slot.
typedef struct bdb_cq {
  const struct bdb_interior *interiors;
  size_t interior_count;
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_cq;

// One linear rec: base arms and rec arms.
typedef struct bdb_rec {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *base;
  size_t base_count;
  const struct bdb_rule *rec;
  size_t rec_count;
} bdb_rec;

// Reach payload: named interiors, a required rec, then the main answer.
// `rec` is the Reach arm's rec by value — not a nullable pointer.
typedef struct bdb_reach {
  const struct bdb_interior *interiors;
  size_t interior_count;
  struct bdb_rec rec;
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_reach;

// Live arm of [`bdb_query`]: CQ or Reach. Read only the arm `kind` names.
typedef union bdb_query_payload {
  struct bdb_cq cq;
  struct bdb_reach reach;
} bdb_query_payload;

// The whole query: tagged encoding of Q1 (`Cq | Reach`).
typedef struct bdb_query {
  uint32_t kind;
  union bdb_query_payload payload;
} bdb_query;

// Capacity measure as two u64 words (lo then hi). Live only on the
// capacity payload arm.
typedef struct bdb_capacity_measure {
  uint64_t lo;
  uint64_t hi;
} bdb_capacity_measure;

// Per-kind payload of [`bdb_violation`]. Inspect the arm that matches
// `kind`; the other cells are uninitialized.
typedef union bdb_violation_payload {
  uint8_t functionality;
  enum bdb_violation_direction containment;
  struct bdb_capacity_measure capacity;
} bdb_violation_payload;

// One rendered violation, viewed: statement id, form tag, canonical
// spelling (borrowed from the owning [`bdb_violations`]), and the
// kind's payload arm.
typedef struct bdb_violation {
  uint16_t statement;
  enum bdb_statement_kind kind;
  struct bdb_string_view spelling;
  union bdb_violation_payload payload;
} bdb_violation;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Crate version, NUL-terminated, program lifetime. Mirrors the Node
// bridge's `engine_version` as a C string the host can print.
const char *bdb_version(void);

// C ABI generation. `3` is instance-lifetime: admission unions, the
// builder/owned/witness handles, and the retirement of snapshot-named
// functions.
uint32_t bdb_abi_version(void);

// Mints an empty answers carrier (never fails; owns nothing yet).
struct bdb_answers *bdb_answers_new(void);

// Empties the carrier, retaining capacity (the zero-alloc reuse path).
enum bdb_status bdb_answers_clear(struct bdb_answers *answers);

// Number of answers (0 for a null handle).
size_t bdb_answers_len(const struct bdb_answers *answers);

// Number of columns — the executed query's find terms, in order (0 for
// a null handle).
size_t bdb_answers_arity(const struct bdb_answers *answers);

// One answer cell, viewed — string/bytes payloads BORROW the carrier
// and are valid only while it is alive, un-cleared, and un-re-executed.
// Bounds-checked bridge-side: `BDB_STATUS_MISUSE` out of range, never a
// panic.
enum bdb_status bdb_answers_get(const struct bdb_answers *answers,
                                size_t row,
                                size_t column,
                                struct bdb_value *out_value);

// Frees the carrier (invalidating every view borrowed from it).
enum bdb_status bdb_answers_destroy(struct bdb_answers *answers);

// Executes a prepared query against the instance with positional
// params, filling the caller's reusable carrier (cleared first,
// capacity retained). The prepared handle is taken exclusively for the
// call (`&mut` on the engine side — one execution at a time); executing
// a prepared query against a foreign instance is the engine's own typed
// `BDB_ERROR_KIND_FOREIGN_PREPARED`.
enum bdb_status bdb_instance_execute(const struct bdb_instance_ref *instance,
                                     struct bdb_prepared *prepared,
                                     const struct bdb_param *params,
                                     size_t param_count,
                                     struct bdb_answers *answers,
                                     struct bdb_error **out_error);

// Creates a fresh DURABLE store. Empty that does not hold is
// `BDB_ADMISSION_REJECTED` with no directory. `BDB_STATUS_OK` always
// fills `out_admission` (never the empty tag).
enum bdb_status bdb_db_create(struct bdb_string_view path,
                              const struct bdb_schema_spec *spec,
                              struct bdb_db_admission *out_admission,
                              struct bdb_error **out_error);

// Opens an existing durable store. No admission union — format-8 open
// carries admission provenance.
enum bdb_status bdb_db_open(struct bdb_string_view path,
                            const struct bdb_schema_spec *spec,
                            struct bdb_db **out_db,
                            struct bdb_error **out_error);

// Raw-copies an admitted heap instance into a new durable store.
enum bdb_status bdb_db_from_instance(struct bdb_string_view path,
                                     const struct bdb_owned_instance *instance,
                                     struct bdb_db **out_db,
                                     struct bdb_error **out_error);

enum bdb_status bdb_db_destroy(struct bdb_db *db);

enum bdb_status bdb_db_fingerprint(const struct bdb_db *db,
                                   struct bdb_fingerprint *out_fingerprint,
                                   struct bdb_error **out_error);

enum bdb_status bdb_instance_builder_new(const struct bdb_schema_spec *spec,
                                         struct bdb_instance_builder **out_builder,
                                         struct bdb_error **out_error);

enum bdb_status bdb_instance_builder_load(struct bdb_instance_builder *builder,
                                          uint32_t relation,
                                          const struct bdb_value *values,
                                          size_t value_count,
                                          size_t row_count,
                                          struct bdb_mutation_report *out_report,
                                          struct bdb_error **out_error);

enum bdb_status bdb_instance_builder_delete(struct bdb_instance_builder *builder,
                                            uint32_t relation,
                                            const struct bdb_value *values,
                                            size_t value_count,
                                            size_t row_count,
                                            struct bdb_mutation_report *out_report,
                                            struct bdb_error **out_error);

enum bdb_status bdb_instance_builder_reserve(struct bdb_instance_builder *builder,
                                             uint32_t relation,
                                             uint16_t field,
                                             uint64_t count,
                                             struct bdb_fresh_range *out_range,
                                             struct bdb_error **out_error);

// Consumes the builder on every outcome and nulls the caller's pointer.
enum bdb_status bdb_instance_builder_admit(struct bdb_instance_builder **builder,
                                           struct bdb_instance_admission *out_admission,
                                           struct bdb_error **out_error);

enum bdb_status bdb_instance_builder_destroy(struct bdb_instance_builder *builder);

enum bdb_status bdb_owned_instance_destroy(struct bdb_owned_instance *instance);

// Borrows an owned instance through the common [`bdb_instance_ref`]
// query surface.
enum bdb_status bdb_owned_instance_read(const struct bdb_owned_instance *instance,
                                        bdb_owned_instance_read_callback callback,
                                        void *context,
                                        struct bdb_error **out_error);

enum bdb_status bdb_db_read(const struct bdb_db *db,
                            bdb_db_read_callback callback,
                            void *context,
                            struct bdb_error **out_error);

enum bdb_status bdb_db_write(const struct bdb_db *db,
                             bdb_write_callback callback,
                             void *context,
                             struct bdb_write_admission *out_admission,
                             struct bdb_error **out_error);

enum bdb_status bdb_db_write_from(const struct bdb_db *db,
                                  const struct bdb_witness *witness,
                                  bdb_write_callback callback,
                                  void *context,
                                  struct bdb_write_admission *out_admission,
                                  struct bdb_error **out_error);

enum bdb_status bdb_witness_retain(const struct bdb_witness *witness,
                                   struct bdb_witness **out_witness,
                                   struct bdb_error **out_error);

enum bdb_status bdb_witness_destroy(struct bdb_witness *witness);

enum bdb_status bdb_tx_insert(const struct bdb_tx_ref *transaction,
                              uint32_t relation,
                              const struct bdb_value *values,
                              size_t value_count,
                              size_t row_count,
                              struct bdb_mutation_report *out_report,
                              struct bdb_error **out_error);

enum bdb_status bdb_tx_delete(const struct bdb_tx_ref *transaction,
                              uint32_t relation,
                              const struct bdb_value *values,
                              size_t value_count,
                              size_t row_count,
                              struct bdb_mutation_report *out_report,
                              struct bdb_error **out_error);

enum bdb_status bdb_tx_contains(const struct bdb_tx_ref *transaction,
                                uint32_t relation,
                                const struct bdb_value *values,
                                size_t value_count,
                                uint8_t *out_contains,
                                struct bdb_error **out_error);

enum bdb_status bdb_tx_get(const struct bdb_tx_ref *transaction,
                           uint32_t relation,
                           uint16_t key_statement,
                           const struct bdb_value *key_values,
                           size_t key_value_count,
                           struct bdb_row_set **out_row,
                           struct bdb_error **out_error);

enum bdb_status bdb_tx_reserve(const struct bdb_tx_ref *transaction,
                               uint32_t relation,
                               uint16_t field,
                               uint64_t count,
                               struct bdb_fresh_range *out_range,
                               struct bdb_error **out_error);

enum bdb_status bdb_instance_contains(const struct bdb_instance_ref *instance,
                                      uint32_t relation,
                                      const struct bdb_value *values,
                                      size_t value_count,
                                      uint8_t *out_contains,
                                      struct bdb_error **out_error);

enum bdb_status bdb_instance_get(const struct bdb_instance_ref *instance,
                                 uint32_t relation,
                                 uint16_t key_statement,
                                 const struct bdb_value *key_values,
                                 size_t key_value_count,
                                 struct bdb_row_set **out_row,
                                 struct bdb_error **out_error);

enum bdb_status bdb_instance_scan(const struct bdb_instance_ref *instance,
                                  uint32_t relation,
                                  struct bdb_row_set **out_rows,
                                  struct bdb_error **out_error);

enum bdb_status bdb_instance_prepare(const struct bdb_instance_ref *instance,
                                     const struct bdb_query *query,
                                     struct bdb_prepared **out_prepared,
                                     struct bdb_error **out_error);

size_t bdb_row_set_len(const struct bdb_row_set *rows);

size_t bdb_row_set_arity(const struct bdb_row_set *rows);

enum bdb_status bdb_row_set_get(const struct bdb_row_set *rows,
                                size_t row,
                                size_t column,
                                struct bdb_value *out_value);

enum bdb_status bdb_row_set_destroy(struct bdb_row_set *rows);

// The error's origin. A null handle answers `Bridge`.
enum bdb_error_origin bdb_error_get_origin(const struct bdb_error *error);

// The error's kind. A null handle answers `Panic`.
enum bdb_error_kind bdb_error_get_kind(const struct bdb_error *error);

// The rendered message, borrowed from the error (valid until
// `bdb_error_destroy`). UTF-8, NOT NUL-terminated.
enum bdb_status bdb_error_get_message(const struct bdb_error *error,
                                      struct bdb_string_view *out_message);

// Frees an error. Exactly once per owned error; a null pointer is
// misuse.
enum bdb_status bdb_error_destroy(struct bdb_error *error);

// The rendered violation count (0 for a null handle).
size_t bdb_violations_len(const struct bdb_violations *violations);

// One rendered violation, viewed (the spelling borrows from the handle).
// Bounds-checked: `BDB_STATUS_MISUSE` past [`bdb_violations_len`].
enum bdb_status bdb_violations_get(const struct bdb_violations *violations,
                                   size_t index,
                                   struct bdb_violation *out_violation);

// Frees a violations handle. A null pointer is misuse.
enum bdb_status bdb_violations_destroy(struct bdb_violations *violations);

// Prepares a query against the database: the engine validates,
// normalizes, reads statistics, and plans ONCE; the returned handle is
// reusable across reads of this database (`&mut` per execution —
// one execution at a time; the handle is not thread-shareable).
// Validation (roster) failures are `BDB_ERROR_KIND_VALIDATION`.
enum bdb_status bdb_db_prepare(const struct bdb_db *db,
                               const struct bdb_query *query,
                               struct bdb_prepared **out_prepared,
                               struct bdb_error **out_error);

// Releases a prepared query (its plan, memo, and engine reference).
enum bdb_status bdb_prepared_destroy(struct bdb_prepared *prepared);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* BUMBLEDB_C_H */
