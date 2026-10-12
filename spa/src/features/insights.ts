import { useQuery } from "@tanstack/react-query"
import { get } from "@/lib/api/client"

export type RatingLiftInsight = {
  kind: "rating_lift"
  genre: string
  genre_avg: number
  user_avg: number
  delta: number
  sample_count: number
}

export type SeasonalityInsight = {
  kind: "seasonality"
  genre: string
  month: number
  multiplier: number
  sample_count: number
}

export type MediumTimingInsight = {
  kind: "medium_timing"
  medium: string
  multiplier: number
  weekend_share: number
  baseline_share: number
  sample_count: number
}

export type RhythmInsight = {
  kind: "rhythm"
  peak_hour_start: number
  peak_hour_end: number
  top_days: number[]
  sample_count: number
}

export type Insight =
  | RatingLiftInsight
  | SeasonalityInsight
  | MediumTimingInsight
  | RhythmInsight

export type InsightsResponse = {
  insights: Insight[]
}

export const insightsKeys = {
  all: ["insights"] as const,
  forZone: (tz: string) => [...insightsKeys.all, tz] as const,
}

// Months, weekdays and hours are bucketed server-side in this zone, so the
// rhythm card's hours read as the viewer's local time.
function browserTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone
}

function getInsights(tz: string) {
  return get<InsightsResponse>("/insights", { tz })
}

export function useInsights() {
  const tz = browserTimeZone()
  return useQuery({
    queryKey: insightsKeys.forZone(tz),
    queryFn: () => getInsights(tz),
  })
}
