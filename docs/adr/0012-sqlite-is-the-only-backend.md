# SQLite is the only database backend

Movies Diary supports exactly one database: SQLite. The PostgreSQL adapters and the `DATABASE_BACKEND` runtime switch are removed.

## Context

The repository carried two adapter families, `sqlite*` and `postgres*`, across five crate pairs (base repositories, social, federation, search, event queue). They were roughly 80% copy-pasted, about 6,000 lines, differing mainly in placeholder syntax (`?` vs `$1`), datetime types and FTS (FTS5 vs tsvector). Every port change had to be made twice. The postgres family had no tests, so it was never verified to work, and the owner has only ever run SQLite. It was dead weight that doubled the maintenance cost of every storage change and the number of feature combinations to build.

## Decision

Delete the `postgres*` crates and the backend switch. SQLite is the sole implementation of the storage ports. If a second backend is ever needed, it is added back deliberately, with tests, behind the existing ports rather than kept alive speculatively.

## Consequences

- One adapter family to change per port; roughly 6,000 fewer lines and five fewer crates to build and review.
- `DATABASE_BACKEND` is ignored. If it is set to anything other than `sqlite`, startup logs one warning ("DATABASE_BACKEND is ignored; SQLite is the only backend") and continues.
- Cargo features collapse to `federation` (default) and `nats`. `sqlite` and `sqlite-federation` remain as deprecated aliases so existing deploy commands (`FEATURES=sqlite,sqlite-federation,nats`) still build. They may be removed in a future release once deploy scripts no longer pass them.
- Existing PostgreSQL deployments are not supported and have no migration path from this repository.
- ADR-0006, ADR-0008 and ADR-0009 mention postgres features and crates; they carry a note that they are superseded in part by this ADR and are otherwise left as written.
