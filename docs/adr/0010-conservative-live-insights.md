# Insights are live, conservative, and computed by a pure analyzer over denormalized rows

Insights (a new always-on surface, distinct from WrapUp and from plain
`UserTrends`) analyse a user's diary for behavioural patterns — rating lift by
genre, genre seasonality, medium timing, watch-time rhythm. Every insight
compares the user against their own baseline rather than reporting a ranking.

## Decision

- **Live, not precomputed.** Unlike WrapUp (async worker, stored
  `report_json`), Insights are computed on every request. The underlying query
  is one join over `reviews` + `movies` + genus tables, the same cost class as
  existing `StatsRepository` aggregates; a materialization layer is only earned
  if diaries grow provably too large for that.
- **Honesty by suppression, not disclaimers.** Insights are conservative: any
  candidate below sample-size minimums (≥3–5 ratings per genre for rating
  lift, ≥15–20 rows for seasonality, ≥5 rows per medium bucket) or strength
  thresholds (+0.3 stars, 1.5×) is silently not shown, and whole categories
  collapse when their inputs are unreliable (>~30% `NULL` `watch_medium`;
  >50% of rows at exactly `00:00`, which betrays imported CSVs that carry no
  real time). An empty-insights page is the correct rendering of a small or
  noisy diary — the UI shows an empty state, not filler. This is deliberate;
  do not "fix" it by lowering thresholds or adding hedges.
- **Fetch rows, then analyze in the domain.** Rejected the `StatsRepository`
  pattern (aggregation in adapter SQL) as the primary carrier of insight logic:
  thresholds and baseline comparisons would hide inside SQL, untestable and
  duplicated per database. Instead the port
  `InsightsQuery::get_watch_rows() -> Vec<WatchRow>` fetches one denormalized
  row per review (genres, medium, watched time), analogous to
  `WrapUpStatsQuery::get_reviews_with_profiles` but without WrapUp's report-era
  fields. A pure analyzer `insights_analyzer::build_insights(rows) -> Vec<Insight>`
  owns every threshold and comparison, unit-tested without a database. Rewatch
  detection also lives in the analyzer — grouping rows by movie and keeping the
  earliest, exactly like `wrapup_analyzer::compute_rewatch_stats` — so there is
  one rewatch definition per analyzer and the adapters stay shapeless
  "rows in, rows out". **Rating lift uses first-time entries only** (rewatches
  inflate genre ratings by choice, not by property), while seasonality and
  timing count every entry, rewatch or not.
- **Multi-genre rows are counted per genre, unnormalized.** A review of a
  movie with three genres contributes to all three genre distributions. Genre
  insights are frequency comparisons, not probabilities over reviews.
- **Private surface for now.** Insights render only on the user's own page
  (`/insights`, own data). They expose habits (late-night watching, mediums),
  which are more sensitive than the taste statistics already public on
  profiles; sharing is an opt-in feature to design later, not a default.

## Considered Options

- **SQL aggregation in adapters (stats pattern)** — rejected for the primary
  logic path: threshold rules would fragment into per-database SQL and resist
  unit testing; SQL still wins where it is just "rows in, rows out".
- **Insights as WrapUp sections** — rejected: WrapUp is a dated snapshot,
  insights decay/shift with every logged review and belong to a live surface;
  whether to feed selected insights into future WrapUps is a later product
  decision.
- **Rewatch detection in adapter SQL** — rejected: it would fork the rewatch
  definition already applied by `wrapup_analyzer::compute_rewatch_stats`; the
  analyzer groups by movie and keeps the earliest review, same as WrapUp.
- **Generic `Insight { title, description }` text payload** — rejected: typed
  serde-tagged variants (`RatingLift`, `Seasonality`, `MediumTiming`, `Rhythm`,
  each carrying `sample_count`) let the SPA render data-backed cards, show
  "why nothing here yet", and stay stable across copy changes.
