# InstanceIdentity owns URL derivation; social queries decompose

ADR-0002 and ADR-0003 unified local and federated social operations, but left `base_url`
travelling as a bare `&str`: a parameter on `FollowQuery` methods, a field on
`CompositeSocialAdapter`, and an argument to `SocialIdentity::from_actor_url`,
`format_local_handle` and `host_from_base_url`, scattered across many call sites. With no
owner, each layer re-derived URLs its own way — and `handlers/social.rs` derived
`local:{uuid}`, which `from_actor_url` could not parse, so every POST echoing a local actor's
url back resolved to `Remote` and was routed to ActivityPub. Accepting a local follow request
through the SPA was broken.

## Decision

- `InstanceIdentity` (`crates/domain/src/value_objects/instance.rs`) is a domain value object
  owning `actor_url_for`, `handle_for`, `image_url_for`, `actor_url_of`, `identify` and `host`.
  `SocialIdentity`'s static URL helpers (`from_actor_url`, `format_local_handle`,
  `host_from_base_url`) are deleted so the duplication cannot return.
- AP object URLs — review, goal and watchlist-entry URLs — deliberately stay out of
  `InstanceIdentity` and live in the ActivityPub adapter's `urls.rs`
  (`crates/adapters/activitypub/src/urls.rs`), which takes `&InstanceIdentity` and builds on
  `instance.base_url()`/`instance.actor_url_for()` rather than re-deriving. A review, goal or
  watchlist entry isn't an identity of this instance the way an actor URL or handle is — it's a
  content object's location — so folding them into the value object would turn it into a general
  URL grab-bag instead of an identity abstraction. `actor_url` is the one exception that
  delegates straight to `instance.actor_url_for(...)`, so there is exactly one construction of
  an actor URL in the codebase. The five ActivityPub adapter structs (`ActivityPubEventHandler`,
  `GoalObjectHandler`, `ReviewObjectHandler`, `DomainUserRepoAdapter`, `WatchlistObjectHandler`)
  and `handlers/users.rs`'s image-serving endpoints now hold/use `InstanceIdentity` instead of a
  bare `base_url: String`, closing the gap the whole-branch review found: this ADR's scope claim
  now holds for the entire codebase, not just the follow/social/profile-identity paths that
  landed first.
- `handle_for` takes `&str`, not `&Username`. Database rows carry unvalidated
  `Option<String>` usernames; requiring a `Username` would force revalidation of data that is
  already stored and already trusted.
- Local actors are identified on the wire by their canonical AP actor URL,
  `{base_url}/users/{uuid}` — what the database already stores. `identify(actor_url_of(id)) ==
  id` is a tested domain property (`instance_actor_url_round_trips_both_variants` in
  `crates/domain/src/tests/value_objects.rs`).
- Adapters hold an `InstanceIdentity` rather than receiving `base_url` per call; it is the same
  value on every call. `CompositeSocialAdapter` and `GetProfileDeps` both carry one.
- `SocialQuery` splits into `FollowGraphQuery` and `BlockQuery`, along the seam
  `CompositeSocialAdapter` already showed: `get_blocked` is the only method that reaches k_ap
  (`ap_service.get_blocked_actors`); every `FollowGraphQuery` method — including the new
  `get_relation` — goes through the local `follow_query` SQL port instead. `is_following ->
  bool` becomes `get_relation -> FollowRelation`, carrying both directions
  (`following`/`followed_by`) and distinguishing pending from accepted. `get_relation`
  propagates sqlx decode errors as `DomainError::InfrastructureError` rather than folding them
  into the same `None` that means "no follow edge" — the original snippet used
  `.ok().flatten()`, which would have reported a broken query as a stranger.
- Social **queries** become one file per use case (`get_followers.rs`, `get_following.rs`,
  `get_blocked.rs`, `get_pending_followers.rs` under `crates/application/src/social/`),
  matching `diary/` and `users/` and making ADR-0002's one-file-per-use-case claim actually
  true — it described a pattern the query side never had until now. `SocialQry` and
  `execute_query` are deleted outright, along with the `queries.rs` file that held nothing
  else.
- The `SocialCmd` **command** dispatcher (`crates/application/src/social/execute.rs`) stays as
  a single `execute_command` matching on an enum. Every branch is genuinely "call port, return
  event" and `execute_command` publishes the event — a real, uniform abstraction, not an
  accident of history the way the query enum was.
- `get_relation`'s application-layer wrapper is deferred, not built. The port method exists and
  is tested (against `InMemorySocialRepository`; neither SQL adapter's query is exercised yet),
  but nothing calls it until a follow-up plan adds a `/social/relationship` endpoint — building
  an unused public function now would be overbuilding.
- The dead `SocialCommandDeps.social_query` field was removed. `execute_command` never read it;
  every construction site was cloning an `Arc` nothing consumed.
- `get_profile` (`crates/application/src/users/get_profile.rs`) now assembles the whole
  `ProfileIdentity` — username, display name, bio, handle, actor URL, avatar/banner URLs — from
  the `User` entity and `InstanceIdentity`, rather than leaving the handler to fill in `None`
  for fields it didn't have. `display_name` and `bio` are populated for the first time; the API
  had always returned `None` for both despite the entity exposing them.
- `get_profile`'s missing-local-row case was deliberately tolerant, not `NotFound`.
  `build_federated_profile_response` built its own `GetProfileDeps` and called `execute()` a
  second time with a federated `user_id` that has no row in the local `users` table — there are
  three `execute` call sites in `handlers/users.rs`, not one, and the file aliases the import as
  `get_user_profile_uc::execute`, which hides two of them from a naive grep. Returning
  `NotFound` here would have broken federated profile viewing outright. The tolerant branch was
  covered by a permanent regression test,
  `tolerates_a_user_id_with_no_local_row_for_federated_style_calls`.
  - **Known wart, closed:** the tolerant branch filled `username` and `handle` with empty-string
    sentinels, but `actor_url` was still a well-formed-looking `{base_url}/users/{uuid}` pointing
    at a user that does not exist locally — one field obviously wrong, one plausibly wrong. It
    was inert only because the federated handler never read `profile.identity` (it built its own
    handle/actor_url from the resolved `fed` actor). A follow-up refactor (see the C1
    bypasses-and-server-split plan, task 1) made that "moment any caller starts consuming
    `profile.identity` on the federated path" arrive: `get_profile` split into
    `users::get_local_profile` — `Err(DomainError::NotFound(_))` for a missing local row,
    `identity` always fully populated — and `users::get_federated_profile_stats`, whose return
    type (`FederatedProfileStats`) carries no `identity` field at all, because the federated
    handler never read one. The old regression test now lives as
    `get_federated_profile_stats::tests::succeeds_for_a_user_id_with_no_local_row`, and a new
    one, `get_local_profile::tests::get_local_profile_is_not_found_for_unknown_user`, pins the
    behavior change directly. No type in the codebase can represent the old sentinel anymore.
    ADR-0006, covering the rest of the bypasses-and-server-split plan, cross-references this
    closure.

## Considered Options

- **Teach `from_actor_url` to parse `local:{uuid}`** — rejected because it keeps two encodings
  for one concept and lets the API's on-the-wire form diverge from the stored and federated
  form.
- **Pass `base_url: String` in deps structs and keep the static helpers** — rejected because it
  fixes the symptom while leaving the primitive threaded through every layer and the duplicated
  `format!` calls in place.
- **A port for instance identity** — rejected as needless indirection over a static config
  value; a value object is testable without mocking.
- **Decompose commands as well as queries** — rejected because the command enum's uniform
  "call port, return event" shape is a real abstraction, and splitting it would produce seven
  near-identical files plus a dissolved event-publishing seam. The query enum was the one worth
  breaking up because it forced every query into a single `Vec<SocialActor>` return type, which
  a count or a relationship could not fit.

Footnote, unrelated to the design decisions above: writing this ADR discovered that `.gitignore`'s
`docs/` + `!docs/adr/` pattern never actually re-included anything under `docs/adr/` for new
files — excluding a directory outright stops git from descending into it, so the negation was
dead on arrival. It silently swallowed `docs/adr/0001-general-review-editing.md`, which existed
on disk with zero git history until this ADR's landing fixed the pattern to `docs/*` +
`!docs/adr/` and recovered it in the same commit.
