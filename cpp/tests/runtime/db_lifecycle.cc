import std;
import bumbledb;
import bumbledb_foreign;

namespace {

namespace abi = bdb::foreign;

struct CaseResult {
	std::string name;
	bool passed;
};

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
	statements.push_back(abi::owned_fd{
	    .relation = "Outage",
	    .projection = {"service", "window"},
	});
	return abi::owned_schema_spec{std::move(relations), std::move(statements)};
}

[[nodiscard]] auto make_store_dir(std::string_view label) -> std::expected<std::filesystem::path, std::string> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::unexpected(std::format("temp_directory_path: {}", code.message()));
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-runtime-{}-{:08x}{:08x}", label, device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::unexpected(std::format("create_directories: {}", code.message()));
	}
	return dir;
}

[[nodiscard]] auto is_lower_hex(std::string_view text) -> bool {
	return std::ranges::all_of(text, [](char character) {
		return (character >= '0' && character <= '9') || (character >= 'a' && character <= 'f');
	});
}

[[nodiscard]] auto insert_outage_body(std::uint64_t service_id, bdb::interval<std::int64_t> window) {
	return [service_id, window](bdb::WriteTx& tx) -> UnitResult {
		return tx.insert(Outage, OutageRow{.service = service_id, .window = window}).transform([](bool) -> UnitDecision {
			return bdb::commit();
		});
	};
}

[[nodiscard]] auto fingerprint_case(bdb::Db const& db) -> CaseResult {
	auto const fingerprint = db.fingerprint();
	return CaseResult{
	    .name = "fingerprint() is 64 lowercase hex chars",
	    .passed = fingerprint.has_value() && fingerprint->size() == 64 && is_lower_hex(*fingerprint),
	};
}

[[nodiscard]] auto insert_service_case(bdb::Db& db, std::vector<CaseResult>& results) -> std::optional<std::uint64_t> {
	using IdDecision = bdb::WriteDecision<std::uint64_t, std::monostate>;
	using IdResult = std::expected<IdDecision, bdb::Error>;

	auto written = db.write([](bdb::WriteTx& tx) -> IdResult {
		return tx.alloc(Service.id).and_then([&tx](std::uint64_t id) -> IdResult {
			return tx.insert(Service, ServiceRow{.id = id, .name = std::string{"search"}}).transform([id](bool changed) -> IdDecision {
				return changed ? IdDecision{bdb::commit(id)} : IdDecision{bdb::abandon(std::monostate{})};
			});
		});
	});
	auto const committed = written.has_value() && std::holds_alternative<bdb::Committed<std::uint64_t>>(*written);
	results.push_back(CaseResult{
	    .name = "write: alloc(Service.id) + reflected insert commits",
	    .passed = committed,
	});
	if (!committed) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

[[nodiscard]] auto read_contains_case(bdb::Db const& db, std::uint64_t service_id) -> CaseResult {
	auto const seen = db.read([service_id](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return snap.contains(Service, ServiceRow{.id = service_id, .name = std::string{"search"}});
	});
	return CaseResult{
	    .name = "read: snapshot contains the committed Service row",
	    .passed = seen.has_value() && *seen,
	};
}

[[nodiscard]] auto scan_case(bdb::Db const& db, std::uint64_t service_id) -> CaseResult {
	struct ScanFacts {
		std::size_t rows;
		std::size_t arity;
		bool id_matches;
		bool name_matches;
	};

	auto const scanned = db.read([service_id](bdb::Snapshot& snap) -> std::expected<ScanFacts, bdb::Error> {
		return snap.scan(Service).transform([&](bdb::RowSet rows) {
			auto const id_cell = rows.cell({.row = 0, .column = 0});
			auto const name_cell = rows.cell({.row = 0, .column = 1});
			return ScanFacts{
			    .rows = rows.len(),
			    .arity = rows.arity(0),
			    .id_matches = id_cell.has_value() && std::holds_alternative<std::uint64_t>(*id_cell) &&
			                  std::get<std::uint64_t>(*id_cell) == service_id,
			    .name_matches = name_cell.has_value() && std::holds_alternative<std::string>(*name_cell) &&
			                    std::get<std::string>(*name_cell) == "search",
			};
		});
	});
	return CaseResult{
	    .name = "read: scan crosses once and cells decode to bdb::Value",
	    .passed = scanned.has_value() && scanned->rows == 1 && scanned->arity == 2 && scanned->id_matches && scanned->name_matches,
	};
}

[[nodiscard]] auto write_from_case(bdb::Db& db, std::uint64_t service_id) -> CaseResult {
	auto const window = bdb::interval<std::int64_t>::literal(0, 100);
	auto const nested = db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return db.write_from(snap, insert_outage_body(service_id, window)).transform([](auto outcome) {
			return std::holds_alternative<UnitCommitted>(outcome);
		});
	});
	auto const stored = db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return snap.contains(Outage, OutageRow{.service = service_id, .window = window});
	});
	return CaseResult{
	    .name = "write_from inside read commits an interval-bearing row (§18)",
	    .passed = nested.has_value() && *nested && stored.has_value() && *stored,
	};
}

[[nodiscard]] auto generation_moved_case(bdb::Db& db, std::uint64_t service_id) -> CaseResult {
	auto const first_window = bdb::interval<std::int64_t>::literal(200, 300);
	auto const second_window = bdb::interval<std::int64_t>::literal(400, 500);
	auto const observed = db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return db.write_from(snap, insert_outage_body(service_id, first_window))
		    .and_then([&](auto first) -> std::expected<bool, bdb::Error> {
			    if (!std::holds_alternative<UnitCommitted>(first)) {
				    return false;
			    }
			    auto second = db.write_from(snap, insert_outage_body(service_id, second_window));
			    if (second.has_value()) {
				    return false;
			    }
			    auto const payload = second.error().generation_moved();
			    return second.error().kind() == bdb::ErrorKind::GenerationMoved && payload.has_value() &&
			           payload->current > payload->witnessed;
		    });
	});
	return CaseResult{
	    .name = "write_from after a moved generation is typed GenerationMoved",
	    .passed = observed.has_value() && *observed,
	};
}

[[nodiscard]] auto abort_on_error_case(bdb::Db& db) -> std::vector<CaseResult> {
	auto ghost_id = std::optional<std::uint64_t>{};
	auto aborted = db.write([&ghost_id](bdb::WriteTx& tx) -> UnitResult {
		return tx.alloc(Service.id)
		    .and_then([&](std::uint64_t id) -> std::expected<bool, bdb::Error> {
			    ghost_id = id;
			    return tx.insert(Service, ServiceRow{.id = id, .name = std::string{"ghost"}});
		    })
		    .and_then([&tx](bool) -> UnitResult {
			    return tx.insert(Service, ShortServiceRow{.id = 1}).transform([](bool) -> UnitDecision {
				    return bdb::commit();
			    });
		    });
	});
	auto const fact_shape = !aborted.has_value() && aborted.error().kind() == bdb::ErrorKind::FactShape;
	auto const ghost_absent =
	    ghost_id.has_value() &&
	    db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		      return snap.contains(Service, ServiceRow{.id = *ghost_id, .name = std::string{"ghost"}}).transform([](bool present) {
			      return !present;
		      });
	      }).value_or(false);
	return {
	    CaseResult{
	        .name = "a deliberate arity violation is ErrorKind::FactShape",
	        .passed = fact_shape,
	    },
	    CaseResult{
	        .name = "a callback-local failure aborts the whole delta (§36)",
	        .passed = ghost_absent,
	    },
	};
}

[[nodiscard]] auto abandon_case(bdb::Db& db) -> std::vector<CaseResult> {
	using Decision = bdb::WriteDecision<std::monostate, std::string>;
	auto maybe_id = std::optional<std::uint64_t>{};
	auto outcome = db.write([&maybe_id](bdb::WriteTx& tx) -> std::expected<Decision, bdb::Error> {
		return tx.alloc(Service.id)
		    .and_then([&](std::uint64_t id) -> std::expected<bool, bdb::Error> {
			    maybe_id = id;
			    return tx.insert(Service, ServiceRow{.id = id, .name = std::string{"maybe"}});
		    })
		    .transform([](bool) -> Decision {
			    return bdb::abandon(std::string{"not today"});
		    });
	});
	auto const carried = outcome.has_value() && std::holds_alternative<bdb::Abandoned<std::string>>(*outcome) &&
	                     std::get<bdb::Abandoned<std::string>>(*outcome).value == "not today";
	auto const absent =
	    maybe_id.has_value() &&
	    db.read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		      return snap.contains(Service, ServiceRow{.id = *maybe_id, .name = std::string{"maybe"}}).transform([](bool present) {
			      return !present;
		      });
	      }).value_or(false);
	return {
	    CaseResult{
	        .name = "abandon-as-data round-trips its payload (§19)",
	        .passed = carried,
	    },
	    CaseResult{
	        .name = "an abandoned write commits nothing (§36)",
	        .passed = absent,
	    },
	};
}

[[nodiscard]] auto commit_rejection_case(bdb::Db& db) -> CaseResult {
	auto const orphan_service = std::uint64_t{999999};
	auto const window = bdb::interval<std::int64_t>::literal(1, 2);
	auto rejected = db.write(insert_outage_body(orphan_service, window));
	auto const shaped = !rejected.has_value() && rejected.error().kind() == bdb::ErrorKind::CommitRejected;
	if (!shaped) {
		return CaseResult{
		    .name = "a commit rejection carries the violation set",
		    .passed = false,
		};
	}
	auto const violations = rejected.error().violations();
	auto const complete = !violations.empty() && std::ranges::any_of(violations, [](bdb::Violation const& cited) {
		return cited.kind == bdb::StatementKind::Containment && !cited.spelling.empty();
	}) && !rejected.error().message().empty();
	return CaseResult{
	    .name = "a commit rejection carries the violation set",
	    .passed = complete,
	};
}

[[nodiscard]] auto reentrant_write_case(bdb::Db& db) -> CaseResult {
	auto inner_kind = std::optional<bdb::ErrorKind>{};
	auto outer = db.write([&](bdb::WriteTx&) -> UnitResult {
		auto inner = db.write([](bdb::WriteTx&) -> UnitResult {
			return bdb::commit();
		});
		if (!inner.has_value()) {
			inner_kind = inner.error().kind();
		}
		return bdb::commit();
	});
	return CaseResult{
	    .name = "a re-entrant write is typed EnvironmentLocked (§17)",
	    .passed = outer.has_value() && inner_kind.has_value() && *inner_kind == bdb::ErrorKind::EnvironmentLocked,
	};
}

[[nodiscard]] auto tx_lanes_case(bdb::Db& db) -> CaseResult {
	auto seen_after_insert = false;
	auto seen_after_remove = true;
	auto removal_changed = false;
	auto outcome = db.write([&](bdb::WriteTx& tx) -> UnitResult {
		return tx.alloc(Service.id)
		    .and_then([&](std::uint64_t id) -> std::expected<bool, bdb::Error> {
			    auto const row = ServiceRow{.id = id, .name = std::string{"temp"}};
			    return tx.insert(Service, row)
			        .and_then([&](bool) {
				        return tx.contains(Service, row);
			        })
			        .and_then([&](bool seen) {
				        seen_after_insert = seen;
				        return tx.remove(Service, row);
			        })
			        .and_then([&](bool changed) {
				        removal_changed = changed;
				        return tx.contains(Service, row);
			        });
		    })
		    .transform([&](bool seen) -> UnitDecision {
			    seen_after_remove = seen;
			    return bdb::commit();
		    });
	});
	return CaseResult{
	    .name = "tx contains/remove judge the final-state view (§17)",
	    .passed = outcome.has_value() && seen_after_insert && removal_changed && !seen_after_remove,
	};
}

[[nodiscard]] auto move_case(bdb::Db db) -> CaseResult {
	auto target = std::move(db);
	return CaseResult{
	    .name = "moving Db leaves the source inert and valid (§36)",
	    .passed = !db.alive() && target.alive() && target.fingerprint().has_value(),
	};
}

[[nodiscard]] auto durable_case(abi::owned_schema_spec const& spec) -> std::vector<CaseResult> {
	auto const dir = make_store_dir("durable");
	if (!dir.has_value()) {
		return {CaseResult{.name = dir.error(), .passed = false}};
	}
	auto stored_id = std::optional<std::uint64_t>{};
	{
		auto created = bdb::Db::create(dir->native(), spec.view());
		if (!created.has_value()) {
			return {CaseResult{
			    .name = std::format("durable create failed: {}", created.error().message()),
			    .passed = false,
			}};
		}
		auto results = std::vector<CaseResult>{};
		stored_id = insert_service_case(*created, results);
		if (!stored_id.has_value()) {
			return results;
		}
	}

	auto reopened = bdb::Db::open(dir->native(), spec.view());
	auto const survived =
	    reopened.has_value() && reopened
	                                ->read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		                                return snap.contains(Service, ServiceRow{.id = *stored_id, .name = std::string{"search"}});
	                                })
	                                .value_or(false);
	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	return {CaseResult{
	    .name = "RAII destruction releases the store; open() sees the commit",
	    .passed = survived,
	}};
}

[[nodiscard]] auto run_cases() -> std::vector<CaseResult> {
	auto results = std::vector<CaseResult>{};
	auto const spec = make_uptime_spec();

	auto const dir = make_store_dir("ephemeral");
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

	results.push_back(fingerprint_case(db));
	auto const service_id = insert_service_case(db, results);
	if (service_id.has_value()) {
		results.push_back(read_contains_case(db, *service_id));
		results.push_back(scan_case(db, *service_id));
		results.push_back(write_from_case(db, *service_id));
		results.push_back(generation_moved_case(db, *service_id));
	}
	results.append_range(abort_on_error_case(db));
	results.append_range(abandon_case(db));
	results.push_back(commit_rejection_case(db));
	results.push_back(reentrant_write_case(db));
	results.push_back(tx_lanes_case(db));
	results.push_back(move_case(std::move(*opened)));

	results.append_range(durable_case(spec));

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
