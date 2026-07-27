# Local follows bypass ActivityPub, share storage

ADR-0002 introduced `SocialIdentity` so the domain never branches on local vs remote, but the adapter (`CompositeSocialAdapter`) still routed everything through k_ap — meaning a local user following another local user triggered WebFinger resolution, HTTP signature verification, and inbox delivery to the same instance. Wasteful on any hardware, unacceptable on an N100.

## Decision

**Commands branch in the adapter, queries don't.**

- `SocialCommand` methods in `CompositeSocialAdapter` check the `SocialIdentity` variant. Local targets get direct SQL writes to the `ap_followers`/`ap_following` tables (via a domain `FollowRepository` port). Remote targets delegate to `k_ap::ActivityPubService` as before.
- `SocialQuery` methods go through the domain port only — a single SQL query that left-joins `ap_followers`/`ap_following` against both `users` (local) and `ap_remote_actors` (remote), returning `SocialActor` directly.
- Local follows store the full actor URL (`https://instance.example/users/{uuid}`) in `remote_actor_url`, same format as remote follows. k_ap's AP collection endpoints read from these tables unchanged, so local relationships are visible to the fediverse automatically.
- Local user metadata (display name, avatar) is resolved from the `users` table at query time — no duplication into `ap_remote_actors`.
- Follow acceptance is required for both local and remote — no behavioral divergence.
- Local follow events (`FollowRequested`, `FollowAccepted`) are not broadcast as AP activities. The fediverse discovers local relationships passively via collection endpoints.

## Considered Options

- **Keep routing local through k_ap** — rejected because it wastes CPU/network on self-delivery and creates an unnecessary runtime dependency on federation for local social features.
- **Separate `local_follows` table** — rejected because it creates two sources of truth for the same concept and requires merging in collection endpoints.
- **Insert local users into `ap_remote_actors`** — rejected because it duplicates profile data and requires sync when local users update their profile.
