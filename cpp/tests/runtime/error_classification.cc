import std;
import bumbledb;
import bumbledb_foreign;

namespace {

namespace abi = bdb::foreign;

struct CaseResult {
	std::string name;
	bool passed;
};

struct KindTransience {
	bdb::ErrorKind kind;
	bool transient;
};

inline constexpr auto classification = std::array<KindTransience, 29>{{
    {.kind = bdb::ErrorKind::Schema, .transient = false},
    {.kind = bdb::ErrorKind::SchemaMismatch, .transient = false},
    {.kind = bdb::ErrorKind::FormatMismatch, .transient = false},
    {.kind = bdb::ErrorKind::AlreadyInitialized, .transient = false},
    {.kind = bdb::ErrorKind::NotInitialized, .transient = false},
    {.kind = bdb::ErrorKind::EnvironmentLocked, .transient = false},
    {.kind = bdb::ErrorKind::StoreKindMismatch, .transient = false},
    {.kind = bdb::ErrorKind::DescriptorMissing, .transient = false},
    {.kind = bdb::ErrorKind::ReadersFull, .transient = true},
    {.kind = bdb::ErrorKind::Validation, .transient = false},
    {.kind = bdb::ErrorKind::CommitRejected, .transient = false},
    {.kind = bdb::ErrorKind::CommitSync, .transient = false},
    {.kind = bdb::ErrorKind::GenerationMoved, .transient = true},
    {.kind = bdb::ErrorKind::ForeignSnapshot, .transient = false},
    {.kind = bdb::ErrorKind::ForeignPrepared, .transient = false},
    {.kind = bdb::ErrorKind::FactShape, .transient = false},
    {.kind = bdb::ErrorKind::ClosedRelationWrite, .transient = false},
    {.kind = bdb::ErrorKind::FreshExhausted, .transient = false},
    {.kind = bdb::ErrorKind::BulkLoad, .transient = false},
    {.kind = bdb::ErrorKind::Param, .transient = false},
    {.kind = bdb::ErrorKind::MeasureOfRay, .transient = false},
    {.kind = bdb::ErrorKind::CapacityRayMeasure, .transient = false},
    {.kind = bdb::ErrorKind::FixpointBudgetExceeded, .transient = false},
    {.kind = bdb::ErrorKind::Overflow, .transient = false},
    {.kind = bdb::ErrorKind::ResultBytesOverflow, .transient = false},
    {.kind = bdb::ErrorKind::Corruption, .transient = false},
    {.kind = bdb::ErrorKind::Io, .transient = false},
    {.kind = bdb::ErrorKind::Lmdb, .transient = false},
    {.kind = bdb::ErrorKind::Panic, .transient = false},
}};

static_assert(
    [] {
	    auto seen = std::array<bool, classification.size()>{};
	    for (auto const entry : classification) {
		    seen.at(std::to_underlying(entry.kind)) = true;
	    }
	    return std::ranges::all_of(seen, std::identity{});
    }(),
    "the classification table names every ErrorKind exactly once");

static_assert(std::ranges::count_if(classification,
                                    [](KindTransience entry) {
	                                    return entry.transient;
                                    }) == 2,
              "exactly GenerationMoved and ReadersFull are transient");

[[nodiscard]] constexpr auto expected_transient(bdb::ErrorKind kind) -> bool {
	return std::ranges::find(classification, kind, &KindTransience::kind)->transient;
}

static_assert(expected_transient(bdb::ErrorKind::GenerationMoved));
static_assert(expected_transient(bdb::ErrorKind::ReadersFull));
static_assert(!expected_transient(bdb::ErrorKind::EnvironmentLocked));
static_assert(!expected_transient(bdb::ErrorKind::CommitSync));
static_assert(!expected_transient(bdb::ErrorKind::Panic));

struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

struct ShortServiceRow {
	std::uint64_t id;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

using UnitDecision = bdb::WriteDecision<std::monostate, std::monostate>;
using UnitResult = std::expected<UnitDecision, bdb::Error>;
using UnitCommitted = bdb::Committed<std::monostate>;

[[nodiscard]] auto make_uptime_spec() -> abi::owned_schema_spec {
	auto relations = std::vector<abi::owned_relation>{};
	relations.push_back(abi::owned_relation{
	    .name = "Service",
	    .fields =
	        {
	            abi::owned_field{
	                .name = "id",
	                .value_type = abi::scalar_type(abi::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_U64),
	                .newtype = "Service.id",
	                .fresh = true,
	            },
	            abi::owned_field{
	                .name = "name",
	                .value_type = abi::scalar_type(abi::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_STRING),
	                .newtype = std::nullopt,
	                .fresh = false,
	            },
	        },
	});
	relations.push_back(abi::owned_relation{
	    .name = "Outage",
	    .fields =
	        {
	            abi::owned_field{
	                .name = "service",
	                .value_type = abi::scalar_type(abi::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_U64),
	                .newtype = "Service.id",
	                .fresh = false,
	            },
	            abi::owned_field{
	                .name = "window",
	                .value_type = abi::interval_type(abi::bdb_interval_element::BDB_INTERVAL_ELEMENT_I64),
	                .newtype = std::nullopt,
	                .fresh = false,
	            },
	        },
	});

	auto statements = std::vector<abi::owned_statement>{};
	statements.push_back(abi::owned_containment{
	    .source = abi::owned_side{.relation = "Outage", .projection = {"service"}},
	    .target = abi::owned_side{.relation = "Service", .projection = {"id"}},
	    .bidirectional = false,
	});
	return abi::owned_schema_spec{std::move(relations), std::move(statements)};
}

[[nodiscard]] auto make_store_dir() -> std::expected<std::filesystem::path, std::string> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::unexpected(std::format("temp_directory_path: {}", code.message()));
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-error-classification-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::unexpected(std::format("create_directories: {}", code.message()));
	}
	return dir;
}

[[nodiscard]] auto insert_service_body(std::string name) {
	return [name = std::move(name)](bdb::WriteTx& tx) -> UnitResult {
		return tx.alloc(Service.id)
		    .and_then([&](std::uint64_t id) {
			    return tx.insert(Service, ServiceRow{.id = id, .name = name});
		    })
		    .transform([](bool) -> UnitDecision {
			    return bdb::commit();
		    });
	};
}

[[nodiscard]] auto classified(std::string_view label, bdb::Error const& error, bdb::ErrorKind kind) -> CaseResult {
	return CaseResult{
	    .name = std::format("{} classifies is_transient() == {}", label, expected_transient(kind)),
	    .passed = error.kind() == kind && error.is_transient() == expected_transient(kind),
	};
}

[[nodiscard]] auto generation_moved_cases(bdb::Db& db) -> std::vector<CaseResult> {
	auto results = std::vector<CaseResult>{};
	auto const observed = db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return db.write_from(snap, insert_service_body("first"))
		    .and_then([&](auto first) -> std::expected<bool, bdb::Error> {
			    if (!std::holds_alternative<UnitCommitted>(first)) {
				    return false;
			    }
			    auto second = db.write_from(snap, insert_service_body("second"));
			    if (second.has_value()) {
				    return false;
			    }
			    results.push_back(classified("a stale write_from (GenerationMoved)", second.error(), bdb::ErrorKind::GenerationMoved));
			    results.push_back(CaseResult{
			        .name = "bulk_committed() is nullopt on a non-BulkLoad error",
			        .passed = !second.error().bulk_committed().has_value(),
			    });
			    return true;
		    });
	});
	if (!observed.has_value() || !*observed) {
		results.push_back(CaseResult{.name = "a stale write_from provokes GenerationMoved", .passed = false});
	}
	return results;
}

[[nodiscard]] auto fact_shape_case(bdb::Db& db) -> CaseResult {
	auto written = db.write([](bdb::WriteTx& tx) -> UnitResult {
		return tx.insert(Service, ShortServiceRow{.id = 1}).transform([](bool) -> UnitDecision {
			return bdb::commit();
		});
	});
	if (written.has_value()) {
		return CaseResult{.name = "an arity violation provokes FactShape", .passed = false};
	}
	return classified("an arity violation (FactShape)", written.error(), bdb::ErrorKind::FactShape);
}

[[nodiscard]] auto commit_rejected_case(bdb::Db& db) -> CaseResult {
	auto written = db.write([](bdb::WriteTx& tx) -> UnitResult {
		auto const window = bdb::interval<std::int64_t>::literal(0, 10);
		return tx.insert(Outage, OutageRow{.service = 999999, .window = window}).transform([](bool) -> UnitDecision {
			return bdb::commit();
		});
	});
	if (written.has_value()) {
		return CaseResult{.name = "an orphan containment provokes CommitRejected", .passed = false};
	}
	return classified("an orphan containment (CommitRejected)", written.error(), bdb::ErrorKind::CommitRejected);
}

[[nodiscard]] auto environment_locked_case(bdb::Db& db) -> CaseResult {
	auto inner_result = std::optional<CaseResult>{};
	auto outer = db.write([&](bdb::WriteTx&) -> UnitResult {
		auto inner = db.write([](bdb::WriteTx&) -> UnitResult {
			return bdb::commit();
		});
		if (!inner.has_value()) {
			inner_result = classified("a re-entrant write (EnvironmentLocked)", inner.error(), bdb::ErrorKind::EnvironmentLocked);
		}
		return bdb::commit();
	});
	if (!outer.has_value() || !inner_result.has_value()) {
		return CaseResult{.name = "a re-entrant write provokes EnvironmentLocked", .passed = false};
	}
	return *inner_result;
}

[[nodiscard]] auto run_cases() -> std::vector<CaseResult> {
	auto results = std::vector<CaseResult>{};
	auto const spec = make_uptime_spec();

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = dir.error(), .passed = false});
		return results;
	}
	auto opened = bdb::Db::ephemeral(dir->native(), spec.view());
	if (!opened.has_value()) {
		results.push_back(CaseResult{
		    .name = std::format("ephemeral create failed: {}", opened.error().message()),
		    .passed = false,
		});
		return results;
	}
	auto& db = *opened;

	results.append_range(generation_moved_cases(db));
	results.push_back(fact_shape_case(db));
	results.push_back(commit_rejected_case(db));
	results.push_back(environment_locked_case(db));

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	return results;
}

}

auto main() -> int {
	auto failures = std::size_t{0};
	for (auto const& result : run_cases()) {
		if (result.passed) {
			std::println("pass: {}", result.name);
		} else {
			std::println("FAIL: {}", result.name);
			++failures;
		}
	}
	return failures == 0 ? 0 : 1;
}
