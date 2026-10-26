# Events go through a transactional outbox and are delivered idempotently

Domain events used to be published after the write that caused them, from the use case, on a
channel the server owned. A crash between the write and the publish lost the event (no poster, no
federation delivery, no search index entry), and with NATS the server needed a live broker
connection just to log a review. Delivery was also at-least-once without any guard, so a redelivered
event ran its handlers twice.

## Decision

**The outbox is `event_queue`.** The same SQLite table the DB-backed bus already used is now the
outbox for every backend. Migration `0101` adds `event_queue.event_id` (UUID, unique when set) and
`claimed_at`, and creates `processed_events` (`event_id`, `handler`, `status`, `started_at`,
`processed_at`). The migration backfills every existing row with a random event id, so legacy rows get dedup too.
The column stays nullable, but the only insert path (`enqueue_events`) always sets it, and the
consumer still tolerates a NULL id (no dedup) for rows from foreign writers.

**Writes are atomic with their events.** Repository and command-port methods that change state take
the events they cause and write them in the same transaction as the data. `shared::enqueue_events`
is the single path that inserts outbox rows. Covered: review log/edit/delete (including orphan
movie deletion and `ReviewDeleted`/`MovieDeleted`), watchlist add/remove, movie discovery and
enrichment requests, wrap-up, integration writes, local follow/unfollow, remote review save.
Use cases never open transactions; the adapter does.

**The server never talks to NATS.** With `EVENT_BUS_BACKEND=nats` the server still only inserts
outbox rows. The worker runs a relay that claims pending rows and publishes them to JetStream with
`Nats-Msg-Id` set to the event id, then marks the row done. With `db` the worker consumes the rows
directly. `max_deliver` on the JetStream consumer equals `EVENT_QUEUE_MAX_ATTEMPTS`, and a JetStream
nak uses the same backoff as the SQLite queue (`domain::events::retry_backoff`: 5 s doubling per
attempt, capped at 300 s), so both backends give up after the same number of deliveries on the same
schedule. They differ in what giving up means. SQLite marks the row `dead_lettered`. JetStream has no
DLQ: after `max_deliver` the message is no longer delivered but stays in the stream (until stream
limits evict it). The consumer logs `event exhausted max deliveries` at error level with the event id
on the final nak. To find such messages, watch the JetStream max-deliveries advisory
(`$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.<stream>.<consumer>`) or inspect the stream
(`nats stream view <stream>`) for sequences the consumer has not acked. The relay never spends
attempts on transport failures: when the bus is unreachable it defers the row with growing backoff and
pauses, so an outage cannot dead-letter rows.

**Three dedup layers.**
1. *Broker.* JetStream drops a repeated `Nats-Msg-Id` inside `duplicate_window`, set explicitly to 1
   hour (longer than the relay's retry horizon) rather than the 2-minute server default.
2. *Relay lease.* A row claimed by a relay or DB consumer that then died is reclaimed once
   `claimed_at` is older than 600 s, so a crash never strands an event in `processing`.
3. *Processed-event ledger.* Before a handler runs, `(event_id, handler)` is claimed in
   `processed_events` (INSERT `in_progress`, or take over a claim older than the 1800 s lease).
   Success marks it `done`; failure releases it; both are conditioned on the claim's millisecond
   `started_at` value (`WHERE ... AND started_at = ?`), which acts as the owner token, so a stale
   worker cannot clobber a newer claim. A delivery that sees a fresh claim held elsewhere is
   deferred (`AckHandle::defer`: rescheduled to the lease expiry without consuming an attempt;
   JetStream `nak` with delay, which does count toward `max_deliver`). A `done` row skips the handler.
   Ledger rows are kept 30 days. A ledger read error skips the handler and nacks.

## Guarantees

- No two deliveries run the same handler for the same event concurrently.
- At-least-once after a crash: a lost claim expires and the event is retried.
- Handlers are idempotent, so the remaining duplicate window (crash after the effect, before the
  ledger mark) is harmless. The O5 audit made each handler safe to re-run: image cleanup (NotFound
  is Ok), image conversion (probes the exact key; missing original means done), follow backfill and
  inbound review import (upserts, deterministic Create ids), person/movie enrichment (upserts; the
  movie profile write is last because it is the "done" marker), wrap-up (a `Generating` record is
  resumed; completion is atomic so `WrapUpCompleted` is emitted once), search handlers (keyed
  upserts and removes), poster sync (skips when the poster is already stored under the movie id, so
  no second `PosterSynced`/`ImageStored`).

## Known gaps

- **Remote and block follow events are best-effort.** `CompositeSocialAdapter` routes remote
  follows and blocks through k-ap, which owns that transaction. Its events are published after the
  fact through an injected publisher, so a crash between the k-ap write and the publish loses them.
  Fixing it needs transaction hooks in k-ap.
- **k-ap 0.5.0 mints random ids for some activities.** Create, Announce and Like ids are
  deterministic (`broadcast.rs:22-35`, `:283-288`; backfill `backfill.rs:208-212`). Delete
  (`broadcast.rs:157`), Add/Undo (`:210`), Update (`:321`), Move (`:377`), the Undo wrappers for
  Announce (`:81`) and Like (`:129`) and the actor update (`:339-344`) use `Uuid::new_v4()` via `url_scheme.rs:52-53` with no override; only
  `broadcast_raw_to_followers` (`:395`) takes a caller id. A duplicate of those after a crash is a
  fresh-id activity. They are semantically idempotent for remote servers (deleting twice, updating
  to the same state), but it is a new activity, not a replay. The ledger makes this rare; closing it
  means building those activities ourselves or a k-ap change.
- **Core NATS has no safety net.** `NATS_MODE=core` is fire-and-forget: no broker dedup, no
  redelivery. Rows are marked done after publish; a subscriber that is down misses the event. The
  ledger still prevents double handling of a repeated id.
- **`ImageStored` stays best-effort.** It is raised after an object-storage write, which no
  database transaction can cover.

## Consequences

- Existing deployments keep their JetStream stream. `get_or_create_stream` applies config only on
  creation, so the consumer now compares an existing stream and consumer with what this build needs
  and updates `duplicate_window` (1 h) and the consumer's `max_deliver`/`ack_wait` in place, with a
  warning in the log. Both are mutable on a live stream/consumer, so no recreate is needed. Each update is a
  clone of the existing config with only those fields changed, so operator-set fields and the
  immutable ones (name, filter, deliver policy) are preserved. If an update is rejected the worker
  logs a warning with the manual `nats stream edit` / `nats consumer edit` command and keeps
  starting.
- Server and worker both migrate at startup (docker compose starts them together). `sqlite::migrate`
  retries with a bounded backoff (100 ms to 1 s, 5 retries), so the loser of a migration race
  re-reads `_sqlx_migrations`, skips what the winner applied and succeeds; only a persistent error
  is returned.
- Handler authors must keep new handlers idempotent; the ledger narrows duplicates but cannot
  remove the crash-after-effect case.
