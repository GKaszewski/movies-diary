# Unified social identity layer — wrap k_ap, don't gut it

Social interactions (follow, unfollow, block, etc.) bypassed the application layer entirely — handlers called the ActivityPub adapter (`k_ap`) directly, and there was no concept of a local-only follow. Every social operation was implicitly federated, with no domain-level orchestration, no CQRS split, and no domain events for most actions. This made it impossible to add local social features without duplicating logic, and meant the codebase would drift as federation and local paths diverged.

We introduce a `SocialIdentity` value object (`Local(UserId)` | `Remote { actor_url }`) in the domain layer. Social command and query ports (`SocialCommand` / `SocialQuery`) accept `SocialIdentity` instead of raw UUIDs or actor URLs. Application-layer use cases follow the existing CQRS pattern (command/query structs, separate deps, one file per use case, domain events on mutations). The adapter implementing `SocialCommand` branches on the identity variant: local goes straight to the database, remote delegates to `k_ap`. `k_ap` stays batteries-included and unchanged — this project just wraps it rather than reaching through it.

## Considered Options

- **Gut `k_ap` into a thin transport layer** — rejected because `k_ap` is shared with other projects (`thoughts`) that rely on its batteries-included API. Forcing all consumers to rewrite social orchestration defeats the purpose of the library.
- **Two-tier API in `k_ap`** (high-level + low-level primitives) — rejected because it adds complexity to `k_ap` for one consumer's needs. Wrapping at the adapter boundary in movies-diary is simpler and keeps `k_ap` focused.
- **Keep the status quo, add local branches in handlers** — rejected because it perpetuates the "no application layer for social" problem and guarantees local/remote drift.
