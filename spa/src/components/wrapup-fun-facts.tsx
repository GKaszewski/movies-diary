import { useTranslation } from "react-i18next"
import { Lightbulb } from "lucide-react"
import { fmtUsd } from "@/lib/format"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { RevealCard } from "@/components/reveal-card"
import type { WrapUpReport } from "@/lib/api/wrapup"

export function FunFacts({ report, watchHours }: { report: WrapUpReport; watchHours: number }) {
  const { t } = useTranslation()
  const facts: string[] = []

  const oldest = report.oldest_movie?.year
  const newest = report.newest_movie?.year
  if (oldest && newest && newest - oldest > 0) {
    facts.push(t("wrapup.funSpan", { span: newest - oldest, oldest, newest }))
  }

  if (watchHours >= 24) {
    const days = (watchHours / 24).toFixed(1)
    facts.push(t("wrapup.funDays", { days }))
  } else if (watchHours > 0) {
    facts.push(t("wrapup.funHours", { hours: watchHours }))
  }

  if (report.total_budget_watched && report.total_budget_watched >= 1_000_000) {
    facts.push(t("wrapup.funBudget", { amount: fmtUsd(report.total_budget_watched) }))
  }

  if (report.genre_diversity > 5) {
    facts.push(t("wrapup.funGenres", { count: report.genre_diversity }))
  }

  if (report.actor_diversity > 10) {
    facts.push(t("wrapup.funActors", { count: report.actor_diversity }))
  }

  if (!facts.length) return null

  return (
    <RevealCard>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Lightbulb className="size-4" /> {t("wrapup.funFacts")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2">
            {facts.map((fact, i) => (
              <li key={i} className="text-sm text-muted-foreground">✦ {fact}</li>
            ))}
          </ul>
        </CardContent>
      </Card>
    </RevealCard>
  )
}
