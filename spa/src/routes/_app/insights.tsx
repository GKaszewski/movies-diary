import { createFileRoute } from "@tanstack/react-router"
import type { ReactNode } from "react"
import { Trans, useTranslation } from "react-i18next"
import {
  AlertTriangle,
  CalendarDays,
  Clapperboard,
  Clock,
  Lightbulb,
  Sparkles,
  TrendingDown,
  TrendingUp,
} from "lucide-react"
import { BackButton } from "@/components/back-button"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { EmptyState } from "@/components/empty-state"
import { Skeleton } from "@/components/ui/skeleton"
import { WATCH_MEDIUMS } from "@/lib/watch-mediums"
import { useInsights, type Insight } from "@/features/insights"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/insights")({
  component: InsightsPage,
})

const MONTH_BASE = new Date(2024, 0, 1) // any year; only the month name matters
const MONDAY_BASE = new Date(2024, 2, 4) // 2024-03-04 is a Monday

const hl = <span className="text-primary" />

function isKind<K extends Insight["kind"]>(kind: K) {
  return (insight: Insight): insight is Extract<Insight, { kind: K }> => insight.kind === kind
}

function monthName(month: number): string {
  return new Intl.DateTimeFormat(undefined, { month: "long" }).format(
    new Date(MONTH_BASE.getFullYear(), month - 1, 1),
  )
}

function dayNames(days: number[]): string[] {
  const fmt = new Intl.DateTimeFormat(undefined, { weekday: "short" })
  return days.map((d) => fmt.format(new Date(MONDAY_BASE.getFullYear(), MONDAY_BASE.getMonth(), MONDAY_BASE.getDate() + d - 1)))
}

function mediumLabel(medium: string, t: (key: string) => string): string {
  const def = WATCH_MEDIUMS.find((m) => m.value === medium)
  return def ? t(def.labelKey) : medium
}

function hourLabel(hour: number): string {
  return `${String(hour).padStart(2, "0")}:00`
}

function InsightsPage() {
  const { t } = useTranslation()
  const { data, isPending, isError, refetch } = useInsights()

  useDocumentTitle(t("insights.title"))
  if (isPending) return <InsightsSkeleton />

  const insights = data?.insights ?? []

  return (
    <div className="space-y-4 p-4">
      <BackButton />
      <div>
        <h1 className="flex items-center gap-2 text-lg font-bold">
          <Sparkles className="size-5 text-primary" />
          {t("insights.title")}
        </h1>
        <p className="text-sm text-muted-foreground">{t("insights.subtitle")}</p>
      </div>

      {isError ? (
        <div className="flex flex-col items-center">
          <EmptyState icon={AlertTriangle} title={t("errors.somethingWrong")} />
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            {t("common.tryAgain")}
          </Button>
        </div>
      ) : insights.length === 0 ? (
        <EmptyState
          icon={Lightbulb}
          title={t("insights.emptyTitle")}
          description={t("insights.emptyDesc")}
        />
      ) : (
        <>
          <InsightGroup
            title={t("insights.taste")}
            items={insights.filter(isKind("rating_lift"))}
            render={(insight) => <RatingLiftCard insight={insight} />}
          />
          <InsightGroup
            title={t("insights.seasons")}
            items={insights.filter(isKind("seasonality"))}
            render={(insight) => <SeasonalityCard insight={insight} />}
          />
          <InsightGroup
            title={t("insights.mediums")}
            items={insights.filter(isKind("medium_timing"))}
            render={(insight) => <MediumTimingCard insight={insight} />}
          />
          <InsightGroup
            title={t("insights.rhythm")}
            items={insights.filter(isKind("rhythm"))}
            render={(insight) => <RhythmCard insight={insight} />}
          />
        </>
      )}
    </div>
  )
}

function InsightGroup<T extends Insight>({
  title,
  items,
  render,
}: {
  title: string
  items: T[]
  render: (insight: T) => ReactNode
}) {
  if (items.length === 0) return null
  return (
    <section className="space-y-2">
      <h2 className="text-sm font-semibold text-muted-foreground">{title}</h2>
      {items.map((insight, i) => (
        <div key={i}>{render(insight)}</div>
      ))}
    </section>
  )
}

type RatingLiftCardProps = { insight: Extract<Insight, { kind: "rating_lift" }> }
type SeasonalityCardProps = { insight: Extract<Insight, { kind: "seasonality" }> }
type MediumTimingCardProps = { insight: Extract<Insight, { kind: "medium_timing" }> }
type RhythmCardProps = { insight: Extract<Insight, { kind: "rhythm" }> }

function RatingLiftCard({ insight }: RatingLiftCardProps) {
  const { t } = useTranslation()
  const up = insight.delta > 0
  const Trend = up ? TrendingUp : TrendingDown
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Trend className={`size-4 ${up ? "text-emerald-500" : "text-rose-500"}`} />
          <span>
            <Trans
              i18nKey={up ? "insights.ratingLiftUp" : "insights.ratingLiftDown"}
              values={{
                genre: insight.genre,
                genreAvg: insight.genre_avg.toFixed(1),
                userAvg: insight.user_avg.toFixed(1),
              }}
              components={{ hl }}
            />
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">
            {insight.genre_avg.toFixed(1)}★ · {insight.genre}
          </Badge>
          <Badge variant="outline">
            {t("insights.yourAverage")}: {insight.user_avg.toFixed(1)}★
          </Badge>
          <SampleBadge unit="films" count={insight.sample_count} />
        </div>
      </CardContent>
    </Card>
  )
}

function SeasonalityCard({ insight }: SeasonalityCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <CalendarDays className="size-4 text-primary" />
          <span>
            <Trans
              i18nKey="insights.seasonality"
              values={{
                genre: insight.genre,
                multiplier: insight.multiplier.toFixed(1),
                month: monthName(insight.month),
              }}
              components={{ hl }}
            />
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">
            {insight.multiplier.toFixed(1)}× · {monthName(insight.month)}
          </Badge>
          <Badge variant="outline">{insight.genre}</Badge>
          <SampleBadge unit="watches" count={insight.sample_count} />
        </div>
      </CardContent>
    </Card>
  )
}

function MediumTimingCard({ insight }: MediumTimingCardProps) {
  const { t } = useTranslation()
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Clapperboard className="size-4 text-primary" />
          <span>
            <Trans
              i18nKey="insights.mediumTiming"
              values={{
                medium: mediumLabel(insight.medium, t),
                multiplier: insight.multiplier.toFixed(1),
              }}
              components={{ hl }}
            />
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">
            {Math.round(insight.weekend_share * 100)}% · {t("insights.weekends")}
          </Badge>
          <Badge variant="outline">
            {t("insights.yourBaseline")}: {Math.round(insight.baseline_share * 100)}%
          </Badge>
          <SampleBadge unit="watches" count={insight.sample_count} />
        </div>
      </CardContent>
    </Card>
  )
}

function RhythmCard({ insight }: RhythmCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Clock className="size-4 text-primary" />
          <span>
            <Trans
              i18nKey="insights.rhythmBody"
              values={{
                start: hourLabel(insight.peak_hour_start),
                end: hourLabel(insight.peak_hour_end),
                days: dayNames(insight.top_days).join(", "),
              }}
              components={{ hl }}
            />
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">
            {hourLabel(insight.peak_hour_start)}–{hourLabel(insight.peak_hour_end)}
          </Badge>
          <Badge variant="outline">{dayNames(insight.top_days).join(", ")}</Badge>
          <SampleBadge unit="watches" count={insight.sample_count} />
        </div>
      </CardContent>
    </Card>
  )
}

// Rating lift counts distinct films (first watches only); every other
// category counts diary entries, rewatches included.
function SampleBadge({ unit, count }: { unit: "films" | "watches"; count: number }) {
  const { t } = useTranslation()
  const key = unit === "films" ? "insights.filmsSample" : "insights.watchesSample"
  return <Badge variant="outline">{t(key, { count })}</Badge>
}

function InsightsSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <BackButton />
      <Skeleton className="h-6 w-40" />
      <Skeleton className="h-4 w-64" />
      <div className="space-y-2">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-28 w-full" />
        ))}
      </div>
    </div>
  )
}
