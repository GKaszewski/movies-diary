# Account deletion is a tombstone now and a purge later

Deleting an account has to take effect at once (the person is gone from the instance and the
fediverse) yet erase a lot: dozens of tables, images in object storage, follows on other servers,
and signed `Delete` deliveries that need the account's key long after the user row would
normally be gone. Doing it in one request would hold a transaction over federation and storage
calls, and a crash halfway would leave a half-deleted person.

## Decision

**Two phases.**

1. *Request* (`request_account_deletion`). `UserRepository::mark_deleted` runs one transaction
   that stamps `users.deleted_at` (the tombstone), inserts the username into `retired_usernames`,
   deletes the user's `refresh_sessions`, and writes a `UserDeleted` event to the outbox
   (ADR-0015). Migration `0102_account_deletion.sql` adds `users.deleted_at`, `users.purged_at`
   and `retired_usernames` (`username` primary key, `COLLATE NOCASE`, `retired_at`).
2. *Purge* (`AccountPurgeHandler`, handler name `account_purge`, runs in the worker on
   `UserDeleted`).

**Authorization.** A user deletes themselves, an admin deletes anyone (others get 403;
`DELETE /api/v1/users/{id}` takes `AuthenticatedUser` and the use case enforces the role, so
there is one policy point). The proof is always the *actor's* password: the user's own, or the
admin's. A wrong password is `DomainError::InvalidCredentials`: 401 with code
`INVALID_CREDENTIALS` (ADR-0014), distinct from `UNAUTHORIZED` so the SPA does not mistake it for
an expired session and refresh and retry. The last active admin cannot be deleted. The guard
is in the `mark_deleted` `UPDATE` itself (the row is stamped only if another non-deleted admin
exists), so two admins deleting each other concurrently cannot both succeed; the use case's
`count_active_admins` check is only the early, friendly refusal (409).

**Immediate effects, before the purge runs.** Login is refused and tokens are rejected
(`ensure_active` in the extractors; the user lookups filter `deleted_at IS NULL`). The user's
webhook tokens stop ingesting at once (`find_by_token_hash` ignores tokens of tombstoned users,
so the endpoint answers 401). The profile
and `?embed=true` pages 404. The ActivityPub actor document, inbox, outbox and featured answer
410 (`tombstone_gate` for every `/users/{id}/...` path, content objects included, and
`actor_is_gone` for the actor). WebFinger
answers 404 (owned by k-ap).

**The service actor is not deletable.** The instance's service actor (`UserId::service_actor()`,
seeded by migration 0022, used for signed fetches) is refused by `request_account_deletion`
(403 `Forbidden`), and both UIs hide the action (`is_service_account` on the profile response;
the classic template checks the same). Deleting it would retire its name and, after the
retention period, its keypair, breaking signed fetches for good.

**Purge.** Order matters and every step is idempotent:

1. Notify the fediverse through `FederationDeletionNotifier`, implemented over k-ap's
   `broadcast_delete`. That only queues signed deliveries. A notifier failure is logged and never
   blocks the purge, and the notice is skipped once the account is purged.
2. `AccountPurger::purge`: one transaction first strips the `users` row (which also claims the
   purge: only a tombstoned, unpurged account matches), then deletes `ap_announces` of the
   user's content and by the user, the *finished* `event_queue` rows whose payload names the
   user, other accounts' follows of this actor (`ap_followers`/`ap_following` by actor URL), and
   every row in `PURGED_USER_TABLES` (including tables with no foreign key to `users`, such as
   `watch_events`, `wrap_up_records`, `import_*`, `blocked_actors`). `PURGED_USER_TABLES`,
   `URL_KEYED_USER_COLUMNS` and `PAYLOAD_KEYED_USER_COLUMNS` are checked against the schema by a
   guard test, so a new user-owned table cannot be forgotten.
   Global wrap-up reports (`wrap_up_records` with no user) embed `most_active_user`; the purge
   sets it to null in any report naming the user and keeps the rest (`wrap_up_records.report_json`
   is in `PAYLOAD_KEYED_USER_COLUMNS`).
   A failed purge is redelivered and queues the `Delete` again; a repeated `Delete` is
   idempotent on remote servers.
3. Image objects (avatar, banner) are deleted after the commit, then the references are dropped
   (`forget_images`), so a crash in between is repaired by redelivery.

**The tombstone.** The stripped `users` row keeps `id`, `username`, `role` and the `ap_local_actors`
keypair (`RETAINED_USER_TABLES`), and nothing else: the email becomes the placeholder
`deleted-<id>@deleted.invalid` (which frees the real address for a new registration), the
password hash becomes `!purged`, display name, bio, `also_known_as` and image paths are cleared,
and `purged_at` is set. The row stays so queued `Delete` deliveries can still be signed.
Broadcasts for a tombstoned author are suppressed (the handler skips them and succeeds) except
the `Delete` itself.

**Cleanup.** `TombstoneCleanupJob` runs daily and removes tombstones whose `purged_at` is older
than `TOMBSTONE_RETENTION_DAYS` (7): all remaining `event_queue` rows naming the user (the
`UserDeleted` trigger and the `Delete` deliveries), the keypair, and the `users` row. A tombstone
that an unfinished (pending/processing) event still names waits for the next run; unfinished
events are never deleted.

**Usernames are never reused.** `retired_usernames` is permanent and survives cleanup; the
registration path refuses a retired username, so a deleted name cannot be impersonated.

## Known gaps

- Events already relayed to NATS JetStream are not erased; they live as long as the stream's
  retention allows.
- A Follow arriving on the shared inbox can create a `Pending` `ap_followers` row for the
  tombstoned user; `purge_expired_tombstones` deletes the user's `ap_followers` and
  `ap_following` rows explicitly before the `users` row (a test covers it), not by relying on
  the foreign-key cascade.
- Remote servers that stay down through the delivery retries, or that ignore `Delete`, may keep
  stale copies.
- A handler that defers a tombstone-naming event forever delays the tombstone (and keypair)
  removal. No event is lost, and nothing logs this case.

## Consequences

- Deletion is visible instantly and has no undo; once the purge has run the data is gone.
- Federation and storage failures cannot strand an account half-deleted: the purge is
  redelivered and each step tolerates a re-run.
- The worker owns the destructive work, so the server needs no new infrastructure beyond the
  outbox it already writes to.
