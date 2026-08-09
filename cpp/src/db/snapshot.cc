export module bumbledb:snapshot;

import std;
import :error;
import :answers;
import :answers_row;
import :row;
import :key;
import :schema;
import :query;
import :manifest;
import :prepared;
import :foreign_program;
import bumbledb_foreign;

export namespace bdb {

/**
 * The lexical borrowed read capability: alive exactly for the Db::read
 * callback. Non-copyable, non-movable, constructible only by Db's
 * trampoline; it never owns and never outlives the callback frame.
 */
class Snapshot {
	foreign::bdb_snapshot_ref const& raw_;
	detail::Manifest const& manifest_;

	Snapshot(foreign::bdb_snapshot_ref const& raw, detail::Manifest const& manifest) : raw_{raw}, manifest_{manifest} {}

	friend class Db;

public:
	Snapshot(Snapshot const&) = delete;
	auto operator=(Snapshot const&) -> Snapshot& = delete;
	~Snapshot() = default;

	/**
	 * Committed-state membership of one row (marshalled by reflection in
	 * declaration order).
	 */
	template<class Facade, class Row>
	[[nodiscard]] auto contains(Facade const& relation, Row const& row) const -> std::expected<bool, Error> {
		auto const cells = marshal_row(row);
		return foreign::snapshot_contains(raw_, detail::resolved_relation(manifest_, detail::facade_relation_name(relation)), cells)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/**
	 * Full-relation export in row_id order: one owned crossing, iterated
	 * host-side. Cells decode to bdb::Value.
	 */
	template<class Facade>
	[[nodiscard]] auto scan(Facade const& relation) const -> std::expected<RowSet, Error> {
		return foreign::snapshot_scan(raw_, detail::resolved_relation(manifest_, detail::facade_relation_name(relation)))
		    .transform([](foreign::row_set_handle handle) {
			    return RowSet{std::move(handle)};
		    })
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/**
	 * Committed-state keyed point read: the stored key law value is the
	 * selector — resolved against the schema's materialized statements
	 * by structural identity, never through a generated nominal type.
	 * Key values arrive as the law's pattern product, members in
	 * projection order. A miss is genuine absence.
	 */
	template<class Facade, class First, class... Rest>
	[[nodiscard]] auto get(Facade const& relation, key_law<First, Rest...> const& law,
	                       typename key_law<First, Rest...>::pattern const& key) const -> std::expected<std::optional<RowSet>, Error> {
		static_assert(detail::facade_relation_name(Facade{}) == key_law<First, Rest...>::relation_name.view(),
		              detail::keyed_get_mismatch<Facade, key_law<First, Rest...>>());
		auto const cells = marshal_row(key);
		return foreign::snapshot_get(raw_, detail::resolved_relation(manifest_, detail::facade_relation_name(relation)),
		                             detail::resolved_key(manifest_, law), cells)
		    .transform(detail::lift_row)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/**
	 * The fresh-field primary read: `snap.get(Service, {.id = id})`
	 * reads through the relation's PRIMARY key — the first materialized
	 * key, i.e. the fresh field's implied key.
	 */
	template<class Facade>
	    requires(fresh_field_count<Facade>() >= 1)
	[[nodiscard]] auto get(Facade const& relation, fresh_pattern_of<Facade> const& key) const
	    -> std::expected<std::optional<RowSet>, Error> {
		auto const cells = marshal_row(key);
		return foreign::snapshot_get(raw_, detail::resolved_relation(manifest_, detail::facade_relation_name(relation)),
		                             detail::resolved_primary(manifest_, detail::facade_relation_name(relation)), cells)
		    .transform(detail::lift_row)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/**
	 * Executes a prepared query into the caller's reusable carrier (the
	 * zero-alloc lane): the carrier is cleared first, capacity retained.
	 * Params arrive as the query's synthesized product — so a wrong name
	 * or type is a compile error; the engine still validates at bind.
	 */
	template<auto Query>
	[[nodiscard]] auto execute_into(Prepared<Query>& prepared, params_of<Query> const& params, Answers<Query>& answers) const
	    -> std::expected<void, Error> {
		auto scratch = foreign::param_scratch{};
		auto const wire = foreign::wire_params_for<Query>(params, scratch);
		return prepared.native().execute(raw_, wire, answers.native().native()).transform_error([](foreign::error_handle handle) {
			return detail::lift(std::move(handle));
		});
	}

	/**
	 * The convenience execute: one whole-result bridge crossing,
	 * iterated locally through the typed rows() range.
	 */
	template<auto Query>
	[[nodiscard]] auto execute(Prepared<Query>& prepared, params_of<Query> const& params) const -> std::expected<Answers<Query>, Error> {
		auto answers = Answers<Query>{};
		return execute_into<Query>(prepared, params, answers).transform([&answers] {
			return std::move(answers);
		});
	}
};

}
