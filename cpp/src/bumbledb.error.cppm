// bumbledb.error — the engine failure vocabulary (TODO_CPP §19, §27–§28).
//
// The taxonomy is deliberately three-way and never collapsed:
//
//   engine failure        -> bdb::Error (this module): an owned, structured
//                            engine/bridge error crossed back over the ABI;
//                            always the std::unexpected path.
//   construction failure  -> bdb::TypeError (bumbledb.types): a pre-engine
//                            checked-value construction failure (interval,
//                            allen_mask) that never reaches the bridge.
//   abandonment           -> data, not an error: a write callback that
//                            decides "no" produces WriteOutcome::Abandoned
//                            (bumbledb.db), on the SUCCESS path.
//
// Reflection-free dialect code, part of BOTH graphs.
export module bumbledb.error;

import std;
import bumbledb.foreign;
import bumbledb.foreign.raii;

export namespace bdb {

/// The engine error families, one constant per C kind (TODO_CPP §27). The
/// C++ enum mirrors bdb_error_kind value-for-value; the static_asserts
/// below pin the correspondence so header drift breaks this compile.
enum class ErrorKind : std::uint8_t {
    Schema,
    SchemaMismatch,
    FormatMismatch,
    AlreadyInitialized,
    NotInitialized,
    EnvironmentLocked,
    StoreKindMismatch,
    DescriptorMissing,
    ReadersFull,
    Validation,
    CommitRejected,
    CommitSync,
    GenerationMoved,
    ForeignSnapshot,
    ForeignPrepared,
    FactShape,
    ClosedRelationWrite,
    FreshExhausted,
    BulkLoad,
    Param,
    MeasureOfRay,
    CapacityRayMeasure,
    FixpointBudgetExceeded,
    Overflow,
    ResultBytesOverflow,
    Corruption,
    Io,
    Lmdb,
    Panic,
};

/// A violated statement's form (the C++ image of bdb_statement_kind,
/// value-for-value).
enum class StatementKind : std::uint8_t {
    Functionality,
    Containment,
    Capacity,
};

/// A containment citation's violated side; None for key and capacity
/// citations (the C++ image of bdb_violation_direction, value-for-value).
enum class ViolationDirection : std::uint8_t {
    None,
    SourceUnsatisfied,
    TargetRequired,
};

/// A capacity measure: u128 as two u64 words.
struct Measure {
    std::uint64_t lo;
    std::uint64_t hi;
};

/// One rendered violation of a rejected commit — ownership-closed (the
/// spelling is copied out of the error).
struct Violation {
    std::uint16_t statement;
    StatementKind kind;
    std::string spelling;
    ViolationDirection direction;
    std::optional<Measure> measure;
};

/// The GenerationMoved payload: the witnessed and current generations.
struct GenerationMoved {
    std::uint64_t witnessed;
    std::uint64_t current;
};

} // namespace bdb

namespace bdb::detail {

// The value-for-value mirror pins (TODO_CPP §27's tag-table discipline —
// the C++ enum is the fourth spelling; drift breaks the compile here).
template<class Mirror, class Wire>
consteval auto mirrors(Mirror mirror, Wire wire) -> bool {
    return std::to_underlying(mirror)
        == static_cast<std::underlying_type_t<Mirror>>(wire);
}

namespace abi = bdb::foreign;

static_assert(mirrors(ErrorKind::Schema, abi::bdb_error_kind::BDB_ERROR_KIND_SCHEMA));
static_assert(mirrors(ErrorKind::SchemaMismatch, abi::bdb_error_kind::BDB_ERROR_KIND_SCHEMA_MISMATCH));
static_assert(mirrors(ErrorKind::FormatMismatch, abi::bdb_error_kind::BDB_ERROR_KIND_FORMAT_MISMATCH));
static_assert(mirrors(ErrorKind::AlreadyInitialized, abi::bdb_error_kind::BDB_ERROR_KIND_ALREADY_INITIALIZED));
static_assert(mirrors(ErrorKind::NotInitialized, abi::bdb_error_kind::BDB_ERROR_KIND_NOT_INITIALIZED));
static_assert(mirrors(ErrorKind::EnvironmentLocked, abi::bdb_error_kind::BDB_ERROR_KIND_ENVIRONMENT_LOCKED));
static_assert(mirrors(ErrorKind::StoreKindMismatch, abi::bdb_error_kind::BDB_ERROR_KIND_STORE_KIND_MISMATCH));
static_assert(mirrors(ErrorKind::DescriptorMissing, abi::bdb_error_kind::BDB_ERROR_KIND_DESCRIPTOR_MISSING));
static_assert(mirrors(ErrorKind::ReadersFull, abi::bdb_error_kind::BDB_ERROR_KIND_READERS_FULL));
static_assert(mirrors(ErrorKind::Validation, abi::bdb_error_kind::BDB_ERROR_KIND_VALIDATION));
static_assert(mirrors(ErrorKind::CommitRejected, abi::bdb_error_kind::BDB_ERROR_KIND_COMMIT_REJECTED));
static_assert(mirrors(ErrorKind::CommitSync, abi::bdb_error_kind::BDB_ERROR_KIND_COMMIT_SYNC));
static_assert(mirrors(ErrorKind::GenerationMoved, abi::bdb_error_kind::BDB_ERROR_KIND_GENERATION_MOVED));
static_assert(mirrors(ErrorKind::ForeignSnapshot, abi::bdb_error_kind::BDB_ERROR_KIND_FOREIGN_SNAPSHOT));
static_assert(mirrors(ErrorKind::ForeignPrepared, abi::bdb_error_kind::BDB_ERROR_KIND_FOREIGN_PREPARED));
static_assert(mirrors(ErrorKind::FactShape, abi::bdb_error_kind::BDB_ERROR_KIND_FACT_SHAPE));
static_assert(mirrors(ErrorKind::ClosedRelationWrite, abi::bdb_error_kind::BDB_ERROR_KIND_CLOSED_RELATION_WRITE));
static_assert(mirrors(ErrorKind::FreshExhausted, abi::bdb_error_kind::BDB_ERROR_KIND_FRESH_EXHAUSTED));
static_assert(mirrors(ErrorKind::BulkLoad, abi::bdb_error_kind::BDB_ERROR_KIND_BULK_LOAD));
static_assert(mirrors(ErrorKind::Param, abi::bdb_error_kind::BDB_ERROR_KIND_PARAM));
static_assert(mirrors(ErrorKind::MeasureOfRay, abi::bdb_error_kind::BDB_ERROR_KIND_MEASURE_OF_RAY));
static_assert(mirrors(ErrorKind::CapacityRayMeasure, abi::bdb_error_kind::BDB_ERROR_KIND_CAPACITY_RAY_MEASURE));
static_assert(mirrors(ErrorKind::FixpointBudgetExceeded, abi::bdb_error_kind::BDB_ERROR_KIND_FIXPOINT_BUDGET_EXCEEDED));
static_assert(mirrors(ErrorKind::Overflow, abi::bdb_error_kind::BDB_ERROR_KIND_OVERFLOW));
static_assert(mirrors(ErrorKind::ResultBytesOverflow, abi::bdb_error_kind::BDB_ERROR_KIND_RESULT_BYTES_OVERFLOW));
static_assert(mirrors(ErrorKind::Corruption, abi::bdb_error_kind::BDB_ERROR_KIND_CORRUPTION));
static_assert(mirrors(ErrorKind::Io, abi::bdb_error_kind::BDB_ERROR_KIND_IO));
static_assert(mirrors(ErrorKind::Lmdb, abi::bdb_error_kind::BDB_ERROR_KIND_LMDB));
static_assert(mirrors(ErrorKind::Panic, abi::bdb_error_kind::BDB_ERROR_KIND_PANIC));

static_assert(mirrors(StatementKind::Functionality, abi::bdb_statement_kind::BDB_STATEMENT_KIND_FUNCTIONALITY));
static_assert(mirrors(StatementKind::Containment, abi::bdb_statement_kind::BDB_STATEMENT_KIND_CONTAINMENT));
static_assert(mirrors(StatementKind::Capacity, abi::bdb_statement_kind::BDB_STATEMENT_KIND_CAPACITY));

static_assert(mirrors(ViolationDirection::None, abi::bdb_violation_direction::BDB_VIOLATION_DIRECTION_NONE));
static_assert(mirrors(ViolationDirection::SourceUnsatisfied, abi::bdb_violation_direction::BDB_VIOLATION_DIRECTION_SOURCE_UNSATISFIED));
static_assert(mirrors(ViolationDirection::TargetRequired, abi::bdb_violation_direction::BDB_VIOLATION_DIRECTION_TARGET_REQUIRED));

} // namespace bdb::detail

export namespace bdb {

/// An owned, structured engine error (TODO_CPP §27): move-only RAII over
/// the bridge's opaque error payload. Formatting is a separate cold
/// operation (message/violations copy out); kind() is the hot accessor.
/// The moved-from Error is inert — never read a moved-from Error.
class Error {
    foreign::error_handle handle_;

public:
    /// The bridge lane: adopts an owned error handle. Application code
    /// never constructs Errors; Db and its capabilities do.
    explicit Error(foreign::error_handle handle)
        : handle_{std::move(handle)} {}

    [[nodiscard]] auto kind() const -> ErrorKind {
        // Value-for-value mirror; pinned by the static_asserts above.
        return static_cast<ErrorKind>(std::to_underlying(handle_.kind()));
    }

    /// The rendered message (cold path; copies).
    [[nodiscard]] auto message() const -> std::string {
        return handle_.message();
    }

    /// The GenerationMoved payload; nullopt for every other kind.
    [[nodiscard]] auto generation_moved() const
        -> std::optional<GenerationMoved> {
        return handle_.generation_moved().transform(
            [](foreign::generation_moved_payload payload)
                -> GenerationMoved {
                return GenerationMoved{
                    .witnessed = payload.witnessed,
                    .current = payload.current,
                };
            });
    }

    /// The complete rendered violation set of a CommitRejected error
    /// (empty for every other kind). Ownership-closed copies.
    [[nodiscard]] auto violations() const -> std::vector<Violation> {
        auto rendered = std::vector<Violation>{};
        auto const count = handle_.violation_count();
        rendered.reserve(count);
        for (auto index = std::size_t{0}; index != count; ++index) {
            auto copy = handle_.violation(index);
            if (!copy.has_value()) {
                break;
            }
            rendered.push_back(Violation{
                .statement = copy->statement,
                .kind = static_cast<StatementKind>(
                    std::to_underlying(copy->kind)),
                .spelling = std::move(copy->spelling),
                .direction = static_cast<ViolationDirection>(
                    std::to_underlying(copy->direction)),
                .measure = copy->has_measure
                    ? std::optional{Measure{
                          .lo = copy->measure_lo, .hi = copy->measure_hi}}
                    : std::nullopt,
            });
        }
        return rendered;
    }
};

} // namespace bdb
