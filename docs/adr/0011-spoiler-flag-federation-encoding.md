# Spoiler Flag federates as both a boolean field and a summary content warning

Reviews marked with the Spoiler Flag serialize on the ActivityPub Note as `containsSpoilers: true` alongside `summary: "Contains spoilers"`. The boolean is the source of truth for Movies-Diary peers; the summary exists so Mastodon-class clients — which render any Note summary as a collapsed content warning — also protect their users instead of showing the spoiler raw. Inbound parsing trusts the boolean only (review Notes always originate from Movies-Diary instances, gated by the `review: true` discriminator), so unrelated summaries from other apps are never misread as spoiler flags.

## Considered Options

- **Boolean field only** — consistent with `watchMedium`, but Mastodon followers saw the full spoiler text raw; the feature silently failed for half the audience.
- **Summary-only** (presence = flag) — rejected because "any summary means spoiler" breaks if reviews ever gain titles or other apps content-warn for other reasons.
- **Emitting neither and relying on veiling at the source** — rejected; the flag must travel with the object or remote instances cannot protect their own viewers.
