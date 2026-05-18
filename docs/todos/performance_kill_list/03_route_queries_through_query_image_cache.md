# 03: Route Queries Through QueryImageCache

**Goal**
- Make `ReadTxn::execute_query` use the existing `Environment` `QueryImageCache` instead of rebuilding `QueryImage` directly.

**Trace Evidence**
The benchmark prebuilds one image per dataset, but timed query execution still rebuilds images because `ReadTxn::execute_query` calls `QueryImageBuilder::new(self, schema).build()`.

Visible image build costs:
- ledger: `8.1ms`
- sailors: `4.1ms`
- joinstress: `5.7ms`
- tpch: `7.4ms`

These costs are paid again inside each timed query execution today.

**Current Code Facts**
- `Environment` owns `query_images: QueryImageCache`.
- `Environment::query_image` correctly calls `QueryImageCache::get_or_build`.
- `ReadTxn` does not carry a cache reference.
- Benchmarks call `env.query_image(&schema)` for stats, then time `env.read(|txn| txn.execute_query(...))`, bypassing the warmed image.

**Required Design**
- Add `query_images: &'env QueryImageCache` to `ReadTxn<'env>`.
- `Environment::read` wires `query_images: &self.query_images`.
- `ReadTxn::execute_query` calls `self.query_images.get_or_build(self, schema)?`.
- Keep the public `ReadTxn::execute_query` signature unchanged.
- Keep `QueryImageBuilder` for low-level deterministic/image tests.

**Implementation Steps**
1. Extend `ReadTxn` internals with a cache reference.
2. Update `Environment::read` construction.
3. Replace direct builder call in `execute_query` with cache lookup.
4. Pass `image.as_ref()` to planner/executor.
5. Add query-image cache hit/miss diagnostics.
6. Update benchmark output to show the image actually used by timed queries.

**Tests**
- Warm `env.query_image(&schema)`, then execute a query and assert no new image build occurs.
- Repeated `execute_query` on the same snapshot reuses cached image.
- Write commit advances tx ID and causes a new image key.
- Different schema fingerprint does not reuse an image.
- Active read snapshot remains stable across later commit.

**Acceptance Criteria**
- `ReadTxn::execute_query` no longer directly calls `QueryImageBuilder::new(...).build()` in production code.
- Query execution obtains images through `QueryImageCache::get_or_build`.
- Existing call sites remain source-compatible.
- Traced benchmark warm runs no longer emit one image build per repeat.
- Existing result and counter gates continue passing.

**Risks**
- Lifetime wiring must keep the cache reference tied to the environment.
- Cache memory retention grows with write tx IDs; eviction can be future work.
- Cold concurrent misses may duplicate builds unless a per-key initializer is used.
