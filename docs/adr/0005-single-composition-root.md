# crates/composition is the sole composition root; worker stops hand-wiring adapters

The workspace had two composition roots. `crates/presentation` built its adapters and deps
through `crates/composition`; `crates/worker` built its own, separately, in
`crates/worker/src/db.rs` (125 lines) — a 25-field `WorkerDbOutput` of which 22 duplicated fields
`composition`'s then-28-field `DatabaseOutput` already had, two of those under different names
(`_goal_command` / `_watch_event_query`, underscore-prefixed because the worker's copy never read
them). Every adapter change had to be
made twice, in two files that had already drifted (three fields — `deduplicator`,
`image_ref_command`, `image_ref_query` — existed only in the worker's copy, unreachable from the
server). Nothing enforced that the two stayed in sync; only a diff would catch it.

## Decision

**`crates/composition` is now the only composition root. `crates/worker/src/db.rs` is deleted.**
Both binaries call `composition::factory::build_database_adapters` to get one `DatabaseOutput`,
and each then builds only the deps it can honestly use from it.

- `DatabaseOutput` (`crates/composition/src/factory.rs`) is a deliberate 31-field superset of
  every adapter either binary needs. The server constructs three adapters it never reads —
  `deduplicator`, `image_ref_command`, `image_ref_query` (worker-only) — in exchange for one
  construction path that cannot drift. That trade is the point of this ADR: a handful of
  wasted `Arc` constructions on the server side is cheaper than a second hand-maintained factory.
- The deps container is split, not shared: `application::Deps` (25 fields across 10 groups) is
  built by `composition::build_deps` and consumed only by `crates/presentation`; the five groups
  with no server-reachable consumer — `enrich_movie`, `reindex_search`, `merge_duplicates`,
  `enrich_person`, `handle_requested` — moved into a new `application::WorkerDeps`, built by
  `composition::build_worker_deps` and consumed only by `crates/worker`. Every field of `Deps` is
  now reachable from `presentation` except `users.delete_account` — see the dead-code exception
  recorded below — and every field of `WorkerDeps` is reachable from `worker`. No field needs an
  `Option` or a Noop port to express "not for this binary" — which container it lives in already
  says that.
- `application::WorkerServices` is the analogous split on the services side: the strict subset of
  `application::Services` the worker can actually construct (`object_storage`, `event_publisher`,
  `person_enrichment`). The worker has no `auth`, `password_hasher`, `diary_exporter`,
  `document_parser`, or `review_logger` — those use cases don't run in the worker process, so
  `WorkerServices` simply doesn't carry them.
- `build_worker_deps` takes `&DatabaseOutput`, not `&Repositories`, even though `Repositories` is
  what `build_deps` takes and superficially looks like the more natural shared type. The worker
  cannot honestly construct a `Repositories`: five of its six federation-sourced fields
  (`remote_watchlist`, `social_command`, `follow_graph`, `block_query`, `federation_admin` — the
  sixth, `federated_profile`, is already `Option` in `Repositories` for unrelated reasons) have no
  worker-side adapter. `DatabaseOutput` only carries fields both binaries can genuinely fill, so
  passing it instead of `Repositories` needed no new `Option` and no fake adapter.
- Exactly one field is unreachable from its own binary, and it is called out by name rather than
  folded into a group comment: `Deps.users.delete_account` (`DeleteAccountDeps`) has zero callers
  anywhere in the workspace — a pre-existing dead use case, not a consequence of this split. Every
  other field that used to be described as "worker-only" or "not reachable" is now reachable from
  the binary that reaches it, because it lives in that binary's own container.
- Both binaries were booted under the default feature set (`sqlite`, `sqlite-federation`) to prove
  this: the worker reached its steady polling state without panicking, and the server served both
  a Bearer-authenticated API call and a cookie-authenticated HTML page. A container wired to the
  wrong adapter is exactly the kind of defect unit tests can't see — only a running process can.

## Considered Options

- **A separate `WorkerExtras` struct, kept alongside `Deps`** — rejected because it recreates the
  exact problem this ADR fixes: two structs that have to be kept in sync by hand, just smaller
  ones than `WorkerDbOutput` was.
- **Noop ports for the five `Services` the worker lacks** (`auth`, `password_hasher`,
  `diary_exporter`, `document_parser`, `review_logger`) — rejected because it reintroduces the
  sentinel-port pattern this codebase has been actively removing elsewhere; a `Services` field the
  worker can't fill should not exist in a struct the worker holds, dressed up as a port that
  panics or no-ops if called.
- **`Option` fields on a single shared `Deps`/`Services`, meaning "not for this binary"** —
  rejected for the same reason as ADR-0002/0003 avoid `Option` for "not applicable here": absence
  is a fact about which binary you're in, and that fact is better expressed by which container
  (`Deps` vs `WorkerDeps`, `Services` vs `WorkerServices`) a field lives in than by an `Option`
  every reader then has to unwrap or justify.

## Known follow-up, out of scope here

`crates/worker/src/follow_backfill_handler.rs` references `activitypub::ActivityPubPort`
unconditionally, with no `#[cfg(feature = "federation")]` gate. `cargo build -p worker
--no-default-features --features sqlite` (federation off) fails to compile because of it. This
predates worker unification — confirmed byte-identical at the commit before this plan started —
and is unrelated to the composition-root split; it needs its own fix.
