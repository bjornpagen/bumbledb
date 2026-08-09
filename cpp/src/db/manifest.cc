/**
 * The coordinate -> wire-id resolution table. Resolution is by NAME:
 * relation id = declaration index, field id = the coordinate's reflected
 * ordinal (lowering.md §1.1/§1.11); keyed reads address statements by
 * their fingerprint-pinned MATERIALIZED index (lowering.md §2).
 */
export module bumbledb:manifest;

import std;
import :spec;
import :key;
import :schema;
import :error;
import :answers;
import bumbledb_foreign;

namespace bdb::detail {

/**
 * One materialized statement's structural identity. Only keys resolve
 * gets, but every statement holds its slot so the ids stay aligned with
 * the engine's materialized order.
 */
struct StatementRow {
	bool is_key;
	std::string relation;
	std::vector<std::string> projection;
};

/**
 * Statement rows are present on the schema lane only — the pre-schema
 * raw-spec lane resolves no keyed reads.
 */
struct Manifest {
	std::vector<std::string> relation_names;
	std::vector<StatementRow> statement_rows;

	[[nodiscard]] auto resolve(std::string_view relation) const -> std::optional<std::uint32_t> {
		for (auto const& [index, name] : std::views::enumerate(relation_names)) {
			if (name == relation) {
				return static_cast<std::uint32_t>(index);
			}
		}
		return std::nullopt;
	}

	/**
	 * The key statement with exactly this structural identity — resolved
	 * by content, never by a nominal type.
	 */
	[[nodiscard]] auto resolve_key(std::string_view relation, std::span<std::string_view const> projection) const
	    -> std::optional<std::uint16_t> {
		for (auto const& [index, row] : std::views::enumerate(statement_rows)) {
			if (!row.is_key || row.relation != relation || row.projection.size() != projection.size()) {
				continue;
			}
			if (std::ranges::equal(row.projection, projection)) {
				return static_cast<std::uint16_t>(index);
			}
		}
		return std::nullopt;
	}

	/**
	 * The relation's PRIMARY key: its first key statement in
	 * materialized order (a fresh-bearing relation's fresh field —
	 * lowering.md §5.3).
	 */
	[[nodiscard]] auto resolve_primary(std::string_view relation) const -> std::optional<std::uint16_t> {
		for (auto const& [index, row] : std::views::enumerate(statement_rows)) {
			if (row.is_key && row.relation == relation) {
				return static_cast<std::uint16_t>(index);
			}
		}
		return std::nullopt;
	}
};

/**
 * Resolves or dies: a coordinate/facade naming a relation outside the
 * admitted spec is an impossible programmer state (the facade and the
 * spec are both compile-time artifacts of the same declaration set), not
 * a recoverable input.
 */
[[nodiscard]] auto resolved_relation(Manifest const& manifest, std::string_view relation) -> std::uint32_t {
	auto const id = manifest.resolve(relation);
	contract_assert(id.has_value());
	return *id;
}

/**
 * The facade's relation name, read off its first coordinate (every
 * coordinate of one facade carries the same relation name).
 */
template<class Facade>
[[nodiscard]] constexpr auto facade_relation_name(Facade const& facade) -> std::string_view {
	auto const& [... coords] = facade;
	static_assert(sizeof...(coords) > 0);
	return [](auto const& first, auto const&...) {
		return first.relation();
	}(coords...);
}

[[nodiscard]] auto lift(foreign::error_handle handle) -> Error {
	return Error{std::move(handle)};
}

/**
 * A keyed read's outcome: a hit is one owned row; a miss is genuine
 * absence (the ABI wrote no row set).
 */
[[nodiscard]] auto lift_row(foreign::row_set_handle handle) -> std::optional<RowSet> {
	auto rows = RowSet{std::move(handle)};
	if (rows.len() == 0) {
		return std::nullopt;
	}
	return rows;
}

/**
 * The schema lane's resolution table. Statement rows land in the
 * engine's MATERIALIZED order — fresh-implied keys (relation order, then
 * field order), then closed auto-keys (one per closed relation,
 * declaration order), then the declared statements in written order
 * (lowering.md §2) — so keyed-read statement ids stay aligned.
 */
template<Theory S>
[[nodiscard]] auto manifest_of(S const& theory) -> Manifest {
	auto manifest = Manifest{};
	manifest.relation_names.reserve(theory.relation_table.size());
	for (auto const& relation : theory.relation_table) {
		manifest.relation_names.emplace_back(relation.name.view());
	}
	for (auto const& relation : theory.relation_table) {
		for (auto index = std::size_t{0}; index != relation.field_count; ++index) {
			auto const& field = relation.fields[index];
			if (!field.fresh) {
				continue;
			}
			manifest.statement_rows.push_back(StatementRow{
			    .is_key = true,
			    .relation = std::string{relation.name.view()},
			    .projection = {std::string{field.name.view()}},
			});
		}
	}
	for (auto const& relation : theory.relation_table) {
		if (!relation.closed) {
			continue;
		}
		manifest.statement_rows.push_back(StatementRow{
		    .is_key = true,
		    .relation = std::string{relation.name.view()},
		    .projection = {std::string{"id"}},
		});
	}
	for (auto const& statement : theory.statements) {
		auto row = StatementRow{
		    .is_key = statement.form == statement_form::key,
		    .relation = std::string{statement.source.relation.view()},
		    .projection = {},
		};
		if (row.is_key) {
			row.projection.reserve(statement.source.width);
			for (auto index = std::size_t{0}; index != statement.source.width; ++index) {
				row.projection.emplace_back(statement.source.fields[index].view());
			}
		}
		manifest.statement_rows.push_back(std::move(row));
	}
	return manifest;
}

/**
 * Resolves a stored key law to its materialized statement id, or dies:
 * passing a law from outside the admitted schema (or keyed-reading a
 * pre-schema-lane store) is an impossible programmer state — the law and
 * the manifest are both artifacts of the same declaration set.
 */
template<class First, class... Rest>
[[nodiscard]] auto resolved_key(Manifest const& manifest, key_law<First, Rest...> const&) -> std::uint16_t {
	using Law = key_law<First, Rest...>;
	auto names = std::array<std::string_view, Law::width>{};
	for (auto index = std::size_t{0}; index != Law::width; ++index) {
		names[index] = Law::projection[index].view();
	}
	auto const id = manifest.resolve_key(Law::relation_name.view(), names);
	contract_assert(id.has_value());
	return *id;
}

/**
 * Resolves a relation's primary key statement, or dies (a fresh-bearing
 * facade of the admitted schema always has one).
 */
[[nodiscard]] auto resolved_primary(Manifest const& manifest, std::string_view relation) -> std::uint16_t {
	auto const id = manifest.resolve_primary(relation);
	contract_assert(id.has_value());
	return *id;
}

template<class Facade, class Law>
[[nodiscard]] consteval auto keyed_get_mismatch() -> std::string {
	return std::string{"bumbledb get(): the key law constrains relation \""} + std::string{Law::relation_name.view()} +
	       "\" but the facade names relation \"" + std::string{facade_relation_name(Facade{})} + "\"";
}

}
