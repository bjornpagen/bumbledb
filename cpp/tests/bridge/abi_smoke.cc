// Smoke test of the wired bridge (TODO_CPP §32/§35): create an ephemeral
// store from a minimal one-relation SchemaSpec view (Service: fresh id u64,
// name str — the bridge's own test theory, statement-free), read the
// fingerprint back (64 lowercase hex chars), destroy. This is the ONE test
// file that consumes the raw ABI views directly through bdb::foreign; every
// raw handle stays inside an immediately-scoped helper and never escapes.
import std;
import bumbledb_foreign;

namespace {

namespace abi = bdb::foreign;

struct CaseResult {
	std::string name;
	bool passed;
};

// Owned bytes for a borrowed ABI text view (the ABI speaks uint8_t, the
// host speaks char; the copy keeps the conversion explicit and the storage
// alive for the whole call).
[[nodiscard]] auto bytes_of(std::string_view text) -> std::vector<std::uint8_t> {
	auto bytes = std::vector<std::uint8_t>{};
	bytes.reserve(text.size());
	std::ranges::transform(text, std::back_inserter(bytes), [](char character) {
		return static_cast<std::uint8_t>(character);
	});
	return bytes;
}

[[nodiscard]] auto view_of(std::span<std::uint8_t const> bytes) -> abi::bdb_string_view {
	return abi::bdb_string_view{.data = bytes.data(), .len = bytes.size()};
}

[[nodiscard]] auto absent_view() -> abi::bdb_string_view {
	return abi::bdb_string_view{.data = nullptr, .len = 0};
}

[[nodiscard]] auto scalar_type(abi::bdb_value_type_kind kind) -> abi::bdb_value_type {
	return abi::bdb_value_type{
	    .kind = kind,
	    .fixed_len = 0,
	    .element = abi::bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
	    .has_width = false,
	    .width = 0,
	};
}

// Renders and frees an owned ABI error (null-safe; the message view dies
// with the error, so it is copied before the destroy).
[[nodiscard]] auto consume_error(abi::bdb_error* error) -> std::string {
	if (error == nullptr) {
		return "(no error payload)";
	}
	auto message = absent_view();
	auto text = std::string{};
	if (abi::bdb_error_get_message(error, &message) == abi::bdb_status::BDB_STATUS_OK) {
		for (auto const byte : std::span{message.data, message.len}) {
			text.push_back(static_cast<char>(byte));
		}
	}
	static_cast<void>(abi::bdb_error_destroy(error));
	return text;
}

// The whole raw lifecycle — create → fingerprint → destroy — in one scope:
// no ABI handle survives this function.
[[nodiscard]] auto read_fingerprint(std::string_view store_path) -> std::expected<std::array<std::uint8_t, 64>, std::string> {
	auto const path_bytes = bytes_of(store_path);
	auto const service_bytes = bytes_of("Service");
	auto const id_bytes = bytes_of("id");
	auto const name_bytes = bytes_of("name");

	auto const fields = std::array{
	    abi::bdb_field_spec{
	        .name = view_of(id_bytes),
	        .value_type = scalar_type(abi::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_U64),
	        .newtype = absent_view(),
	        .fresh = true,
	    },
	    abi::bdb_field_spec{
	        .name = view_of(name_bytes),
	        .value_type = scalar_type(abi::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_STRING),
	        .newtype = absent_view(),
	        .fresh = false,
	    },
	};
	auto const relation = abi::bdb_relation_spec{
	    .name = view_of(service_bytes),
	    .fields = fields.data(),
	    .field_count = fields.size(),
	    .closed = nullptr,
	};
	auto const spec = abi::bdb_schema_spec{
	    .relations = &relation,
	    .relation_count = std::size_t{1},
	    .statements = nullptr,
	    .statement_count = 0,
	};

	abi::bdb_db* database = nullptr;
	abi::bdb_error* error = nullptr;
	auto const created = abi::bdb_db_ephemeral(view_of(path_bytes), &spec, &database, &error);
	if (created != abi::bdb_status::BDB_STATUS_OK) {
		return std::unexpected(std::format("bdb_db_ephemeral failed: {}", consume_error(error)));
	}

	auto fingerprint = abi::bdb_fingerprint{};
	auto const read = abi::bdb_db_fingerprint(database, &fingerprint, &error);
	if (read != abi::bdb_status::BDB_STATUS_OK) {
		auto const detail = consume_error(error);
		static_cast<void>(abi::bdb_db_destroy(database));
		return std::unexpected(std::format("bdb_db_fingerprint failed: {}", detail));
	}

	if (abi::bdb_db_destroy(database) != abi::bdb_status::BDB_STATUS_OK) {
		return std::unexpected(std::string{"bdb_db_destroy failed"});
	}
	return std::to_array(fingerprint.hex);
}

[[nodiscard]] auto make_store_dir() -> std::expected<std::filesystem::path, std::string> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::unexpected(std::format("temp_directory_path: {}", code.message()));
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-abi-smoke-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::unexpected(std::format("create_directories: {}", code.message()));
	}
	return dir;
}

[[nodiscard]] auto is_lower_hex(std::span<std::uint8_t const> chars) -> bool {
	return std::ranges::all_of(chars, [](std::uint8_t character) {
		return (character >= '0' && character <= '9') || (character >= 'a' && character <= 'f');
	});
}

[[nodiscard]] auto run_cases() -> std::vector<CaseResult> {
	auto const dir = make_store_dir();
	if (!dir) {
		return {CaseResult{
		    .name = std::format("store dir exists ({})", dir.error()),
		    .passed = false,
		}};
	}
	auto const fingerprint = read_fingerprint(dir->string());
	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	if (!fingerprint) {
		return {CaseResult{
		    .name = std::format("create/fingerprint/destroy succeed ({})", fingerprint.error()),
		    .passed = false,
		}};
	}
	return {
	    CaseResult{
	        .name = "ephemeral create → fingerprint → destroy round-trips",
	        .passed = true,
	    },
	    CaseResult{
	        .name = "fingerprint is 64 lowercase hex chars",
	        .passed = is_lower_hex(*fingerprint),
	    },
	    CaseResult{
	        .name = "fingerprint is not the zero string",
	        .passed = std::ranges::any_of(*fingerprint,
	                                      [](std::uint8_t character) {
		                                      return character != '0';
	                                      }),
	    },
	};
}

} // namespace

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
