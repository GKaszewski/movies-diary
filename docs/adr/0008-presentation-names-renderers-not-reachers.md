# `presentation` may name renderers, not reachers; `ActivityPubPort` inverts into three `domain` ports

ADR-0006 and ADR-0007 both closed with the same known follow-up, in the same words: "Plan D inverts
`ActivityPubPort` so `presentation` drops its one remaining adapter dependency, `activitypub`." That
framing understated the problem in one direction and overstated it in another. Understated, because
`activitypub` was not the only adapter crate `presentation` depended on — `crates/presentation/Cargo.toml`
carried **eleven** adapter-crate entries at `051d9ce`: ten unconditional (`auth`, `metadata`,
`poster-fetcher`, `object-storage`, `template-askama`, `rss`, `export`, `importer`, `jellyfin`, `plex`)
plus `activitypub` as an optional dependency behind the `federation` feature. Overstated, because the
reason the inversion kept getting deferred — that moving the trait into `domain` would drag
`k_ap::RemoteActor` and `k_ap::BlockedDomain` in with it, giving the domain layer a dependency on a
third-party federation library — turned out to be an artifact of treating a 17-method trait as one
indivisible unit.

Seven of the eleven were simply dead. They were found by commenting each dependency line out and
running `cargo check -p presentation --all-features`, then repeating with `--all-targets`: `auth`,
`metadata`, `poster-fetcher`, `object-storage`, `rss`, `export`, `importer`. **Both grep-based methods
that would have been the obvious way to audit this gave wrong answers**, and the reason is worth
recording because it also determines the shape of this ADR's guard: `use <crate>::` misses the
fully-qualified call sites, and plain `<crate>::` collides with same-named *modules* inside
`application` and `presentation`. `auth::` matches `application::auth::logout`. `rss::` matches
`handlers::rss::get_user_feed`. Both crates therefore looked live to a source grep while being
entirely unreferenced as crates. The one function in `presentation` that genuinely named an adapter
type — `mappers/social.rs::remote_actor_to_dto`, taking `activitypub::RemoteActor` — had zero callers
anywhere in the workspace and was deleted with the file.

`ActivityPubPort`'s 17 methods split across three **disjoint** consumer sets, which is what dissolves
the `k_ap`-in-`domain` blocker:

| Methods | Only consumer |
| --- | --- |
| `follow`, `unfollow`, `accept_follower`, `reject_follower`, `remove_follower`, `block_actor`, `unblock_actor`, `get_blocked_actors` | `CompositeSocialAdapter`, **inside the `activitypub` crate itself** |
| `actor_json`, `followers_collection_json`, `following_collection_json`, `get_blocked_domains`, `add_blocked_domain`, `remove_blocked_domain` | `presentation` |
| `get_following`, `import_remote_outbox`, `run_backfill_for_follower` | `worker::FollowBackfillHandler` |

Eight of the seventeen have exactly one call site today, and it lives in the same crate as the trait,
so they did not need a trait. That is a claim about call sites, not reachability: `ActivityPubWire`
also hands `crates/server` the same `Arc<ActivityPubService>` that `CompositeSocialAdapter` wraps (the
`service` field, below), so all eight are `pub` and callable from outside the crate today — nothing
does, but nothing stops it either. See the deferred item below on closing that gap.

Exactly two of the seventeen return `k_ap::RemoteActor` — `get_blocked_actors`, which is one of those
eight and therefore lost its trait, and `get_following`, which is worker-only. Presentation reached
neither. The only `k_ap` type in
presentation's six signatures is `BlockedDomain` in `get_blocked_domains`'s return, and both call
sites consumed it structurally — reading `.domain`, `.reason`, `.blocked_at` into an
`api_types::BlockedDomainResponse` or a `template_askama::BlockedDomainEntry` — without ever naming
the type. `BlockedDomain` is `{ domain: String, reason: Option<String>, blocked_at: String }`;
`blocked_at` is `String` at every layer including both destinations, so a three-field domain-owned
record reproduces it exactly. The worker's `get_following` does return `Vec<RemoteActor>`, but the
worker reads exactly `.url` and `.outbox_url`. Two minimal domain models therefore replace both `k_ap`
types at the boundary, and `crates/domain/Cargo.toml` gained no dependency other than a new
`[dev-dependencies]` entry for `tokio`, to run the noop tests.

## Decision

**`presentation` may name things that render output; it may not name things that reach external
systems or storage.** `template-askama` is the sole permitted adapter crate, because an HTML template
engine is a renderer: it turns data the handler already holds into bytes. Everything else in
`crates/adapters/` reaches — over HTTP, to a database, to object storage, to a message queue — and a
handler that names one of those has, by construction, a path around the application layer that
ADR-0006 and ADR-0007 spent two plans closing. `crates/presentation/Cargo.toml`'s `[dependencies]`
now lists exactly one entry from `crates/adapters/`: `template-askama`. Verified by iterating all 26
directory names under `crates/adapters/` against the section and finding exactly one hit.

**`ActivityPubPort` is deleted and replaced by three consumer-shaped traits in
`domain::ports::federation`.** `ApDocumentPort` (3 methods) serves ActivityPub documents for content
negotiation and is called only from presentation. `InstanceBlocklistPort` (3 methods) is instance-wide
domain blocklist administration, also presentation-only. `ApBackfillPort` (3 methods) pulls remote
content in and pushes local content out after a follow is established, and is worker-side only — no
HTTP handler calls it. The eight adapter-internal methods lost their trait entirely;
`CompositeSocialAdapter` now takes a concrete `Arc<k_ap::ActivityPubService>` and calls them as
inherent methods. All nine surviving methods return `Result<_, DomainError>` rather than
`anyhow::Result<_>`, so the error type crosses the boundary as a domain type. `grep -rn
"ActivityPubPort\|NoopActivityPubService" crates` returns nothing.

**The adapter carries the three impls on a local wrapper type, `ApServiceAdapter`, not on
`k_ap::ActivityPubService` directly — this is forced, not stylistic.** The plan originally specified
`impl domain::ports::ApDocumentPort for k_ap::ActivityPubService`. That does not compile: E0117, the
orphan rule. From inside `crates/adapters/activitypub`, both the trait (owned by `domain`) and the
type (owned by the external `k-ap` crate) are foreign. The deleted `port.rs` compiled only because
`ActivityPubPort` was *local* to that crate — the very property being given up by moving the trait to
`domain`. `ApServiceAdapter { service: Arc<ActivityPubService> }` in
`crates/adapters/activitypub/src/federation_ports.rs` holds all three impls, matching two existing
precedents in the same crate: `DomainUserRepoAdapter` (a local wrapper across a foreign boundary) and
`CompositeSocialAdapter` (one type serving three `domain` traits, handed out as three `Arc`s). A
side effect worth naming, since the plan feared the opposite: because `ApServiceAdapter` has no
inherent `actor_json`, `self.service.actor_json(..)` is unambiguous and no fully-qualified call or
recursion guard is needed. `ActivityPubWire` gained `document`, `blocklist`, and `backfill` fields,
three `Arc` casts of one `ApServiceAdapter`.

**`actor_json` keeps its `Err(_) => 404`; the two collection handlers now return a logged 500.** This
is the plan's one intentional behavior change and lands in its own commit (`8bfeae5`). The asymmetry
is a measured property of `k_ap`, not a preference: `followers_collection_json` and
`following_collection_json` never look the user up. They derive a URL from the UUID and count rows, so
a nonexistent user yields `total = 0` and a valid empty `OrderedCollection` — `Ok`, not `Err`. Every
`Err` those two can produce is therefore a genuine infrastructure or serialization failure, and the
404 they used to emit misreported it as "no such user", hiding real faults from operators.
`actor_json` calls `get_local_actor`, which genuinely fails for a missing user, and a federation peer
probing an actor URL should get 404 — so it is untouched, deliberately. Two tests in
`crates/presentation/src/tests/api_handlers.rs` pin the new behavior; both were observed failing with
`left: 404, right: 500` before the change, and both send `Accept: application/activity+json`, without
which the handlers redirect and the tests would have passed against the old code too.

**Jellyfin and Plex parser construction moved to the composition root.** `presentation`'s two webhook
handlers used to name `jellyfin::JellyfinParser` / `plex::PlexParser` inline, passing each unit struct
by reference straight into `run_ingest`. The parsers are now built once in `composition::build_deps`
and held on `application::Deps`'s `IntegrationsGroup`; the
handlers read `state.app_ctx.deps.integrations.{jellyfin,plex}_parser`. `jellyfin` and `plex` moved
from `presentation`'s `[dependencies]` to `composition`'s, unconditionally. `ingest::execute`'s
signature and its test-suite `FakeParser` injection point are untouched.

**A fourth Makefile guard, `check-presentation-adapters`, encodes the rule.** `make check` now runs
`fmt-check clippy test check-appcontext check-handler-deps check-handler-repos
check-presentation-adapters`, and both CI workflows (`.github/workflows/ci.yml:47` and
`.gitea/workflows/ci.yml:47`, which must stay in sync) invoke it. **It reads `Cargo.toml`, not `src/`,
deliberately** — for exactly the reason the `auth`/`rss` audit above failed: a source-level
`<crate>::` grep cannot distinguish an adapter crate from a same-named module inside `application` or
`presentation`, so it is structurally incapable of deciding this question. The guard `awk`s the
`[dependencies]` section, `sed`s out the key of each `key = ...` line, and rejects any that matches a
directory name under `crates/adapters/` other than `template-askama`. All 26 adapter directory names
equal their package `name` today (verified by reading each `Cargo.toml`), which is what makes
directory names a safe source of truth. Both behaviors were observed: `jellyfin` added under
`[dependencies]` produced `❌ presentation depends on adapter crate(s): jellyfin` and exit 1; the same
line under `[dev-dependencies]` passed, which is what proves the `awk` range actually stops at the
next section header rather than scanning the whole file.

**`cargo build -p worker --no-default-features --features sqlite` compiles again, closing a follow-up
ADR-0005 opened and ADR-0006/0007 carried forward.** `crates/worker/src/follow_backfill_handler.rs`
named `activitypub::ActivityPubPort` unconditionally, with no `#[cfg(feature = "federation")]` gate,
so the federation-off worker build failed with E0433. `FollowBackfillHandler` now holds
`Arc<dyn domain::ports::ApBackfillPort>`, a trait from a crate the worker depends on unconditionally,
so the gate is unnecessary rather than merely added. Three `map_err` closures that converted `anyhow`
to `DomainError` at the call site collapsed to bare `?`, because the port already returns
`DomainError` — verified to produce the same `DomainError` and the same log output as the closures did.

### Four things this ADR does not claim

These were measured or observed during the work and are recorded rather than fixed. This project
documents its guards as useful-not-airtight (see ADR-0007 on `check-handler-repos`); the same honesty
applies here.

**`check-presentation-adapters` has four known blind spots.** Each needs an atypical or deliberately
obfuscated manifest form, and none is present today:
1. `[dependencies.jellyfin]` dotted-table form — `awk`'s `/^\[/` closes the range, and the crate name
   lives in the section header rather than on a `name = ` line. Invisible to the guard.
2. `[target.'cfg(...)'.dependencies]` — never scanned at all; the range opens only on an exact
   `^\[dependencies\]`. This is the most plausible *accidental* blind spot, though unlikely in a web
   backend crate.
3. `alias = { package = "jellyfin", workspace = true }` — the `sed` extracts `alias`, which matches no
   adapter directory. Requires deliberate evasion.
4. Directory name equal to package name is *assumed*, not enforced. True for all 26 adapters today; a
   future adapter whose directory and package names diverge would be unguarded.

**Two HTML handlers now log one extra line on the error path.** The plan's contract for Tasks 1-7 was
"identical status codes, identical response bodies, identical log lines", and the third of those is
not literally true. `ap_err` in the adapter logs `ActivityPub error: {:?}` before returning — the same
message, at the same level, that presentation's deleted `ap_to_domain` used to emit — so the two
blocklist HTML handlers (`handlers/social.rs` `post_blocked_domain`, `post_remove_blocked_domain`),
which log their own line on error, now emit two lines where they emitted one. The two collection
handlers (`get_followers_collection`, `get_following_collection` in `handlers/social.rs:707` and
`:742`) previously logged nothing on the error path and now log two lines, not one: the adapter's
`ap_err` line and their own (`followers_collection_json error: {:?}` / `following_collection_json
error: {:?}`), so their error path went from 0 lines to 2. The two lines are complementary, not
duplicative — the adapter's carries the cause, the full `anyhow` chain via `{:?}`; the handler's
identifies which operation failed. Strictly more detail, same level, no client-visible change;
unavoidable once the conversion point moved into the adapter, short of reintroducing a shim in
presentation purely to suppress it.

**"The worker sqlite-only build compiles" is not "it is clippy-clean."**
`cargo clippy -p worker --no-default-features --features sqlite -- -D warnings` reports three errors:
unused `app_config`, unused `remote_goal`, and `FollowBackfillHandler is never constructed`. The first
two predate this work and are unrelated to it. No gate runs this combination — `make clippy` is
`cargo clippy -- -D warnings` and CI adds `--all-targets`, both at default features, where the
worker's defaults are `["sqlite", "sqlite-federation"]` and the struct *is* constructed. Gating
`mod follow_backfill_handler;` behind a `federation` cfg would silence one of the three and leave the
combination dirty regardless, so it was measured and deliberately not done.

**`cargo check -p presentation --no-default-features --all-targets` fails, and failed identically
before this work.** Two errors, both in the test target: E0433 `cannot find social in handlers`
(`handlers::social` is federation-gated; something in the test tree names it unconditionally) and
E0277 `Panic: RemoteWatchlistRepository` not satisfied. Measured on the worktree and on master at
`051d9ce` with the same command and the same two errors, so it is pre-existing, not a regression from
the port swap. The library itself compiles federation-off both before and after — `cargo check -p
presentation --no-default-features` finishes clean on both — so the `#[cfg(feature = "federation")]`
gates around the new `ap_document` / `ap_blocklist` fields do hold. It remains a known-broken feature
combination in the repo — not the only one; see the `server`/`worker` `federation` meta-feature gap
recorded under Known follow-ups below, which is also pre-existing.

## Considered Options

- **Moving `ActivityPubPort` into `domain` whole, accepting a `k_ap` dependency in the domain layer** —
  the option the previous two ADRs implicitly assumed and rejected, which is why the work kept being
  deferred. Rejected here too, but the point is that it was never the only option: splitting by
  consumer means the two `k_ap` types that forced the dependency (`RemoteActor`, `BlockedDomain`) land
  respectively in the eight methods that need no trait at all and in one method whose three scalar
  fields a domain-owned record reproduces exactly.
- **One `domain` trait with all nine cross-boundary methods, instead of three** — simpler to wire: one
  `Arc` on `AppContext` instead of two, and one trait to name instead of three. Rejected because the
  three consumer sets are disjoint: presentation would hold a handle exposing `run_backfill_for_follower`,
  the worker one exposing `actor_json`, and the ability to call a method is what the previous two ADRs
  were about removing. Three traits make "no HTTP handler runs a backfill" a compile-time fact rather
  than a convention.
- **Grepping `src/` for adapter crate names instead of reading `Cargo.toml`** — the intuitive form of
  this guard. Rejected because it was tried as the audit method and demonstrably produced wrong
  answers in both directions: `auth` and `rss` looked live because `application::auth` and
  `handlers::rss` exist, while fully-qualified call sites are invisible to a `use <crate>::` pattern. A
  guard cannot be more reliable than the measurement technique it encodes.
- **Deleting `port.rs` in the same task that added the new ports** — the plan's original shape, which
  would have left `presentation`, `server`, and `worker` uncompilable for three tasks and required
  skipping `make check` in between, contradicting the plan's own green-at-every-commit constraint. Split
  into additive-first (new ports alongside the old trait) and delete-last (after every consumer moved),
  which is why the branch has an extra commit and no known-red window.

## Known follow-ups, out of scope here

- **Collapsing the worker's three backfill calls into one adapter-side operation.** `FollowBackfillHandler`
  calls `get_following`, filters for the actor, then `import_remote_outbox`. That filtering could live
  behind one port method, but moving it would move logic across the boundary during a refactor whose
  contract was behavior preservation. Not started; a real question, deliberately left open. The
  duplication is also a naming collision: `ApBackfillPort::get_following` is now the third
  `get_following` in the `domain::ports` glob namespace, alongside `FollowGraphQuery::get_following`
  (`crates/domain/src/ports/social.rs:47`) and `FollowQuery::get_following`
  (`crates/domain/src/ports/follow.rs:53`). All three compile and answer different questions, but a
  reader seeing `deps.something.get_following()` now has three candidates.
- **Splitting `actor_json`'s error so a missing actor and an infrastructure failure become
  distinguishable.** Today both produce `DomainError::InfrastructureError` and the handler returns 404
  for either. Doing it properly needs `k_ap` to expose a typed error; doing it by inspecting the
  `anyhow` message for a substring would be fragile against a third-party crate's wording. Rejected as
  currently formulated, not deferred for lack of time.
- **`service`'s reachability gap, noted above.** `CompositeSocialAdapter::new` is only called from
  `crates/server/src/main.rs:127`, outside this crate, using `ActivityPubWire::service`. Moving that
  construction into `wire()` itself and exposing `Arc<dyn SocialCommand>` / `FollowGraphQuery` /
  `BlockQuery` on `ActivityPubWire` instead would let `service` become crate-private, and would make
  "the eight adapter-internal methods are unreachable from outside the crate" true of reachability, not
  just of call sites, closing the gap recorded above. Not started here — `wire()` doesn't have the
  `UserRepository`, `FollowCommand`, and `FollowQuery` handles `CompositeSocialAdapter::new` takes,
  and threading them through is real wiring work, not a rename.
- **The `PRESENTATION_ALLOWED_ADAPTERS` guard trusts `template-askama` by name, not by property.**
  `PRESENTATION_ALLOWED_ADAPTERS := template-askama` permits that crate because it is a renderer today,
  but nothing stops it from later acquiring a reaching dependency and smuggling it through the guard
  transitively — the Makefile checks the manifest's direct `[dependencies]` entries, not what
  `template-askama` itself depends on. It is clean today: its own `[dependencies]` are `askama`,
  `chrono`, `uuid`, `domain`, `api-types`, none of which reach. The "renderer, not reacher" premise
  genuinely holds, but it is trusted rather than checked.
- **A fourth known-broken feature combination, pre-existing and untouched by this branch:**
  `cargo check -p server --no-default-features --features sqlite,federation` fails with E0433 on
  `activitypub`. `server`'s `federation` feature (`crates/server/Cargo.toml:14`) is a bare meta-feature
  that enables `application/federation` and `presentation/federation` but does not imply
  `dep:activitypub` — only `sqlite-federation` and `postgres-federation` do that. The same gap exists in
  `crates/worker/Cargo.toml`. Neither manifest was modified by this branch (both last touched at
  `4d8f836`, before this branch's merge-base `051d9ce`), so this is recorded here rather than
  attributed to this plan.
- The other deferred items recorded in ADR-0005/0006/0007 are unaffected by this plan: the isolated
  `cargo test -p composition` `worker_deps_wiring` failure, the admin-extractor body assertions, the
  SPA zod schemas, and `check-handler-repos`'s substring fragility. The one item this plan does close
  is the worker sqlite-only build, as recorded above.
- `crates/presentation/Cargo.toml`'s `[dev-dependencies]` still lists `composition`, which ADR-0007
  established as legitimate — test helpers build a real `Repositories` and call `composition::build_deps`.
  `check-presentation-adapters` intentionally does not scan that section, and it does not need to:
  `composition` is not an adapter crate.

---

Superseded in part by [ADR-0012](0012-sqlite-is-the-only-backend.md): the postgres backends, crates and features mentioned above no longer exist; SQLite is the only backend.

Now `make check-http-adapter-deps` over `crates/adapters/http-axum/Cargo.toml` (was `check-presentation-adapters`); see [ADR-0014](0014-http-adapter-and-one-refusal-shape.md).
