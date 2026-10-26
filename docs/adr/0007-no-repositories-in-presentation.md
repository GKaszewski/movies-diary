# `AppContext` carries no `Repositories`; `composition` is a dev-only dependency of `presentation`

ADR-0006 closed sixteen call sites where a presentation handler called a repository method
directly, but explicitly left `AppContext.repos` in place: ~40 sites still read
`state.app_ctx.repos.<field>.clone()` to pass an `Arc` positionally into a use case that took
individual params rather than a deps struct, and migrating all of them at once would have made
ADR-0006's handler-extraction change harder to review and revert independently. `check-handler-repos`
was written narrowly on purpose, forbidding only the direct-call bypass ADR-0006 closed, not the
clone-and-pass pattern this plan (Plan C2) exists to remove.

Plan C2 did that migration in five tasks. Tasks 1-3 converted the 23 use cases the plan's initial
audit found reading an `Arc` cloned from `Repositories` — across `integrations`/`webhook`, `wrapup`,
`watchlist`, `import`, `users`, `movies`, `goals`, `auth`, and `search` — from individual `Arc`
parameters to a single `&FooDeps` struct each, registered on `application::Deps` and constructed by
`composition::build_deps`, following the convention ~30 other use cases already used. Task 4 replaced
the last two direct repository calls in the codebase — `extractors.rs`'s `AdminApiUser` and
`AdminUser` both called `repos.user.find_by_id(...)` inline to check admin status — with a new
`users::authorize_admin` use case, closing the gap ADR-0006 honestly scoped as handlers-only (these
were two axum extractors, not handler functions).

Task 5 widened `check-handler-repos` from `handlers/` to all of `crates/presentation/src`, and reported
zero offenders. That measurement was wrong. Its pattern was `repos\.[a-z_]+\.clone\(\)` — built to
catch the clone-and-pass shape the plan had inventoried — and it could not see two other shapes present
in the same tree: a bare reference passed by argument (`&state.app_ctx.repos.diary`, no `.clone()`
anywhere) and a `let`-bound access (`if let Some(ref fed_query) = ...repos.federated_profile`,
followed by a method call on the *binding* rather than on `repos` itself). `diary::get_diary`,
`diary::get_review_history`, and `diary::export_diary` all took their repository argument **by
reference** rather than owned, so their three call sites (`handlers/diary.rs`, `handlers/rss.rs`,
`handlers/movies.rs`, `handlers/helpers.rs`) were structurally invisible to a regex anchored on
`.clone()`. `handlers/users.rs`'s federated-profile lookup bound `repos.federated_profile` with `if
let` and called `.get_federated_profile(...)` on the bound variable, which no `repos.<field>.<method>(`
pattern can match across a `let`. The first attempt at this task found the five live sites by
inspection when it went to delete the field the guard was meant to make safe to delete, and correctly
returned blocked rather than deleting `AppContext.repos` out from under them; a new Task 7 then
converted all five (plus a sixth, non-`repos` internal caller broken purely by `get_diary`'s signature
change) before this attempt at Task 6 ran.

## Decision

**`AppContext` no longer has a `repos` field, and `presentation` no longer depends on `composition`
in production.** `pub repos: Repositories` and the `pub use composition::Repositories;` re-export are
both deleted from `crates/presentation/src/context.rs`. There is no route by which any code in
`crates/presentation/src` (outside `src/tests/`, which builds its own `Repositories` purely to drive
`composition::build_deps` for test fixtures) can reach a repository directly — not a stored field, not
an import, not a re-export. Every dependency a handler or extractor needs now comes from
`state.app_ctx.deps`, a prebuilt `Arc<application::Deps>` assembled once at the composition root.

`crates/presentation/Cargo.toml` moves `composition` from `[dependencies]` to `[dev-dependencies]`.
After the field deletion, `composition` is referenced only from `src/tests/extractors.rs`, which calls
`composition::build_deps` to produce a real `Deps` for test fixtures — a legitimate use of the
composition root's public API, just not one production code needs. Confirmed both ways:
`cargo tree -p presentation -e normal --depth 1` shows no `composition` anywhere in the production
dependency tree; `cargo tree -p presentation -e dev --depth 1` shows it as a direct dev-dependency.
`crates/server` (ADR-0006's home for the binary and every backend-selection feature) already depended
on `composition` directly in `[dependencies]` — that entry is untouched by this task — because it is
the actual composition root, not `presentation`. It still builds a `Repositories` and still needs
`AppContext { deps, .. }` without a `repos` field, so this task's field deletion required two small
production-adjacent fixes outside `crates/presentation` that no earlier task in this plan touched:
`crates/server/src/main.rs::wire_dependencies` (production) and `crates/server/tests/api_test.rs`
(its integration-test harness) both used to write `presentation::context::{AppContext, Repositories}`
and construct `AppContext { repos, deps, .. }`; both now import `Repositories` from `composition`
directly and drop `repos` from the struct literal, keeping the local `repos` binding alive only long
enough to pass `&repos` into `composition::build_deps`, the same pattern `presentation`'s own test
helper uses.

**All use cases that used to read an `Arc` out of `AppContext.repos` — 23 converted in Tasks 1-3, three
more (`get_diary`, `get_review_history`, `export_diary`) converted in Task 7 once the guard's blind
spot surfaced them, plus two newly-created use cases (`users::authorize_admin` in Task 4,
`users::get_federated_profile` in Task 7) — now take a single deps struct.** The two `extractors.rs`
admin checks (`AdminApiUser`, `AdminUser`) both now call `users::authorize_admin::execute`, returning
`Result<Option<bool>, DomainError>` so "user not found" and "repository call failed" stay
distinguishable by construction rather than by pattern-matching a `DomainError` variant an adapter
happens not to raise today. `handlers/users.rs`'s federated-profile branch now calls
`users::get_federated_profile::execute`, which mirrors `Repositories::federated_profile`'s own
`Option`-typed optionality (federation can be configured off) rather than inventing a new way to say
"this binary doesn't need it." Both migrations preserve their call sites' exact rejection behavior on
every path (missing port, repository error, empty result, found result) — verified path-by-path
against the pre-migration code in Task 4's and Task 7's own reports, not asserted here.

**Two use cases remain on individual params, correctly, because they were never reachable through
`AppContext.repos` in the first place:** `movies::request_enrichment::fetch_if_stale` and
`movies::enrich_movie::execute`, both called only from `MovieEnrichmentHandler`
(`crates/application/src/movies/event_handler.rs`), a `domain::ports::EventHandler` implementor that
the composition root constructs directly with individual `Arc` fields, the same way any adapter is
wired — never through `state.app_ctx.deps`. `fetch_if_stale` takes fully individual parameters
(`&dyn MovieEnrichmentClient`, `&Arc<dyn MovieProfileRepository>`, plus two value params).
`enrich_movie::execute` actually does take a struct, `EnrichMovieDeps` — but one built locally inside
`MovieEnrichmentHandler::handle` from its own already-held `Arc` fields, not a struct registered on
`application::Deps` or constructed by `composition::build_deps`. Neither shape is what this ADR means
by "use cases now take deps structs"; both are correctly out of scope for the same reason `Services`-
sourced use cases are (see below), and the plan's original note that they're "adapter-called, don't
migrate them" holds up.

**This ADR's "use cases take deps structs" claim is scoped to what Plan C2 touched: use cases that
used to read `AppContext.repos`.** It is not a claim that every `execute` function in the codebase
takes an `application::Deps`-registered struct. Two examples outside that scope, found while
fact-checking this ADR, are worth naming so nobody mistakes their shape for a regression:
`diary::log_review::execute` is called directly by two presentation handlers
(`handlers/diary.rs::post_review` and `::create_review`) with an individual
`&state.app_ctx.services.review_logger` argument — legitimate, because `review_logger` comes from
`AppContext.services`, a field this plan never touched, not from `repos`. `wrapup::compute::execute`,
`import::cleanup::execute`, and `integrations::cleanup::execute` also take individual params, but none
of the three is ever called from a presentation handler — `compute` is called internally by
`wrapup::handle_requested`, and the two `cleanup` use cases are called only from
`crates/application/src/jobs/`, the background-job runner. All four were outside Plan C2's stated
scope before this task started and remain untouched by it.

**`check-handler-repos` is now a single whole-crate rule: any mention of `app_ctx.repos` outside
`src/tests/` is an offender.** The two-pass shape-matching regex (`repos.<field>.<method>(`,
`repos.<field>.clone().<method>(`) is gone. It is gone because it is no longer needed, not because it
was simplified for its own sake: with `AppContext.repos` deleted, there is no shape of legitimate
production access left to distinguish from an illegal one — no clone-as-argument to permit, no direct
call to forbid separately. A single substring match is sufficient and, unlike the old guard,
unevadeable: it cannot be blind to a reference pass or a `let`-binding the way the shape-matching guard
demonstrably was (see above), because it does not attempt to parse the shape of the access at all — it
only asks whether the now-nonexistent field is mentioned. Verified by injecting `app_ctx.repos.user`
into `extractors.rs` (a non-test production file) and confirming `make check-handler-repos` fails,
naming the exact file and line, before reverting.

## Considered Options

- **Leaving the shape-matching two-pass guard in place, just widened to the whole crate** — this is
  what Task 5 actually did, and it is what let three reference-pass sites and one `let`-binding site
  through with a clean ✅. Once `repos` no longer exists, keeping the old guard's shape-matching would
  mean maintaining two regex passes to catch a category of mistake (any mention at all) that a single
  `grep` already catches completely. Rejected as strictly worse than the simple rule, not merely
  equivalent to it.
- **Deleting `check-handler-repos` entirely, since the compiler now enforces the invariant** — a
  production file referencing `app_ctx.repos` simply fails to build once the field is gone, so the
  guard is redundant with `cargo build` in the narrow sense of "will this compile." Rejected because
  the guard is cheaper to run (`grep` vs. a full workspace build) for `make check`'s fast-path checks,
  and because it fails with a direct "here's the offending line" message rather than a compiler error
  several frames removed from the actual mistake (e.g. a struct-literal error at the `AppContext`
  construction site rather than at the stray reference itself).

## Known follow-ups, out of scope here

- Plan D inverts `ActivityPubPort` so `presentation` drops its one remaining adapter dependency,
  `activitypub` — untouched by this plan, exactly as ADR-0006 recorded.
- The pre-existing `follow_backfill_handler.rs` federation-gate bug and the isolated
  `cargo test -p composition` `worker_deps_wiring` failure (both recorded in ADR-0006) are unaffected.
- `diary::log_review`, `wrapup::compute`, `import::cleanup`, and `integrations::cleanup` remain on
  individual params, correctly — none of them ever read from `AppContext.repos`, so none was ever in
  this plan's scope. Whether they *should* eventually move to deps structs for consistency with the
  rest of the codebase is a separate, unstarted question this ADR takes no position on.
- `CONTRIBUTING.md`'s architecture rules were updated in this same commit to stop describing use cases
  as taking `&AppContext` (a claim already false before this plan) and instead describe the
  `&FooDeps`-struct convention this ADR documents.

Now at `crates/adapters/http-axum/src` (was `crates/presentation/src`); see [ADR-0014](0014-http-adapter-and-one-refusal-shape.md).
