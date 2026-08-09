// Cookbook recipe 12 — Entity-component (ts/COOKBOOK.md §12): the 0..1
// idiom (recipe 3) at scale. Components are sidecar relations; an entity
// has a component iff the fact exists; a new component kind is a new
// relation, not a wider fact. The archetype rule is one containment:
// every Renderable has a Transform (and, through it, an Entity —
// containment composes, and the class composes with it).
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r12 <64-hex>"); physics (the component-intersection join) prepares
// AND answers the recipe's own semantics; a Renderable without its
// Transform is commit-rejected (the archetype containment).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct EntityRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct TransformRow {
	std::uint64_t entity;
	std::int64_t x;
	std::int64_t y;
};

struct VelocityRow {
	std::uint64_t entity;
	std::int64_t dx;
	std::int64_t dy;
};

struct RenderableRow {
	std::uint64_t entity;
	std::string mesh;
};

inline constexpr auto Entity = bdb::relation<"Entity", EntityRow>;
inline constexpr auto Transform = bdb::relation<"Transform", TransformRow>;
inline constexpr auto Velocity = bdb::relation<"Velocity", VelocityRow>;
inline constexpr auto Renderable = bdb::relation<"Renderable", RenderableRow>;

inline constexpr auto Ecs =
    bdb::schema<"Ecs">(Entity, Transform, Velocity, Renderable,

                       // Each component 0..1 per entity.
                       bdb::key(Transform.entity), bdb::contained(bdb::on(Transform.entity), bdb::on(Entity.id)), bdb::key(Velocity.entity),
                       bdb::contained(bdb::on(Velocity.entity), bdb::on(Entity.id)), bdb::key(Renderable.entity),

                       // An archetype rule is one containment: every Renderable has a
                       // Transform (and, through it, an Entity — containment composes, and
                       // the class composes with it: every `entity` column lands in
                       // "Entity.id").
                       bdb::contained(bdb::on(Renderable.entity), bdb::on(Transform.entity)));

// The physics join is the component intersection.
inline constexpr auto Physics = bdb::query(Ecs).rule([](auto r) consteval {
	auto t = r.vars(Transform);
	auto w = r.vars(Velocity);
	return r
	    .match(Transform,
	           {
	               .entity = t.entity,
	               .x = t.x,
	               .y = t.y,
	           })
	    .match(Velocity,
	           {
	               .entity = t.entity,
	               .dx = w.dx,
	               .dy = w.dy,
	           })
	    .find({
	        .entity = t.entity,
	        .x = t.x,
	        .y = t.y,
	        .dx = w.dx,
	        .dy = w.dy,
	    });
});

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
	auto const dir = root / std::format("bumbledb-cookbook-r12-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t player;
	std::uint64_t rock;
};

/// The player carries all three components; the rock only a Transform —
/// an entity has a component iff the fact exists.
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const mint = [&](std::uint64_t& out) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Entity.id).transform([&](std::uint64_t minted) {
				out = minted;
				return true;
			});
		};
		auto rows_land = mint(ids.player)
		                     .and_then([&](bool) {
			                     return mint(ids.rock);
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Entity, EntityRow{.id = ids.player, .name = std::string{"player"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Entity, EntityRow{.id = ids.rock, .name = std::string{"rock"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Transform, TransformRow{.entity = ids.player, .x = 1, .y = 2});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Velocity, VelocityRow{.entity = ids.player, .dx = 3, .dy = 4});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Renderable, RenderableRow{.entity = ids.player, .mesh = std::string{"cube"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Transform, TransformRow{.entity = ids.rock, .x = 5, .y = 6});
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

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r12") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r12 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r12 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Ecs);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Ecs", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r12 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "entities and their component sidecars commit",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// A Renderable without its Transform violates the archetype
	// containment.
	auto bare = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Entity.id)
		                  .and_then([&](std::uint64_t ghost) {
			                  return tx.insert(Entity, EntityRow{.id = ghost, .name = std::string{"ghost"}}).transform([&](bool) {
				                  return ghost;
			                  });
		                  })
		                  .and_then([&](std::uint64_t ghost) {
			                  return tx.insert(Renderable, RenderableRow{.entity = ghost, .mesh = std::string{"wisp"}});
		                  });
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a Renderable without its Transform is commit-rejected "
	            "(the archetype containment)",
	    .passed = !bare.has_value() && bare.error().kind() == bdb::ErrorKind::CommitRejected && !bare.error().violations().empty(),
	});

	auto physics = db->prepare<Physics>();
	results.push_back(CaseResult{
	    .name = "physics prepares through the engine validator",
	    .passed = physics.has_value(),
	});
	if (!physics.has_value()) {
		return;
	}

	// The component intersection: only the player has BOTH Transform and
	// Velocity.
	auto moving = db->execute(*physics, {});
	results.push_back(CaseResult{
	    .name = "physics answers exactly the player's (x, y, dx, dy)",
	    .passed = moving.has_value() && moving->size() == 1 && moving->rows().front().entity == ids->player &&
	              moving->rows().front().x == 1 && moving->rows().front().y == 2 && moving->rows().front().dx == 3 &&
	              moving->rows().front().dy == 4,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r12_ecs <fixtures-file>");
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
