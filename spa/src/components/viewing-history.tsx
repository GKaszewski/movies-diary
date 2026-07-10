import { useTranslation } from "react-i18next"
import { TrendingUp } from "lucide-react"
import { StarDisplay } from "@/components/star-display"
import { shortDate } from "@/lib/date"
import type { ReviewHistoryResponse } from "@/features/movies"

export function ViewingHistory({ history }: { history: ReviewHistoryResponse }) {
  const { t } = useTranslation()

  if (history.viewings.length === 0) return null

  return (
    <section>
      <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.yourHistory")}</h3>
      <div className="space-y-2">
        {history.trend && (
          <div className="flex items-center gap-2 rounded-xl bg-card p-3 text-xs text-muted-foreground">
            <TrendingUp className="size-3.5" />
            {t("movie.trend", { trend: history.trend })}
          </div>
        )}
        {history.viewings.map((v) => (
          <div key={v.id} className="flex items-center justify-between rounded-xl bg-card p-3">
            <div>
              <p className="text-sm font-medium">{shortDate(v.watched_at)}</p>
              {v.comment && (
                <p className="mt-0.5 text-xs text-muted-foreground line-clamp-1">{v.comment}</p>
              )}
            </div>
            <StarDisplay rating={v.rating} size="xs" />
          </div>
        ))}
      </div>
    </section>
  )
}
