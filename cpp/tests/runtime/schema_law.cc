import std;
import bumbledb;

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

inline constexpr auto outage_key = bdb::key(Outage.service, Outage.window);

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)),

                                                     outage_key);

static_assert(Uptime.relation_count == 2);
static_assert(Uptime.statement_count == 2);
static_assert(Uptime.coordinate_count == 4);
static_assert(Uptime.schema_name.view() == "Uptime");

static_assert(Uptime.relations.Service.id.fresh);
static_assert(Uptime.relations.Outage.service.relation() == "Outage");
static_assert(Uptime.relation_table[0].name.view() == "Service");
static_assert(Uptime.relation_table[1].name.view() == "Outage");
static_assert(Uptime.relation_table[0].field_count == 2);
static_assert(Uptime.relation_table[0].fields[0].fresh);
static_assert(!Uptime.relation_table[0].fields[1].fresh);
static_assert(Uptime.relation_table[1].fields[1].kind == bdb::value_kind::interval_i64);

static_assert(Uptime.statements[0].form == bdb::statement_form::containment);
static_assert(!Uptime.statements[0].bidirectional);
static_assert(Uptime.statements[0].source.relation.view() == "Outage");
static_assert(Uptime.statements[0].source.fields[0].view() == "service");
static_assert(Uptime.statements[0].target.relation.view() == "Service");
static_assert(Uptime.statements[0].target.fields[0].view() == "id");
static_assert(Uptime.statements[1].form == bdb::statement_form::key);
static_assert(Uptime.statements[1].source.relation.view() == "Outage");
static_assert(Uptime.statements[1].source.width == 2);
static_assert(Uptime.statements[1].source.fields[0].view() == "service");
static_assert(Uptime.statements[1].source.fields[1].view() == "window");

[[nodiscard]] consteval auto class_is(std::optional<bdb::coord_ref> entry, std::string_view relation, std::string_view field) -> bool {
	return entry.has_value() && entry->relation.view() == relation && entry->field.view() == field;
}

static_assert(class_is(Uptime.class_of(Service.id), "Service", "id"));
static_assert(class_is(Uptime.class_of(Outage.service), "Service", "id"));
static_assert(!Uptime.class_of(Service.name).has_value());
static_assert(!Uptime.class_of(Outage.window).has_value());

template<auto Schema>
struct schema_probe {
	static constexpr auto relations = Schema.relation_count;
};

static_assert(schema_probe<Uptime>::relations == 2);

template<auto Law>
struct law_probe {
	static constexpr auto width = Law.width;
};

static_assert(law_probe<outage_key>::width == 2);

using OutageKeyPattern = decltype(outage_key)::pattern;
static_assert(std::same_as<decltype(std::declval<OutageKeyPattern>().service), std::uint64_t>);
static_assert(std::same_as<decltype(std::declval<OutageKeyPattern>().window), bdb::interval<std::int64_t>>);

using ServiceFresh = bdb::fresh_pattern_of<std::remove_cvref_t<decltype(Service)>>;
static_assert(std::same_as<decltype(std::declval<ServiceFresh>().id), std::uint64_t>);
static_assert(bdb::fresh_field_count<std::remove_cvref_t<decltype(Service)>>() == 1);
static_assert(bdb::fresh_field_count<std::remove_cvref_t<decltype(Outage)>>() == 0);

struct PoolRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t supply;
};

struct DeviceRow {
	std::uint64_t pool;
	std::uint64_t watts;
	bdb::interval<std::uint64_t> uptime;
};

inline constexpr auto Pool = bdb::relation<"Pool", PoolRow>;
inline constexpr auto Device = bdb::relation<"Device", DeviceRow>;

inline constexpr auto Power = bdb::schema<"Power">(
    Pool, Device,

    bdb::contained(bdb::on(Device.pool), bdb::on(Pool.id)),

    bdb::capacity(bdb::on(Pool.id), bdb::weigh(Device.watts), bdb::within(std::uint64_t{0}, bdb::ref(Pool.supply)), bdb::on(Device.pool)));

static_assert(Power.statements[1].form == bdb::statement_form::capacity);
static_assert(Power.statements[1].target.relation.view() == "Pool");
static_assert(Power.statements[1].source.relation.view() == "Device");
static_assert(Power.statements[1].weight == bdb::weight_form::field);
static_assert(Power.statements[1].weight_field.view() == "watts");
static_assert(Power.statements[1].window.form == bdb::window_form::range);
static_assert(Power.statements[1].window.lo.form == bdb::bound_form::lit);
static_assert(Power.statements[1].window.lo.lit == 0);
static_assert(Power.statements[1].window.hi.form == bdb::bound_form::field);
static_assert(Power.statements[1].window.hi.field.view() == "supply");

static_assert(class_is(Power.class_of(Device.pool), "Pool", "id"));
static_assert(!Power.class_of(Device.watts).has_value());
static_assert(!Power.class_of(Pool.supply).has_value());

struct BookingRow {
	std::uint64_t room;
	bdb::interval<std::uint64_t> span;
};

struct RoomRow {
	[[= bdb::fresh]] std::uint64_t id;
};

inline constexpr auto Booking = bdb::relation<"Booking", BookingRow>;
inline constexpr auto Room = bdb::relation<"Room", RoomRow>;

inline constexpr auto Rooms = bdb::schema<"Rooms">(Room, Booking,

                                                   bdb::mirrors(bdb::on(Booking.room), bdb::on(Room.id)),

                                                   bdb::capacity(bdb::on(Room.id), bdb::weigh(bdb::duration(Booking.span)),
                                                                 bdb::within(std::uint64_t{0}, std::uint64_t{720}), bdb::on(Booking.room)));

static_assert(Rooms.statements[0].form == bdb::statement_form::containment);
static_assert(Rooms.statements[0].bidirectional);
static_assert(Rooms.statements[1].weight == bdb::weight_form::duration_field);
static_assert(Rooms.statements[1].weight_field.view() == "span");
static_assert(Rooms.statements[1].window.hi.lit == 720);

inline constexpr auto WideVocab = bdb::closed<"WideVocab", "A", "B", "C", "D", "E", "F", "G", "H", "I">();
static_assert(WideVocab.data.handle_count == 9);

static_assert(bdb::within(std::uint64_t{3}).data.form == bdb::window_form::exact);
static_assert(bdb::within(std::uint64_t{3}).data.lo.lit == 3);

namespace {

struct CaseResult {
	std::string_view name;
	bool passed;
};

[[nodiscard]] auto run_cases() -> std::array<CaseResult, 4> {
	return std::array{
	    CaseResult{
	        .name = "schema flattens relations/statements in written order",
	        .passed = Uptime.relation_table[0].name.view() == "Service" && Uptime.statements[0].form == bdb::statement_form::containment &&
	                  Uptime.statements[1].form == bdb::statement_form::key,
	    },
	    CaseResult{
	        .name = "class laws: generator-first, paired, bare (§10)",
	        .passed = class_is(Uptime.class_of(Outage.service), "Service", "id") && !Uptime.class_of(Outage.window).has_value() &&
	                  !Uptime.class_of(Service.name).has_value(),
	    },
	    CaseResult{
	        .name = "capacity flattens weigh/within/ref positionally (§9)",
	        .passed = Power.statements[1].weight_field.view() == "watts" && Power.statements[1].window.hi.field.view() == "supply",
	    },
	    CaseResult{
	        .name = "mirrors stays one bidirectional statement",
	        .passed = Rooms.statements[0].bidirectional && Rooms.statement_count == 2,
	    },
	};
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
