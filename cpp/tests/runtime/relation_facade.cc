// The relation reflector's coordinate facade (TODO_CPP §6–§7, §39): the
// Service/Outage rows of the first vertical slice, proven for ordinals,
// names (through the coord name hooks), fresh flags, structural kinds, and
// NTTP-friendliness of coordinates. GCC-only: imports reflective modules,
// excluded from the lint graph.
import std;
import bumbledb;

// TODO_CPP §39 — the first-slice rows, spelled exactly as specified.
struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

// The facade is compile-time semantic data; everything below is proven
// during constant evaluation, then re-reported at runtime so ctest shows
// the cases. A coordinate's physical type rides its value_type; its
// IDENTITY rides the whole coord type (two fields are two types).
static_assert(std::same_as<decltype(Service.id)::value_type, std::uint64_t>);
static_assert(std::same_as<decltype(Service.name)::value_type, std::string>);
static_assert(std::same_as<decltype(Outage.window)::value_type, bdb::interval<std::int64_t>>);
static_assert(!std::same_as<decltype(Service.id), decltype(Outage.service)>);

static_assert(Service.id.ordinal == 0);
static_assert(Service.name.ordinal == 1);
static_assert(Outage.service.ordinal == 0);
static_assert(Outage.window.ordinal == 1);

static_assert(Service.id.fresh);
static_assert(!Service.name.fresh);
static_assert(!Outage.service.fresh);
static_assert(!Outage.window.fresh);

static_assert(Service.id.kind == bdb::value_kind::u64);
static_assert(Service.name.kind == bdb::value_kind::string);
static_assert(Outage.service.kind == bdb::value_kind::u64);
static_assert(Outage.window.kind == bdb::value_kind::interval_i64);

static_assert(Service.id.relation() == "Service");
static_assert(Service.id.field() == "id");
static_assert(Service.name.field() == "name");
static_assert(Outage.window.relation() == "Outage");
static_assert(Outage.window.field() == "window");

// The whole classification vocabulary, through one row.
struct KindsRow {
	bool flag;
	std::uint64_t count;
	std::int64_t delta;
	std::string label;
	bdb::bytes<16> digest;
	bdb::interval<std::uint64_t> span;
	bdb::interval<std::int64_t> window;
};

inline constexpr auto Kinds = bdb::relation<"Kinds", KindsRow>;

static_assert(Kinds.flag.kind == bdb::value_kind::boolean);
static_assert(Kinds.count.kind == bdb::value_kind::u64);
static_assert(Kinds.delta.kind == bdb::value_kind::i64);
static_assert(Kinds.label.kind == bdb::value_kind::string);
static_assert(Kinds.digest.kind == bdb::value_kind::fixed_bytes);
static_assert(Kinds.digest.fixed_len == 16);
static_assert(Kinds.span.kind == bdb::value_kind::interval_u64);
static_assert(Kinds.window.kind == bdb::value_kind::interval_i64);
static_assert(Kinds.window.ordinal == 6);

// Coordinates are structural NTTP-friendly literal types (TODO_CPP §6):
// a coord travels as a template argument.
template<auto Coordinate>
struct coordinate_probe {
	static constexpr auto ordinal = Coordinate.ordinal;
	static constexpr auto fresh = Coordinate.fresh;
};

static_assert(coordinate_probe<Service.id>::ordinal == 0);
static_assert(coordinate_probe<Service.id>::fresh);
static_assert(coordinate_probe<Outage.window>::ordinal == 1);

namespace {

struct CaseResult {
	std::string_view name;
	bool passed;
};

[[nodiscard]] auto run_cases() -> std::array<CaseResult, 4> {
	return std::array{
	    CaseResult{
	        .name = "facade ordinals follow declaration order",
	        .passed = Service.id.ordinal == 0 && Service.name.ordinal == 1 && Outage.service.ordinal == 0 && Outage.window.ordinal == 1,
	    },
	    CaseResult{
	        .name = "name hooks render relation and field names",
	        .passed = Service.id.relation() == "Service" && Service.id.field() == "id" && Outage.window.relation() == "Outage" &&
	                  Outage.window.field() == "window",
	    },
	    CaseResult{
	        .name = "fresh flag marks exactly the annotated u64",
	        .passed = Service.id.fresh && !Service.name.fresh && !Outage.service.fresh && !Outage.window.fresh,
	    },
	    CaseResult{
	        .name = "kinds classify per the closed value vocabulary",
	        .passed = Service.id.kind == bdb::value_kind::u64 && Service.name.kind == bdb::value_kind::string &&
	                  Outage.window.kind == bdb::value_kind::interval_i64 && Kinds.digest.fixed_len == 16,
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
