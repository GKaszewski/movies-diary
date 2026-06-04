import { useTranslation } from "react-i18next"
import { Card, CardContent } from "@/components/ui/card"
import { RevealCard } from "@/components/reveal-card"
import { useCountUp } from "@/hooks/use-animate"
import type { WrapUpReport } from "@/lib/api/wrapup"

export function HeroCard({ report, watchHours }: { report: WrapUpReport; watchHours: number }) {
  const { t } = useTranslation()
  const movies = useCountUp(report.total_movies)
  const hours = useCountUp(watchHours)

  return (
    <RevealCard>
      <Card>
        <CardContent className="py-8 text-center" ref={movies.ref}>
          <p className="text-xs uppercase tracking-widest text-muted-foreground">{t("wrapup.heroSubtitle")}</p>
          <p className="mt-2 text-5xl font-extrabold tracking-tight">{movies.value}</p>
          <p className="text-sm text-muted-foreground">{t("wrapup.moviesWatched")}</p>
          {watchHours > 0 && (
            <p className="mt-1 text-xs text-muted-foreground" ref={hours.ref}>
              {t("wrapup.watchHours", { hours: hours.value })}
            </p>
          )}
        </CardContent>
      </Card>
    </RevealCard>
  )
}
