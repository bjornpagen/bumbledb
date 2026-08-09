import std;
import bumbledb;

struct RoomRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::interval<std::int64_t> span;
};

struct BookingRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t room;
	bdb::interval<std::int64_t> booked;
};

inline constexpr auto Room = bdb::relation<"Room", RoomRow>;
inline constexpr auto Booking = bdb::relation<"Booking", BookingRow>;

inline constexpr auto Rooms = bdb::schema<"Rooms">(Room, Booking,

                                                   bdb::contained(bdb::on(Booking.room), bdb::on(Room.id)),

                                                   bdb::key(Booking.room, Booking.booked),

                                                   bdb::capacity(bdb::on(Room.id), bdb::weigh(bdb::duration(Booking.booked)),
                                                                 bdb::within(0, bdb::duration(Room.span)), bdb::on(Booking.room)));

inline constexpr auto Booked = bdb::query(Rooms).rule([](auto r) consteval {
	auto vars = r.vars(Booking);
	return r
	    .match(Booking,
	           {
	               .id = vars.id,
	               .room = vars.room,
	               .booked = vars.booked,
	           })
	    .find(
	        {
	            .room = vars.room,
	        },
	        bdb::sum<"total">(r.duration(vars.booked)));
});

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

[[nodiscard]] auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
	for (auto const line_range : std::views::split(fixtures, '\n')) {
		auto const line = std::string_view{line_range};
		if (!line.starts_with(recipe)) {
			continue;
		}
		auto const rest = line.substr(recipe.size());
		if (!rest.starts_with(' ')) {
			continue;
		}
		auto hex = rest.substr(1);
		while (!hex.empty() && (hex.back() == '\r' || hex.back() == ' ')) {
			hex.remove_suffix(1);
		}
		if (hex.size() != 64) {
			return std::nullopt;
		}
		return std::string{hex};
	}
	return std::nullopt;
}

[[nodiscard]] auto slurp(std::string_view path) -> std::optional<std::string> {
	auto stream = std::ifstream{std::string{path}, std::ios::binary | std::ios::ate};
	if (!stream) {
		return std::nullopt;
	}
	auto const size = stream.tellg();
	if (size < 0) {
		return std::nullopt;
	}
	auto text = std::string(static_cast<std::size_t>(size), '\0');
	stream.seekg(0);
	stream.read(text.data(), size);
	if (!stream) {
		return std::nullopt;
	}
	return text;
}

[[nodiscard]] auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r32-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t room_a;
	std::uint64_t room_b;
};

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const book = [&](std::uint64_t room, std::int64_t lo, std::int64_t hi) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Booking.id).and_then([&](std::uint64_t minted) {
				return tx.insert(Booking, BookingRow{.id = minted, .room = room, .booked = *bdb::interval<std::int64_t>::make(lo, hi)});
			});
		};
		auto rows_land = tx.alloc(Room.id)
		                     .and_then([&](std::uint64_t minted) {
			                     ids.room_a = minted;
			                     return tx.insert(Room, RoomRow{.id = minted, .span = bdb::interval<std::int64_t>::literal(0, 100)});
		                     })
		                     .and_then([&](bool) {
			                     return tx.alloc(Room.id);
		                     })
		                     .and_then([&](std::uint64_t minted) {
			                     ids.room_b = minted;
			                     return tx.insert(Room, RoomRow{.id = minted, .span = bdb::interval<std::int64_t>::literal(0, 50)});
		                     })
		                     .and_then([&](bool) {
			                     return book(ids.room_a, 0, 30);
		                     })
		                     .and_then([&](bool) {
			                     return book(ids.room_a, 50, 80);
		                     })
		                     .and_then([&](bool) {
			                     return book(ids.room_b, 10, 20);
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(ids);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

[[nodiscard]] auto book_one(bdb::Db& db, std::uint64_t room, std::int64_t lo, std::int64_t hi)
    -> std::expected<bdb::WriteOutcome<std::monostate, std::monostate>, bdb::Error> {
	return db.write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Booking.id).and_then([&](std::uint64_t minted) {
			return tx.insert(Booking, BookingRow{.id = minted, .room = room, .booked = *bdb::interval<std::int64_t>::make(lo, hi)});
		});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
}

[[nodiscard]] auto rejected_citing(std::expected<bdb::WriteOutcome<std::monostate, std::monostate>, bdb::Error> const& written, bdb::StatementKind kind)
    -> bool {
	if (written.has_value() || written.error().kind() != bdb::ErrorKind::CommitRejected) {
		return false;
	}
	auto const violations = written.error().violations();
	return !violations.empty() && std::ranges::any_of(violations, [&](bdb::Violation const& violation) {
		return violation.kind == kind;
	});
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r32") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r32 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r32 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Rooms);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits Rooms", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r32 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto booked = db->prepare<Booked>();
	results.push_back(CaseResult{
	    .name = "booked (sum of durations per room) prepares through the "
	            "engine validator",
	    .passed = booked.has_value(),
	});
	if (!booked.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "rooms and the in-span bookings commit",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	using BookedRow = bdb::row_of<Booked>;
	auto totals = db->execute(*booked, {}).transform([](bdb::Answers<Booked> answers) {
		auto rows = std::vector<BookedRow>{};
		for (auto const& row : answers.rows()) {
			rows.push_back(row);
		}
		std::ranges::sort(rows, bdb::by(&BookedRow::room));
		return rows;
	});
	auto totals_pass = totals.has_value() && totals->size() == 2;
	if (totals_pass) {
		auto const& a = (*totals)[0].room == ids->room_a ? (*totals)[0] : (*totals)[1];
		auto const& b = (*totals)[0].room == ids->room_b ? (*totals)[0] : (*totals)[1];
		totals_pass = a.room == ids->room_a && a.total == 60 && b.room == ids->room_b && b.total == 10;
	}
	results.push_back(CaseResult{
	    .name = "booked answers {(A, 60), (B, 10)}",
	    .passed = totals_pass,
	});

	auto const double_booked = book_one(*db, ids->room_a, 20, 40);
	results.push_back(CaseResult{
	    .name = "a double-booking is commit-rejected citing the pointwise "
	            "key",
	    .passed = rejected_citing(double_booked, bdb::StatementKind::Functionality),
	});

	auto const over_capacity = book_one(*db, ids->room_a, 200, 400);
	results.push_back(CaseResult{
	    .name = "an over-capacity booking is commit-rejected citing the "
	            "capacity law",
	    .passed = rejected_citing(over_capacity, bdb::StatementKind::Capacity),
	});

	auto const topped = book_one(*db, ids->room_a, 80, 100);
	results.push_back(CaseResult{
	    .name = "an in-budget booking still lands after the rejections",
	    .passed = topped.has_value() && std::holds_alternative<bdb::Committed<std::monostate>>(*topped),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r32_calendar_capacity <fixtures-file>");
		return 1;
	}

	auto results = std::vector<CaseResult>{};
	run_cases(std::string_view{arguments[1]}, results);

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
