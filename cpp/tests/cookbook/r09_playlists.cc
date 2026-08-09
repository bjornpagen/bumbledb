// Cookbook recipe 9 — Ordered collections (ts/COOKBOOK.md §9): the
// linked-list verdict. Order is a value: the idiomatic ordered collection
// is an interval partition spelled as a TRIPLE — the entity, the extent
// as a 0..1 child (empty lists exist, empty intervals do not), and the
// unit-slot sidecar (`bdb::interval<std::uint64_t, 1>` — the width is the
// TYPE: a wrong-width value is unrepresentable). The mixed-width
// multi-column mirrors ([playlist, span] ~ [playlist, slot]) is the
// tiling law: slots tile the extent exactly.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r09 <64-hex>"); playingAt (positional access IS membership) prepares
// AND answers the recipe's own semantics; a slot outside the extent is
// commit-rejected (the tiling mirrors); and the host owns the sort — the
// SDK ships the comparator (`bdb::by` / `bdb::desc`, keys as data).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct PlaylistRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

// The extent: a 0..1 child, because empty playlists exist and empty
// intervals do not — presence of the child IS nonemptiness.
struct ExtentRow {
	std::uint64_t playlist;
	bdb::interval<std::uint64_t> span;
};

// The unit slot: position p occupies [p, p+1) — the width is the type.
struct SlotRow {
	std::uint64_t playlist;
	bdb::interval<std::uint64_t, 1> slot;
	std::string track;
};

inline constexpr auto Playlist = bdb::relation<"Playlist", PlaylistRow>;
inline constexpr auto Extent = bdb::relation<"Extent", ExtentRow>;
inline constexpr auto Slot = bdb::relation<"Slot", SlotRow>;

inline constexpr auto Playlists = bdb::schema<"Playlists">(
    Playlist, Extent, Slot,

    bdb::contained(bdb::on(Extent.playlist), bdb::on(Playlist.id)), bdb::contained(bdb::on(Slot.playlist), bdb::on(Playlist.id)),

    // 0..1 extent per playlist.
    bdb::key(Extent.playlist),

    // The exact target key (recipe 26's note).
    bdb::key(Extent.playlist, Extent.span),

    // One occupant per position.
    bdb::key(Slot.playlist, Slot.slot),

    // Slots tile the span exactly — the mixed-width multi-column mirrors.
    bdb::mirrors(bdb::on(Extent.playlist, Extent.span), bdb::on(Slot.playlist, Slot.slot)));

// Positional access is membership — "what plays at position ?pos".
inline constexpr auto PlayingAt = bdb::query(Playlists).rule([](auto r) consteval {
	auto vars = r.vars(Slot);
	return r
	    .match(Slot,
	           {
	               .playlist = bdb::param<"list">(),
	               .slot = vars.slot,
	               .track = vars.track,
	           })
	    .where(bdb::point_in(bdb::param<"pos">(), vars.slot))
	    .find({
	        .track = vars.track,
	    });
});

// The width IS the type: a wrong-width unit slot is unrepresentable.
static_assert(bdb::interval<std::uint64_t, 1>::make(4, 5).has_value());
static_assert(!bdb::interval<std::uint64_t, 1>::make(4, 6).has_value());

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
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

auto slurp(std::string_view path) -> std::optional<std::string> {
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

auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r09-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

/// One playlist with extent [0,2) tiled by two unit slots: position 0
/// plays "a", position 1 plays "b" — the tiling mirrors demands the
/// extent and its slots land in ONE commit.
auto seed(bdb::Db& db) -> std::optional<std::uint64_t> {
	using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto list = std::uint64_t{0};
		auto rows_land =
		    tx.alloc(Playlist.id)
		        .and_then([&](std::uint64_t minted) {
			        list = minted;
			        return tx.insert(Playlist, PlaylistRow{.id = list, .name = std::string{"road trip"}});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Extent, ExtentRow{.playlist = list, .span = bdb::interval<std::uint64_t>::literal(0, 2)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(
			            Slot, SlotRow{.playlist = list, .slot = bdb::interval<std::uint64_t, 1>::literal(0, 1), .track = std::string{"a"}});
		        })
		        .and_then([&](bool) {
			        return tx.insert(
			            Slot, SlotRow{.playlist = list, .slot = bdb::interval<std::uint64_t, 1>::literal(1, 2), .track = std::string{"b"}});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(list);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<std::uint64_t>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r09") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r09 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r09 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Playlists);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Playlists", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r09 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const list = seed(*db);
	results.push_back(CaseResult{
	    .name = "the triple commits: entity, extent, and the unit slots "
	            "that tile it",
	    .passed = list.has_value(),
	});
	if (!list.has_value()) {
		return;
	}

	// A slot OUTSIDE the extent breaks the tiling mirrors.
	auto untiled = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed =
		    tx.insert(Slot, SlotRow{.playlist = *list, .slot = bdb::interval<std::uint64_t, 1>::literal(2, 3), .track = std::string{"c"}});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a slot outside the extent is commit-rejected (slots "
	            "tile the span exactly)",
	    .passed = !untiled.has_value() && untiled.error().kind() == bdb::ErrorKind::CommitRejected && !untiled.error().violations().empty(),
	});

	auto playing_at = db->prepare<PlayingAt>();
	results.push_back(CaseResult{
	    .name = "playingAt prepares through the engine validator",
	    .passed = playing_at.has_value(),
	});
	if (!playing_at.has_value()) {
		return;
	}

	// Positional access is membership: position 1 is [1,2) — track "b".
	auto at_one = db->execute(*playing_at, {.list = *list, .pos = std::uint64_t{1}});
	results.push_back(CaseResult{
	    .name = "playingAt(pos: 1) answers {b}",
	    .passed = at_one.has_value() && at_one->size() == 1 && at_one->rows().front().track == "b",
	});
	auto at_zero = db->execute(*playing_at, {.list = *list, .pos = std::uint64_t{0}});
	results.push_back(CaseResult{
	    .name = "playingAt(pos: 0) answers {a}",
	    .passed = at_zero.has_value() && at_zero->size() == 1 && at_zero->rows().front().track == "a",
	});
	auto past_end = db->execute(*playing_at, {.list = *list, .pos = std::uint64_t{5}});
	results.push_back(CaseResult{
	    .name = "playingAt(pos: 5) answers the empty set (past the "
	            "extent)",
	    .passed = past_end.has_value() && past_end->size() == 0,
	});

	// Answers are SETS — the host sorts them, and the SDK ships the
	// comparator: sort keys as data, a bare member pointer ascending,
	// `bdb::desc(...)` the flip; intervals order by (start, end).
	struct Entry {
		bdb::interval<std::uint64_t> slot;
		std::string track;
	};

	auto in_play_order = std::vector<Entry>{
	    Entry{.slot = bdb::interval<std::uint64_t>::literal(1, 2), .track = std::string{"b"}},
	    Entry{.slot = bdb::interval<std::uint64_t>::literal(0, 1), .track = std::string{"a"}},
	};
	std::ranges::sort(in_play_order, bdb::by(&Entry::slot, &Entry::track));
	auto reversed = in_play_order;
	std::ranges::sort(reversed, bdb::by(bdb::desc(&Entry::slot)));
	results.push_back(CaseResult{
	    .name = "the host sorts with the shipped comparator (bdb::by "
	            "ascends, bdb::desc flips)",
	    .passed = in_play_order.front().track == "a" && in_play_order.back().track == "b" && reversed.front().track == "b",
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r09_playlists <fixtures-file>");
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
