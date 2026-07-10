import { createFileRoute, Link } from "@tanstack/react-router"
import { lazy, Suspense, useState } from "react"
import { useTranslation } from "react-i18next"
import { Bar, BarChart, XAxis, YAxis } from "recharts"
import { BarChart3, DollarSign, Globe, Hash, Share2, Star } from "lucide-react"
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from "@/components/ui/chart"
import { BackButton } from "@/components/back-button"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { RatingHistogram } from "@/components/rating-histogram"
import { RevealCard } from "@/components/reveal-card"
import { HeroCard } from "@/components/wrapup-hero"
import { FunFacts } from "@/components/wrapup-fun-facts"
import { RankCard } from "@/components/wrapup-rank-card"
import { posterUrl } from "@/lib/api/client"
import { fmtUsd } from "@/lib/format"
import { useWrapUpReport } from "@/features/wrapup"
import { useDocumentTitle } from "@/hooks/use-document-title"
import type { MovieRef } from "@/features/wrapup"

const WrapUpShareCard = lazy(() => import("@/components/wrapup-share-card").then((m) => ({ default: m.WrapUpShareCard })))

const monthlyChartConfig = {
  count: { label: "Movies", color: "var(--primary)" },
} satisfies ChartConfig

const genreChartConfig = {
  count: { label: "Movies", color: "var(--primary)" },
} satisfies ChartConfig

export const Route = createFileRoute("/_app/wrapup/$id")({
  component: WrapUpReportPage,
})

function WrapUpReportPage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { data: report, isPending } = useWrapUpReport(id)
  const [showShare, setShowShare] = useState(false)

  useDocumentTitle(report ? `${report.date_range.start.slice(0, 4)} Wrap-Up` : undefined)
  if (isPending) return <ReportSkeleton />
  if (!report) return null

  const watchHours = Math.round(report.total_watch_time_minutes / 60)

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <BackButton />
        <Button variant="ghost" size="icon" onClick={() => setShowShare(true)}>
          <Share2 className="size-5" />
        </Button>
      </div>

      {showShare && (
        <Suspense>
          <WrapUpShareCard report={report} onClose={() => setShowShare(false)} />
        </Suspense>
      )}

      <HeroCard report={report} watchHours={watchHours} />

      {/* Ratings */}
      <RevealCard>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Star className="size-4" /> {t("wrapup.ratings")}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {report.avg_rating != null && (
              <div className="text-center">
                <p className="text-4xl font-bold text-amber-500">{report.avg_rating.toFixed(1)}★</p>
                <p className="text-xs text-muted-foreground">{t("wrapup.averageRating")}</p>
              </div>
            )}
            <RatingHistogram histogram={report.rating_distribution} />
            <div className="flex flex-wrap gap-2">
              {report.busiest_month && (
                <Badge variant="secondary">{t("wrapup.busiestMonth", { month: report.busiest_month })}</Badge>
              )}
              {report.busiest_day_of_week && (
                <Badge variant="secondary">{t("wrapup.favoriteDay", { day: report.busiest_day_of_week })}</Badge>
              )}
            </div>
          </CardContent>
        </Card>
      </RevealCard>

      {/* Top Directors */}
      {report.top_directors.length > 0 && (
        <RevealCard>
          <RankCard
            title={t("wrapup.topDirectors")}
            subtitle={t("wrapup.uniqueDirectors", { count: report.director_diversity })}
            items={report.top_directors.slice(0, 5)}
          />
        </RevealCard>
      )}

      {/* Top Actors */}
      {report.top_actors.length > 0 && (
        <RevealCard>
          <RankCard
            title={t("wrapup.topActors")}
            subtitle={t("wrapup.uniqueActors", { count: report.actor_diversity })}
            items={report.top_actors.slice(0, 5)}
            profilePaths={report.top_cast_profile_paths}
          />
        </RevealCard>
      )}

      {/* Genres */}
      {report.top_genres.length > 0 && (
        <RevealCard>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">{t("wrapup.genres")}</CardTitle>
              <CardDescription>{t("wrapup.genresExplored", { count: report.genre_diversity })}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              <ChartContainer config={genreChartConfig} className="w-full" style={{ height: Math.min(report.top_genres.length, 8) * 28 + 16 }}>
                <BarChart data={report.top_genres.slice(0, 8)} layout="vertical" margin={{ top: 0, right: 4, bottom: 0, left: 0 }}>
                  <XAxis type="number" hide />
                  <YAxis type="category" dataKey="genre" tick={{ fontSize: 11, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} width={80} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Bar dataKey="count" fill="var(--color-count)" radius={[0, 4, 4, 0]} />
                </BarChart>
              </ChartContainer>
              <div className="flex flex-wrap gap-2 pt-2">
                {report.highest_rated_genre && (
                  <Badge variant="secondary">{t("wrapup.highestRated", { genre: report.highest_rated_genre })}</Badge>
                )}
                {report.lowest_rated_genre && (
                  <Badge variant="secondary">{t("wrapup.lowestRated", { genre: report.lowest_rated_genre })}</Badge>
                )}
              </div>
            </CardContent>
          </Card>
        </RevealCard>
      )}

      {/* Monthly Activity */}
      {report.movies_per_month.length > 0 && (
        <RevealCard>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <BarChart3 className="size-4" /> {t("wrapup.monthlyActivity")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={monthlyChartConfig} className="aspect-[2/1] w-full">
                <BarChart data={report.movies_per_month} margin={{ top: 8, right: 0, bottom: 0, left: -20 }}>
                  <XAxis dataKey="year_month" tickFormatter={(v: string) => v.slice(5)} tick={{ fontSize: 10, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} />
                  <YAxis allowDecimals={false} tick={{ fontSize: 10, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} width={30} />
                  <ChartTooltip content={<ChartTooltipContent labelFormatter={(v) => report.movies_per_month.find((m) => m.year_month === String(v))?.label ?? String(v)} />} />
                  <Bar dataKey="count" fill="var(--color-count)" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ChartContainer>
            </CardContent>
          </Card>
        </RevealCard>
      )}

      {/* Keywords */}
      {report.top_keywords.length > 0 && (
        <RevealCard>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <Hash className="size-4" /> {t("wrapup.keywords")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-1.5">
                {report.top_keywords
                  .filter((k) => !k.keyword.includes("creditsstinger"))
                  .slice(0, 15)
                  .map((k) => (
                    <Badge key={k.keyword} variant="secondary" className="text-xs">
                      {k.keyword} <span className="ml-1 opacity-60">{k.count}</span>
                    </Badge>
                  ))}
              </div>
            </CardContent>
          </Card>
        </RevealCard>
      )}

      {/* Budget & Language */}
      {(report.total_budget_watched != null || report.language_distribution.length > 1) && (() => {
        const hasBudget = report.total_budget_watched != null
        const hasLang = report.language_distribution.length > 1
        const bothVisible = hasBudget && hasLang
        return (
        <RevealCard>
          <div className={bothVisible ? "grid grid-cols-2 gap-3" : ""}>
            {hasBudget && (
              <Card>
                <CardContent className="py-4 text-center">
                  <DollarSign className="mx-auto mb-1 size-4 text-muted-foreground" />
                  <p className="text-lg font-bold">{fmtUsd(report.total_budget_watched!)}</p>
                  <p className="text-[10px] text-muted-foreground">{t("wrapup.totalBudget")}</p>
                  {report.avg_budget != null && (
                    <p className="mt-1 text-[10px] text-muted-foreground">
                      {t("wrapup.avgBudget", { amount: fmtUsd(report.avg_budget) })}
                    </p>
                  )}
                </CardContent>
              </Card>
            )}
            {hasLang && (
              <Card>
                <CardContent className="py-4 text-center">
                  <Globe className="mx-auto mb-1 size-4 text-muted-foreground" />
                  <p className="text-lg font-bold">{report.language_distribution.length}</p>
                  <p className="text-[10px] text-muted-foreground">{t("wrapup.languages")}</p>
                  <p className="mt-1 text-[10px] text-muted-foreground">
                    {report.language_distribution.slice(0, 3).map((l) => l.language.toUpperCase()).join(", ")}
                  </p>
                </CardContent>
              </Card>
            )}
          </div>
        </RevealCard>
        )})()}

      {/* Highlights */}
      <RevealCard>
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">{t("wrapup.highlights")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-3">
              <MovieHighlight label={t("wrapup.highlightHighest")} movie={report.highest_rated_movie} />
              <MovieHighlight label={t("wrapup.highlightLowest")} movie={report.lowest_rated_movie} />
              <MovieHighlight label={t("wrapup.highlightOldest")} movie={report.oldest_movie} />
              <MovieHighlight label={t("wrapup.highlightNewest")} movie={report.newest_movie} />
              <MovieHighlight label={t("wrapup.highlightLongest")} movie={report.longest_movie} showRuntime />
              <MovieHighlight label={t("wrapup.highlightShortest")} movie={report.shortest_movie} showRuntime />
              <MovieHighlight label={t("wrapup.highlightFirst")} movie={report.first_movie_of_period} />
              <MovieHighlight label={t("wrapup.highlightLast")} movie={report.last_movie_of_period} />
            </div>
          </CardContent>
        </Card>
      </RevealCard>

      <FunFacts report={report} watchHours={watchHours} />

      {/* Rewatches */}
      {report.total_rewatches > 0 && (
        <RevealCard>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">{t("wrapup.rewatches")}</CardTitle>
            </CardHeader>
            <CardContent className="text-center">
              <p className="text-3xl font-bold">{report.total_rewatches}</p>
              <p className="text-xs text-muted-foreground">{t("wrapup.moviesRewatched")}</p>
              {report.most_rewatched_movie && (
                <p className="mt-2 text-sm text-muted-foreground">
                  {t("wrapup.mostRewatched")} <strong>{report.most_rewatched_movie.title}</strong> ({report.most_rewatched_movie.year})
                </p>
              )}
            </CardContent>
          </Card>
        </RevealCard>
      )}

      {/* All Movies */}
      {report.poster_paths.length > 0 && (
        <RevealCard>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">{t("wrapup.allMovies", { count: report.poster_paths.length })}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-5 gap-1">
                {report.poster_paths.map((path, i) => (
                  <div key={i} className="aspect-[2/3] overflow-hidden rounded-md bg-muted">
                    <img src={posterUrl(path)} alt="" className="size-full object-cover" loading="lazy" />
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </RevealCard>
      )}
    </div>
  )
}

function MovieHighlight({ label, movie, showRuntime }: { label: string; movie?: MovieRef; showRuntime?: boolean }) {
  if (!movie) return null
  const content = (
    <div className="overflow-hidden rounded-xl bg-muted">
      {movie.poster_path && (
        <div className="aspect-[2/3] w-full">
          <img src={posterUrl(movie.poster_path)} alt={movie.title} className="size-full object-cover" />
        </div>
      )}
      <div className="p-2">
        <p className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</p>
        <p className="truncate text-xs font-medium">{movie.title}</p>
        <p className="text-[10px] text-muted-foreground">
          {showRuntime && movie.runtime_minutes ? `${movie.runtime_minutes} min` : movie.year}
        </p>
      </div>
    </div>
  )
  if (movie.movie_id) {
    return <Link to="/movies/$id" params={{ id: movie.movie_id }}>{content}</Link>
  }
  return content
}

function ReportSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <Skeleton className="h-4 w-16" />
      <Skeleton className="h-40 w-full rounded-xl" />
      <Skeleton className="h-60 w-full rounded-xl" />
      <Skeleton className="h-40 w-full rounded-xl" />
    </div>
  )
}
