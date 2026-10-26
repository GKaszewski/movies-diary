# Federation is optional at the dependency level, not just in code paths

The `federation` feature has existed since ADR-0002 and has been treated by every ADR since as the
switch that decides whether this instance speaks ActivityPub. It did not do that. It gated code
paths, HTTP routes and struct fields; it removed nothing from the build. Measured at this branch's
merge-base `f6ff7dd` with `cargo tree -p server -e normal --prefix none`, deduplicated by
`sed 's/ (\*)$//' | awk '{print $1" "$2}' | sort -u | wc -l`:

| Feature set at `f6ff7dd` | unique crates | `activitypub_federation` |
| --- | --- | --- |
| `sqlite` | 357 | present |
| `sqlite,sqlite-federation` | 357 | present |
| `postgres` | 368 | present |
| `postgres,postgres-federation` | 368 | present |

Identical, to the crate. The cause was one unconditional manifest edge per backend:
`crates/adapters/sqlite/Cargo.toml:16` read `sqlite-federation = { workspace = true }`, and
`crates/adapters/postgres/Cargo.toml:16` the same for `postgres-federation`. Those crates depend on
`k-ap`, which depends on `activitypub_federation`. Anything that wanted a database therefore got the
whole ActivityPub stack, and `--no-default-features --features sqlite` compiled, linked and shipped
it. The feature was a runtime posture, not a build-time decision.

Two consequences followed from that, and only the first was known. The first is size: an operator who
does not want federation still pays for it in compile time, binary size and dependency surface. The
second needs more care than an earlier draft of this document gave it: it is a read-path bug, not the
write-path bug it first looked like.

Because the whole social stack lived on one struct inside the federation crate, a federation-off
`crates/server` had nothing to wire `SocialCommand`/`FollowGraphQuery`/`BlockQuery` to, so it wired
`NoopSocialCommand`/`NoopSocialQuery` — `follow` returned `Ok(())` and wrote nothing, every query
returned `0` or `vec![]`. It is tempting to read that as "following a user on your own instance
silently did nothing," directly contradicting ADR-0003. **That reading is wrong: nothing federation-off
could ever reach `NoopSocialCommand::follow` in the first place.** Every social write route —
`/social/follow`, `/social/unfollow`, and everything else `SocialCommand` exposes — lives in
`federation_api_routes()` (`crates/presentation/src/routes.rs:485`) and `federation_html_routes()`
(`routes.rs:194`), both `#[cfg(feature = "federation")]`, merged into the router only under that gate
(`routes.rs:459`, `:188`). `crates/server/Cargo.toml` takes `presentation` with `default-features =
false`, so on a federation-off build `POST /api/v1/social/follow` is a 404 — the handler was never
compiled in. Nobody could follow anybody, successfully or silently, because there was no route to call.
`NoopSocialCommand::follow`'s silent `Ok(())` was a latent trap, genuinely worth removing, but it was
never a shipped bug: correct-looking code sitting behind a door that no federation-off request could
open.

`NoopSocialQuery` sat behind no such door. Three consumers call `FollowGraphQuery` through routes that
carry **no federation gate at all**: `get_local_profile` (follower/following counts on `/users/{id}`,
which is an ungated route at `routes.rs:76`), `get_page_viewer` (the pending-follower badge), and
`get_activity_feed` (the followed-user set behind the activity feed's `filter_following`). All three
read `deps.social_query`, which `crates/composition` wires from `repos.follow_graph` — before this
branch, `NoopSocialQuery`, federation-off. **A federation-off instance did not fail to follow; it lied
about its own follow graph on every profile view and every activity-feed request** — reporting zero
followers, zero following, no pending requests, no followed users, regardless of what
`ap_followers`/`ap_following` actually held. That is the live bug this branch fixes, and it is the read
half, not the write half: the write half was unreachable over HTTP, so its noop cost nothing; the read
half was reachable everywhere, so its noop cost the truth.

## Decision

**A `<backend>-social` crate speaks only `domain::ports`; a `<backend>-federation` crate speaks
ActivityPub; the former must never depend on the latter.** `crates/adapters/sqlite-social` and
`crates/adapters/postgres-social` are new and hold the implementations of the **seven** *domain* ports
that the federation crates used to carry, across three types. `SqliteSocialRepository` /
`PostgresSocialRepository` carry five — `FollowCommand`, `FollowQuery`, `FederatedProfileQuery`,
`RemoteWatchlistRepository`, `FederationAdminQuery` — and two more sit on their own single-purpose
types: `SqliteApContentQuery` implements `LocalApContentQuery` (`sqlite-social/src/ap_content.rs:9`),
and `SqliteRemoteGoalRepository` implements `RemoteGoalRepository`
(`sqlite-social/src/remote_goals.rs:18`). This is not cosmetic: those last two are exactly what the base
`sqlite` crate re-exported from the federation crate before this split
(`crates/adapters/sqlite/src/lib.rs:35`, `:121`) — they are the answer to "why did `sqlite` depend on
`sqlite-federation` at all." `sqlite-federation` and `postgres-federation` keep the twelve `k_ap` trait
impls — `ActivityRepository`, `KeypairRepository`, `RemoteActorCache`,
`AnnounceRepository`, `RemoteReviewRepository`, `DomainBlocklist`, `ActorBlocklist`, and the five
`follow/` traits. The base backend crates now depend on `-social` unconditionally and on `-federation`
not at all; the federation crates enter the graph only through the `sqlite-federation` /
`postgres-federation` features. That inversion is the whole mechanism: the feature is real because the
edge it gates is the only edge.

Measured on this branch with the same command and the same dedup pipeline:

| Feature set | unique crates | `activitypub_federation` |
| --- | --- | --- |
| `sqlite` | **282** | **absent** |
| `sqlite,sqlite-federation` | 358 | present |
| `postgres` | **293** | **absent** |
| `postgres,postgres-federation` | 369 | present |

Both backends drop exactly **76** crates when federation is off, and `grep -c activitypub_federation`
over the federation-off tree returns `0` for each. The on-counts are 358 and 369 rather than the old
357 and 368 because `sqlite-social` and `postgres-social` are themselves new crates in the graph;
against the old build, a federation-off deployment resolves 75 fewer crates.

**The 76 crates are visible in the shipped image, not only in `cargo tree`.** Both container images
were built and compared:

```
docker build --build-arg FEATURES=sqlite                   .   ->  171MB
docker build --build-arg FEATURES=sqlite,sqlite-federation .   ->  190MB
```

**19MB smaller.** This is the measurement that matters to an operator: a crate count is an argument
about the build graph, whereas the image size is the consequence someone actually deploys. It is also
the evidence that the decoupling reaches all the way through — a change that only moved `impl` blocks
around would leave both images the same size.

Note the second invocation is what plain `docker build .` does: **the Dockerfile's default is
`ARG FEATURES=sqlite,sqlite-federation`, i.e. federation-on.** The federation-off image — the entire
point of this ADR — is only built when asked for explicitly, so `docker build .` on its own does not
exercise it. Both commands above are the reproduction, and both should be run before a release that
claims the federation-off deployment works.

**The orphan rule forced the struct split, and this is the third consecutive plan in which it has
dictated a design.** `SqliteFederationRepository` was a two-field struct — `pool` and `instance` —
carrying **seventeen** trait impls: five owned by `domain`, twelve owned by `k_ap`. Moving five impls
to a new crate is not a matter of moving five `impl` blocks, because from inside `sqlite-social` both
`domain`'s traits and a type defined in `sqlite-federation` are foreign, which is E0117. The five
impls needed a type defined where they live, so `SqliteSocialRepository { pool, instance }` exists,
and `PostgresSocialRepository` likewise. ADR-0008 records the first two occasions
(`ApServiceAdapter`, `DomainUserRepoAdapter`); the pattern is now settled enough to state as a rule
of this codebase: **"move these impls to another crate" always implies "and define a type there to
carry them."** Plan for the type, not just the impls.

The two structs are deliberately *not* symmetric. `SqliteFederationRepository` lost its `instance`
field entirely — after the five domain impls left, nothing read it, and the alternative on the table
was an `#[allow(dead_code)]`, which was reversed during review in favour of deleting the field and
updating its seven crate-internal call sites. `SqliteSocialRepository` keeps `instance`, because
`follow_repository.rs` reads it in eight places to build the actor URLs ADR-0003 specifies. The
asymmetry was measured before the postgres half was written, so `postgres-social` was built to the
corrected shape rather than copied from its sibling and then fixed.

**A non-recursive glob misclassified an entire subdirectory, and the correction is worth recording
because it nearly moved the wrong code.** The plan's first pass classified which files were
ActivityPub-free using a `src/*.rs` glob. That glob never descends into `src/follow/`, so `follow/`
was scored as clean and slated to move into the social crate. It is the opposite of clean: all five
of its traits — `FollowerWriter`, `FollowerReader`, `FollowingWriter`, `FollowingReader`,
`FollowMigration` — are `k_ap` traits, and its files import `k_ap::{Follower, FollowerStatus,
RemoteActor, ...}` directly. A recursive rescan before any code moved corrected the classification and
moved the split from a claimed 1333/359 line division to an actual 923/769. `follow/` stayed put.

The near-miss is easy to make because the *local* follow path really is ActivityPub-free — it is just
a different file. `crates/adapters/sqlite-social/src/follow_repository.rs` imports only `domain`,
`sqlx`, `chrono`, `async_trait` and `adapter_common`, names no `k_ap` type, and carries the
`FollowCommand`/`FollowQuery` impls. Two similarly-named things sit side by side: the AP follower
protocol in `follow/`, and ADR-0003's direct-SQL local follow in `follow_repository.rs`. That the file
layout already separated them cleanly is ADR-0003's design showing through — the split was legible
because that decision had been made years earlier. The lesson stands regardless: **scan recursively**.
This was the second wrong classification produced by a single grep pattern in the same session.

**The wiring is correct now; the read path is what a federation-off deployment actually gets today.**
`application::social::LocalSocialService` implements `SocialCommand`, `FollowGraphQuery` and
`BlockQuery` against the domain ports only, and `crates/server/src/main.rs` wires one instance of it
across all three when `federation` is off, replacing three separate noops. The bodies were ported from
`CompositeSocialAdapter`'s local branches rather than rewritten, and were compared token-by-token
against the originals during review — the two copies coexisted and agreed for three commits, which is
what made that comparison possible. `CompositeSocialAdapter` then delegates its local branches to
`Arc<dyn LocalSocial>` and shrank from 299 to 207 lines; every surviving `SocialCommand` line in it is
either a delegation or genuinely ActivityPub-specific.

What this buys a federation-off build, concretely, is the read half described above:
`get_local_profile`, `get_page_viewer` and `get_activity_feed` now read real
`ap_followers`/`ap_following` rows through `LocalSocialService` instead of `NoopSocialQuery`'s
always-empty answer. **An instance rebuilt federation-off stops lying about its own follow graph** —
that is the user-visible improvement this branch ships, and the ADR previously did not name it.

The write half — `SocialCommand::follow`/`unfollow`/`accept_follow`/`reject_follow` — is now wired
correctly and errors honestly for a remote target (below), but it has **no HTTP surface** federation-off:
the social routes that would call it are federation-gated in `presentation`, as established above, and
this branch does not touch that gating. The wiring exists so it is correct on the day those routes are
ungated, not because anything reaches it today. Ungating the local-only social routes — splitting
`federation_api_routes`/`federation_html_routes` into a local-only half (follow, unfollow, accept/reject,
the counts and lists) and an AP-only half (blocks, remote admin queries, the AP follower/following
collections) — was raised and considered during review. **Decision: do not ungate them here.** It is a
deliberate non-goal of this branch and a candidate follow-up, not a gap in it: the split is a
presentation design task nobody has done or reviewed, since the two halves are currently interleaved
one function each, and getting it right means deciding case-by-case which of
`get_followers`/`get_following`/`get_relationship`/etc. are actually AP-agnostic versus AP-flavored in
their response shape. Recorded in the follow-ups below.

**A remote target on a federation-off build now errors, and the error names the actor:** `cannot
reach remote actor {actor}: this instance was built without the federation feature`
(`crates/application/src/social/local_service.rs:74`). This is the branch's one intentional behaviour
change and lands in its own commit (`b8f74eb`). The verb is "reach", not "follow", because the same
constructor serves five call sites including unfollow and unblock, and "cannot follow" would
misdescribe four of them. At the four non-follow sites `{actor}` is whatever `actor_url_of` returns,
which for a persisted row is an AP URL rather than an `@user@host` handle; the message says "remote
actor", not "handle", so both read correctly.

**The wiring fix is guarded by a Makefile grep rather than a test, and that is a limitation, not a
preference.** The decision lives in `wire_dependencies()` — a private `async fn` in
`crates/server/src/main.rs`, a *binary* crate. Binary crates expose no library API, so nothing under
`crates/server/tests/` can call it; `server/tests/api_test.rs` builds `AppState` and `AppContext` by
hand with its own fakes and never exercises the real wiring either. Making it testable means splitting
`server` into lib+bin, a structural change well outside this work. `LocalSocialService`'s *behaviour*
has five unit tests from earlier in the branch; what has no test is the *wiring*, and this repo
already expresses that class of rule as a Makefile grep guard — four existed, and ADR-0006/0007
record the convention. `check-federation-off-social` is therefore the idiomatic instrument here rather
than a consolation prize. It was observed red (three hits at `main.rs:161,164,167`) before the fix and
green after, and is wired into `make check` (`Makefile:4`), `.PHONY`, and line 47 of both
`.github/workflows/ci.yml` and `.gitea/workflows/ci.yml`. `make check` now runs **six** guards: the
five above, plus `check-social-crates-are-ap-free`, added during this review to enforce the invariant
the whole federation-optional property rests on — a `<backend>-social` crate must never depend on
`k-ap`, `activitypub`, or anything matching `*-federation`. A single manifest line
(`k-ap = { version = "0.5.0", registry = "gitea" }` in `sqlite-social/Cargo.toml`) would silently
restore all 76 crates and 19MB while `make check`, `cargo test` and CI stayed green without it.
`check-federation-off-social` was also strengthened in the same review: it previously only asserted the
noops were *absent* from `main.rs`, which passes even if `LocalSocialService` were deleted from the same
file and nothing were wired at all. It now also asserts `LocalSocialService` is *present*.

Its scope is a single file, which is narrower than its siblings and is now documented in the recipe:
`crates/server/src/main.rs` is the sole wiring site, and `worker` and `tui` do not name
`SocialCommand`, `FollowGraphQuery` or `BlockQuery` at all — verified by recursive grep over both
crates' `src/`, zero hits. Relocating the wiring out of `main.rs` would move it out of the guard's
scope; the comment says to widen the guard to `crates/server/src` if that ever happens.

**Both halves share the `ap_followers` and `ap_following` tables, and neither owns the migrations.**
Those tables are created by `crates/adapters/sqlite/migrations/0003_activitypub.sql` and
`crates/adapters/postgres/migrations/0001_initial.sql` — the *base* backend crates, which both halves
already depend on. This is ADR-0003's arrangement unchanged: a local follow writes the same rows in
the same format as a remote one, which is what makes local relationships visible to the fediverse
through the AP collection endpoints without any duplication. The consequence for maintenance is that
**a schema change to either table touches both crates**, and neither crate's directory is where you
would look for the migration. This is a real coupling, accepted deliberately, because the alternative
— a separate `local_follows` table — was rejected in ADR-0003 as two sources of truth for one concept.

**A latent bug in `worker` was exposed, not caused, by cutting the dependency edge.**
`crates/worker/src/main.rs` calls `tokio::signal::ctrl_c()` with no feature gate, but the workspace
`tokio` (`Cargo.toml:44`) declares only `macros, net, rt, rt-multi-thread, sync, time` — no `signal`.
It had been arriving transitively, through `sqlite → sqlite-federation → activitypub_federation`.
Removing that edge broke the worker build, which looked like collateral damage and was not: the worker
had a hard requirement satisfied by a dependency four hops away that it did not declare and had no
reason to expect. It would have broken the moment anything else in the graph shifted. Diagnosed by
stashing back to the pristine tree to confirm the transitive source, and fixed at the place the
requirement actually lives — `crates/worker/Cargo.toml:31` now reads
`tokio = { workspace = true, features = ["signal"] }`. This is the kind of thing a decoupling finds
that no amount of reading finds.

### What this ADR does not claim

**Blocks remain federation-only, and `LocalSocialService::block`/`unblock` always error.** There is no
domain port for block storage — blocking is `k_ap::ActorBlocklist` and `k_ap::DomainBlocklist`, both
`k_ap` traits on the federation struct — so making blocks work federation-off is a new port and new
tables, not a wiring change. It was not attempted.

**`LocalSocialService::get_blocked` returns `Ok(vec![])` federation-off — the same stale-row shape this
section already discusses at length for `FederationAdminQuery`, and worth naming rather than leaving
implicit.** `block`/`unblock` always error, so no block can be *recorded* through this path either way,
but an instance that accumulated blocks while federation-ON and was then rebuilt federation-OFF would
report none. Operationally harmless: block enforcement itself lives in `k_ap`, absent federation-off,
and the route that exposes this list (`/social/blocked`) is federation-gated, so nothing acts on the
stale answer. But it is the same class of claim as the paragraph below makes for
`FederationAdminQuery`, and this section should say so plainly rather than naming `block`/`unblock` and
skipping the query.

**`FederationAdminQuery` is still noop'd federation-off, returning an empty list, and the empty list
is an invariant rather than an assumption.** The weaker argument — "you cannot follow a remote actor
federation-off, so empty is honest" — is only plausible. The stronger one is provable from the wiring
this change installs: `LocalSocialService::follow_resolved` hard-errors on any non-`Local` target
(`local_service.rs:253-256`) and `block`/`unblock` always error, so a federation-off build **cannot
persist a remote follow through this path at all**. The empty list is enforced, not estimated. A
comment at the wiring site now records this so a future reader does not re-derive it.

There is one exception, and it is a display artifact rather than a functional regression: **an
instance that ran federation-ON, accumulated remote follow rows, and was then rebuilt federation-OFF
would report zero remote follows while the rows still exist.** Those follows cannot be acted on
without federation anyway, so nothing is lost operationally, but the admin view understates reality.
Wiring `FederationAdminQuery` for real is viable whenever that is judged worth surfacing — both
`-social` crates implement it, and both are present federation-off. It was left noop'd because doing
otherwise would surface rows the build cannot act on.

**"The federation-off builds compile" is not "they are clippy-clean."**
`cargo clippy -p worker --no-default-features --features sqlite -- -D warnings` still fails, with two
errors: unused `remote_goal` and `FollowBackfillHandler is never constructed`. ADR-0008 recorded
*three* here; the third, unused `app_config`, is incidentally gone because this branch hoists
`let instance` above `build_database_adapters` in both binaries and derives it from `app_config`
unconditionally. The remaining two predate this work and no gate runs this combination. Likewise
`cargo build -p server --no-default-features --features sqlite` emits one pre-existing warning,
unused `ap_content_repo` at `main.rs:65` (introduced at `ac7edd6`, long before this branch), and the
single-backend postgres builds emit unreachable-pattern warnings on `DbPool` matches that are
inherent to compiling a two-variant enum with one variant enabled. All warnings, none errors, none
new.

**The known-broken feature combinations ADR-0008 recorded are unchanged.**
`--features sqlite,federation` on `server` still fails — `federation` is a bare meta-feature that does
not imply `dep:activitypub` — and `cargo check -p presentation --no-default-features --all-targets`
still fails with the same two pre-existing errors. This branch neither fixed nor worsened either.

## Considered Options

- **Gate the existing `sqlite-federation` dependency behind the feature instead of splitting the
  crate** — the obvious one-line fix, and the reason it does not work is the whole point. The five
  domain-port impls the rest of the system needs unconditionally (`FollowCommand`, `FollowQuery`,
  `FederatedProfileQuery`, `RemoteWatchlistRepository`, `FederationAdminQuery`) lived on the same
  struct in the same crate as the twelve `k_ap` impls. Gating the edge would have removed those five
  too, and the `federation`-off build would have had no follow storage at all — which is precisely the
  state that produced the silent-noop bug. The dependency could not be made conditional until the code
  behind it was separated by what it speaks.
- **`#[cfg(feature = "federation")]` inside the existing crates rather than two new crates** —
  cheaper in file moves, and rejected because a cfg inside a crate does not remove the crate's
  manifest dependency on `k-ap`. `cargo tree` would have been unchanged, `activitypub_federation`
  would still resolve, and the headline measurement would still read 357/357. Conditional compilation
  cannot subtract a dependency edge; only the manifest can.
- **Keep the noops and document that federation-off means no social features** — internally
  consistent, and rejected because it contradicts ADR-0003 on the merits, not merely on the docs. That
  ADR's entire argument is that a local follow is two SQL writes and does not need ActivityPub. A build
  that omitted ActivityPub and therefore omitted local-follow *storage and querying* would prove
  ADR-0003 wrong; the split proves it right at the storage and service level — the HTTP surface for the
  write path remains gated regardless of which option was chosen here, see the follow-up below.
- **Put `follow_resolved` on `SocialCommand` instead of on a new sibling trait** — the smaller-looking
  option, rejected by enumerating implementors rather than by taste. Only `LocalSocialService` and one
  domain test stub satisfy all four original `LocalSocial` bounds; `CompositeSocialAdapter` and
  `InMemorySocialRepository` are real `SocialCommand` implementors that are *not* `LocalSocial` and
  could not meaningfully implement `follow_resolved`. Extending `SocialCommand` would have stranded
  two types. `LocalSocial` is now a marker supertrait over five traits with an unconstrained blanket
  impl, matching `k_ap::FollowRepository`'s precedent over its own five.
- **Resolve the follow target twice on the local path** — what the plan originally prescribed, and
  what the first implementation did: the composite called `resolve_target` to choose a branch, then
  handed the raw `FollowTarget` to `local.follow`, which resolved it again. Cost was a redundant
  `find_by_username` per handle-based local follow plus a race in which a user deleted between the two
  resolutions makes a *federation-ON* deployment emit "this instance was built without the federation
  feature", which is nonsense in that configuration. Replaced by `ResolvedFollow::follow_resolved`, so
  the composite passes the identity it already computed. The five `LocalSocialService` tests passed
  byte-unmodified across that change, which is the evidence that plumbing moved and behaviour did not.

## Known follow-ups, out of scope here

- **Ungating the local-only social routes** (`/social/follow`, `/social/unfollow`, accept/reject, the
  counts and lists in `federation_api_routes`/`federation_html_routes`) so a federation-off build gets an
  HTTP surface for the write path described above. Considered and deliberately deferred during review,
  not because it is undesirable but because it is unreviewed design work: it needs splitting those two
  functions into a local-only half and an AP-only half, deciding case-by-case which handlers are
  AP-agnostic versus AP-flavored, and nobody has done or reviewed that split. Until it happens,
  `LocalSocialService`'s write path is correctly wired and unreachable over HTTP federation-off.
- **The Dockerfile hand-enumerates every workspace member's `Cargo.toml`, and this branch broke it.**
  `docker build .` failed at `RUN cargo fetch` with `failed to read
  /build/crates/adapters/sqlite-social/Cargo.toml`, because the dependency-cache layer copies manifests
  one `COPY` line at a time and the two new crates had no lines. Fixed here by adding them and by
  verifying the list now matches `find crates -name Cargo.toml` exactly (37 against 37), with a
  comment saying that adding a crate means adding a line.

  **This is a standing trap, not a one-off, and it is still unguarded.** Adding any workspace crate
  silently breaks the container image, and the breakage is invisible to `make check`, to `cargo build`
  in every feature combination, and to CI as currently configured — only a full `docker build` finds
  it. This plan tripped it, and the project's history already records a Docker build broken across
  three merges for the same reason: nobody ran it. **Recommendation, not implemented here** (it is a
  separate decision): a `make` target that diffs the Dockerfile's `COPY` list against
  `find crates -name Cargo.toml` and fails on any difference would close it in a few lines, and is
  exactly the check that was run by hand to confirm this fix. A seventh guard is cheap; a fourth broken
  release is not.
- **The error message names the actor and the build-time cause, but not which mutation was attempted.**
  A user who unfollows and sees "cannot reach remote actor X: built without the federation feature"
  learns why but not what. Closing this needs either per-site verbs — which reintroduces five message
  strings to keep in sync — or route-level logging. Left open deliberately.
- **`FederationAdminQuery`'s stale-row case**, described above. Both `-social` crates already implement
  the port federation-off, so this is wiring, not new code, if the staleness is judged worth surfacing.
- **`crates/infra-wiring` (87 lines across two files) still exists to break a real cycle** — `DbPool`,
  `EventBusBackend` and `AppConfig` need to be visible to both `composition` and the binaries without
  `composition` depending on `application::config`. It looks like a candidate for dissolution and is
  not one; the cycle is genuine. Recorded so the next reader does not re-investigate.
- **`build_database_adapters`' two backend arms are near-duplicates** (`crates/composition/src/factory.rs:49`).
  Each now constructs a `*SocialRepository` alongside everything else, so the duplication grew slightly
  in this branch. Collapsing them needs a trait over the two backends' `wire()` outputs — real design
  work, not a tidy-up, and not attempted during a refactor whose contract was behaviour preservation.
- **`tests/local_service.rs`'s `service()` helper takes `user_repo` by value and `follow_store` by
  reference.** Pure polish, explicitly left alone.
- **`crates/adapters/` now holds 28 directories, up from the 26 ADR-0008 counted.**
  `check-presentation-adapters` iterates those directory names and still passes, so neither new crate
  leaked into `presentation`. ADR-0008's caveat carries over unchanged: the guard assumes directory
  name equals package name, true for all 28 today but not enforced.
- The deferred items in ADR-0005/0006/0007/0008 are otherwise unaffected by this plan.

---

Superseded in part by [ADR-0012](0012-sqlite-is-the-only-backend.md): the postgres backends, crates and features mentioned above no longer exist; SQLite is the only backend.

Now `make check-sqlite-is-ap-free` (was `check-social-crates-are-ap-free`); the sqlite social crates merged into `crates/adapters/sqlite`.
