// Row marshalling (TODO_CPP §24): an OutageRow lowers to [U64,
// IntervalI64] with the exact payloads, in declaration order; string and
// bytes cells are borrowed views into the row (valid for the call). This
// test consumes the ABI value STRUCTS through bdb::foreign — never
// handles or functions. GCC-only: imports reflective modules, excluded
// from the lint graph.
import std;
import bumbledb;
import bumbledb_foreign;

struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

struct TaggedRow {
    bool live;
    bdb::bytes<4> tag;
};

static_assert(bdb::row_field_count<OutageRow> == 2);
static_assert(bdb::row_field_count<ServiceRow> == 2);

namespace {

namespace abi = bdb::foreign;

struct CaseResult {
    std::string_view name;
    bool passed;
};

auto check_outage_lowers_to_u64_interval_i64() -> CaseResult {
    auto const outage = OutageRow{
        .service = 7,
        .window = bdb::interval<std::int64_t>::literal(-5, 9),
    };
    auto const cells = bdb::marshal_row(outage);
    auto const& service_cell = std::get<0>(cells);
    auto const& window_cell = std::get<1>(cells);
    return CaseResult{
        .name = "OutageRow lowers to [U64(7), IntervalI64(-5, 9)]",
        .passed = service_cell.kind
                == abi::bdb_value_kind::BDB_VALUE_KIND_U64
            && service_cell.u64_value == 7
            && window_cell.kind
                == abi::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64
            && window_cell.interval_i64_start == -5
            && window_cell.interval_i64_end == 9,
    };
}

auto check_service_string_cell_borrows_the_row() -> CaseResult {
    auto const service = ServiceRow{.id = 1, .name = "search"};
    auto const cells = bdb::marshal_row(service);
    auto const& id_cell = std::get<0>(cells);
    auto const& name_cell = std::get<1>(cells);
    auto const borrowed = std::string_view{
        std::bit_cast<char const*>(name_cell.string_value.data),
        name_cell.string_value.len,
    };
    auto const borrows_row = name_cell.string_value.data
        == std::bit_cast<std::uint8_t const*>(service.name.data());
    return CaseResult{
        .name = "ServiceRow lowers to [U64, String] borrowing row storage",
        .passed = id_cell.kind == abi::bdb_value_kind::BDB_VALUE_KIND_U64
            && id_cell.u64_value == 1
            && name_cell.kind
                == abi::bdb_value_kind::BDB_VALUE_KIND_STRING
            && borrowed == service.name && borrows_row,
    };
}

auto check_bool_and_bytes_cells() -> CaseResult {
    auto const tagged = TaggedRow{
        .live = true,
        .tag = bdb::bytes<4>{
            std::byte{0xde}, std::byte{0xad},
            std::byte{0xbe}, std::byte{0xef}},
    };
    auto const cells = bdb::marshal_row(tagged);
    auto const& live_cell = std::get<0>(cells);
    auto const& tag_cell = std::get<1>(cells);
    auto const borrows_row = tag_cell.bytes_value.data
        == std::bit_cast<std::uint8_t const*>(tagged.tag.data());
    return CaseResult{
        .name = "TaggedRow lowers to [Bool, FixedBytes(4)] borrowing row",
        .passed = live_cell.kind
                == abi::bdb_value_kind::BDB_VALUE_KIND_BOOL
            && live_cell.bool_value
            && tag_cell.kind
                == abi::bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES
            && tag_cell.bytes_value.len == 4 && borrows_row,
    };
}

} // namespace

auto main() -> int {
    auto const results = std::array{
        check_outage_lowers_to_u64_interval_i64(),
        check_service_string_cell_borrows_the_row(),
        check_bool_and_bytes_cells(),
    };

    auto failures = std::size_t{0};
    for (auto const& result : results) {
        if (result.passed) {
            std::println("pass: {}", result.name);
        } else {
            std::println("FAIL: {}", result.name);
            ++failures;
        }
    }
    return failures == 0 ? 0 : 1;
}
