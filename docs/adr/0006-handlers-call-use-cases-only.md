# Handlers call use cases only; check-handler-repos guard, and presentation goes lib-only

CONTRIBUTING.md had said "presentation handlers never touch repositories directly" for a while,
but nothing enforced it. 16 call sites across `crates/presentation/src/handlers/` called a
repository method directly — `repos.user.find_by_id(...)`, `repos.wrapup_repo.find_existing(...)`,
`repos.import_session.get(...)`, and thirteen more — instead of going through an
`application`-layer use case. Each bypass carried real logic that had nowhere principled to live:
`get_user_profile`'s local-vs-federated dispatch, `get_watchlist_page`'s decision to read a local
or a remote repository, `get_wrapup_html`'s "is this report actually ready" gate, `get_mapping_page`'s
"has this session even been parsed" gate, `build_page_context`'s viewer-chrome assembly (email,
admin flag, pending-follower badge count) — all sitting in a handler function, untested except by
whatever HTTP-level test happened to exercise that route, and invisible to `check-appcontext`
because bypassing `application` entirely means there is no `application` code to scan.

A companion problem, orthogonal but resolved in the same plan: `crates/presentation` carried
`main.rs` and every backend-selection feature (`sqlite`, `postgres`, `sqlite-federation`,
`postgres-federation`, `nats`) even though `crates/composition` (ADR-0005) was already the actual
composition root. The binary and the library were the same crate for no reason connected to
either one's job.

## Decision

**Handlers call use cases only, never repository methods, enforced by `make check-handler-repos`.**
The Makefile target greps `crates/presentation/src/handlers/` for `repos.<field>.<method>(` and
fails, naming file and line, on any match whose method name isn't `clone`. `.clone()` stays
legal: ~40 sites in handlers still read `state.app_ctx.repos.<field>.clone()` to pass an `Arc`
positionally into a use case that takes individual params rather than a deps struct — Plan C2's
job, not this plan's. The guard is two grep passes rather than one, specifically so a single
line carrying both a legal clone and an illegal call doesn't hide the illegal one: `grep -o`
extracts each `repos.<field>.<method>(` occurrence onto its own output line (file:line still
attached, since `-n`/`-r` key off the match, not the containing line), then a second `grep -v`
drops the occurrences whose method name is exactly `clone`. A single `grep -v` filtering whole
lines would have wrongly cleared a line pairing `repos.a.clone()` with an illegal
`repos.b.find_by_id(...)`. Verified against the current tree (zero offenders) and against a
deliberately reintroduced `repos.user.find_by_id(` call in `handlers/users.rs::get_user_profile`,
which the guard reported by exact file and line before the call was reverted.

**Sixteen bypass call sites became eleven use cases** (twelve modules, because one use case
split in two — see below), each one absorbing the logic that used to live in the handler:

- `users::get_local_profile` / `users::get_federated_profile_stats` — the local and federated
  halves of what was one `get_profile` use case (ADR-0004). Absorbs the local-vs-federated
  identity decision described below.
- `users::get_page_viewer` — **page-chrome assembly**: viewer email, admin flag, and pending-
  follower badge count, previously three separate lookups scattered through
  `handlers/helpers.rs::build_page_context` with independent, inconsistent error tolerance.
  `get_page_viewer::execute` now propagates every error with `?`; `build_page_context` is the
  one place that decides to degrade chrome on failure (logs, falls back to `(None, false, 0)`).
- `users::resolve_username_to_id` — username → id lookup, previously a bare
  `repos.user.find_by_username(...)` call.
- `users::get_profile_settings` — settings-page field assembly (username, display name, bio,
  avatar/banner, `also_known_as`, custom profile fields), previously assembled inline from two
  separate repository calls in the handler.
- `import::get_mapping_stage` — **import stage gating**: `NotFound` for both "session missing"
  and "session has no parsed file yet", collapsing two conditions the old handler redirected to
  the same place for. Owns the `SAMPLE_ROW_LIMIT = 5` constant that used to be a bare `.take(5)`.
- `import::get_preview_stage` — the other **import stage gate**: `Ok(NotYetMapped)` when row
  results are absent (a business state, not an error) vs `Ok(Ready(PreviewRows))`. Called from
  two handler sites (`get_preview_page` HTML and `api_get_preview` JSON) — the one deliberate
  exception to "one use case, one call site" in this plan.
- `import::get_session_state` — session summary (columns, mapping status, row count) for the
  JSON session-state endpoint.
- `import::apply_profile_and_map` — orchestrates `apply_profile::execute` then
  `apply_mapping::execute` inside the use case layer instead of the handler, matching the
  existing `MovieEnrichmentHandler`-calls-`enrich_movie` precedent rather than inventing a new
  shape for cross-use-case calls.
- `diary::get_user_feed` — **feed author lookup**: resolves the RSS feed's author display name
  (from the user's email local-part) alongside the diary entries, previously two separate reads
  (`repos.user.find_by_id` then `get_diary::execute`) glued together in `handlers/rss.rs`.
- `watchlist::get_watchlist_for_owner` — **the local-vs-federated watchlist decision**: probes
  `repos.user.find_by_id` to decide whether the requested owner is a local user (paginated,
  validated `PageParams`) or a federated one (unpaginated, `limit`/`offset` ignored — a
  pre-existing asymmetry, preserved deliberately rather than "fixed" into new behavior). Returns
  `WatchlistView::Local(Paginated<..>) | Remote(Vec<..>)`; the handler still chooses which
  template to render.
- `wrapup::get_ready_report` — **wrapup report gating**: collapses "no record", "wrong status",
  "repository error", and "`Ready` but `report: None`" into one `NotFound`/`ValidationError`
  surface, reproducing a pre-existing quirk (repo errors becoming a 404) rather than introducing
  new-looking behavior for it. Takes `domain::models::wrapup::WrapUpScope` (`User(Uuid) |
  Global`) directly — an application-layer duplicate of that type was drafted and then dropped in
  favor of the existing domain enum once it was clear the duplicate added no impedance, just a
  second name for the same idea.

**The pending-follower count is one use case behind two transports.** Both the SPA's
`GET /api/v1/social/followers/pending/count` endpoint and the classic UI's page chrome
(`get_page_viewer`) read the same `FollowGraphQuery::count_pending_followers` call, which is also
the same method `application::social::count_pending_followers` wraps. No second counting path
exists; the two frontends cannot show different numbers because there is exactly one place the
number comes from.

**The ADR-0004 sentinel wart is closed.** ADR-0004 recorded a "known wart": `get_profile`'s
tolerant, non-`NotFound` handling of a missing local row filled `username`/`handle` with
empty-string sentinels while `actor_url` stayed a well-formed-looking (but meaningless) URL — one
field obviously wrong, one plausibly wrong — and noted it was inert only because no caller read
`profile.identity` on the federated path. This plan is the "moment any caller starts consuming
`profile.identity` on the federated path" ADR-0004 said to watch for, and it resolved the wart by
removing the shared type rather than patching it: `get_local_profile` returns
`Err(DomainError::NotFound(_))` for a missing local row with `identity` always fully populated on
success, and `get_federated_profile_stats`'s return type (`FederatedProfileStats`) carries no
`identity` field at all, because the federated handler never read one. No type in the codebase
can represent the old sentinel state anymore — there is no `ProfileIdentity` for the federated
path to half-fill.
`get_federated_profile_stats::tests::succeeds_for_a_user_id_with_no_local_row` carries forward
the old regression test's intent;
`get_local_profile::tests::get_local_profile_is_not_found_for_unknown_user` pins the new
`NotFound` behavior directly.

**`crates/presentation` is lib-only; `crates/server` owns the binary.** `main.rs` and the
integration test that exercised it moved to a new `crates/server` crate, which now holds every
backend-selection feature (`sqlite`, `postgres`, `sqlite-federation`, `postgres-federation`,
`nats`) that used to live on `presentation`. `presentation`'s `Cargo.toml` now has exactly one
feature axis — `federation` (on by default) — and depends on no adapter crate except
`activitypub`, gated behind that same feature. `cargo tree -p presentation` shows no
`sqlite`/`postgres`/`sqlx`/`infra-wiring` anywhere in the tree. The one remaining adapter
dependency, `activitypub`, is out of scope here — inverting `ActivityPubPort` so `presentation`
depends on nothing but `domain`/`application`/`composition` ports is Plan D's job, not this
plan's.

**`app_ctx.repos` deliberately remains, and stays `pub`.** This plan removed sixteen repository
*method calls* from handlers; it did not touch the ~40 `state.app_ctx.repos.<field>.clone()`
sites that pass an `Arc` positionally into use cases taking individual params, and it did not
delete the `repos` field itself. `check-handler-repos` is written narrowly on purpose — it
forbids the bypass this plan closes, not the clone-and-pass pattern Plan C2 is scoped to migrate.
Removing `repos` now would break every one of those 40 sites for no gain this plan can claim.

## Considered Options

- **Thin entity-fetch use cases** — e.g. a bare `get_user(id) -> User` wrapping
  `repos.user.find_by_id` — were rejected as a repository with extra indirection. They would
  satisfy `check-handler-repos` literally while leaving every real decision (local-vs-federated,
  stage gating, chrome assembly, report gating) sitting in the handler exactly where it was,
  which is the actual problem this plan exists to fix, not a naming exercise.
- **Deleting `app_ctx.repos` in this plan** — rejected because ~40 clone-passed sites across the
  converted handler files still construct use case calls positionally from it; deleting the field
  would require migrating all 40 to deps-struct calls in the same change, which is Plan C2's
  scope, not this one's. Landing that migration and the use-case extraction together would make
  either piece harder to review and revert independently.

## Known follow-ups, out of scope here

- Plan C2 migrates the ~40 `repos.<field>.clone()` sites in handlers to deps-struct calls and
  then removes `app_ctx.repos`. `check-handler-repos` does not (and should not) block on these —
  it only forbids the method-call bypass this plan closed.
- Plan D inverts `ActivityPubPort` so `presentation` drops its one remaining adapter dependency,
  `activitypub`.
- The pre-existing `follow_backfill_handler.rs` federation-gate bug (recorded in ADR-0005's
  follow-ups) is untouched by this plan.
- `cargo test -p composition` in isolation fails one test, `worker_deps_wiring`, with
  `unreachable code: worker_deps_wiring needs the sqlite feature; cargo test --workspace enables
  it transitively via presentation/worker's default features` — pre-existing, not introduced or
  fixed by this plan, and invisible to `make check`'s workspace-wide `cargo test`, which enables
  the feature transitively.

---

Superseded in part by [ADR-0012](0012-sqlite-is-the-only-backend.md): the postgres backends, crates and features mentioned above no longer exist; SQLite is the only backend.
