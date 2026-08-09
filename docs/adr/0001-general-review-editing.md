# General review editing with partial updates

Reviews were insert-only by design — one watch, one immutable record. Adding the WatchMedium field (optional metadata for how a movie was watched) required an update path for backfilling existing entries. Rather than adding a narrow "set medium" operation, we chose general partial editing of all mutable fields (rating, comment, watched_at, watch_medium). This mirrors the update capability that already exists for inbound federated reviews, and avoids accumulating single-field setters as new optional fields are added over time. Local edits broadcast an AP `Update` activity to stay consistent with federation.

## Considered Options

- **Narrow setter per field** — rejected because it would need to be repeated for every future optional field, and the infrastructure cost (use case, repo method, API endpoint, UI) is identical to general edit.
- **Delete and re-create** — rejected because it changes the review ID, breaks AP references, and is a worse UX for correcting a typo.
