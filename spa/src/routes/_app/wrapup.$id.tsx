import { createFileRoute, Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { Star, Users } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { RatingHistogram } from "@/components/rating-histogram"
import { posterUrl, tmdbProfileUrl } from "@/lib/api/client"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { useWrapUpReport } from "@/hooks/use-wrapup"
import type { MovieRef, PersonStat } from "@/lib/api/wrapup"

export const Route = createFileRoute("/_app/wrapup/$id")({
  component: WrapUpReportPage,
})

function WrapUpReportPage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { data: report, isPending } = useWrapUpReport(id)

  if (isPending) return <ReportSkeleton />
  if (!report) return null

  const watchHours = Math.round(report.total_watch_time_minutes / 60)

  return (
    <div className="space-y-4 p-4">
      <BackButton />

      {/* Hero */}
      <Card>
        <CardContent className="py-8 text-center">
          <p className="text-xs uppercase tracking-widest text-muted-foreground">{t("wrapup.heroSubtitle")}</p>
          <p className="mt-2 text-5xl font-extrabold tracking-tight">{report.total_movies}</p>
          <p className="text-sm text-muted-foreground">{t("wrapup.moviesWatched")}</p>
          {watchHours > 0 && (
            <p className="mt-1 text-xs text-muted-foreground">{t("wrapup.watchHours", { hours: watchHours })}</p>
          )}
        </CardContent>
      </Card>

      {/* Ratings */}
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

      {/* Top Directors */}
      {report.top_directors.length > 0 && (
        <RankCard
          title={t("wrapup.topDirectors")}
          subtitle={t("wrapup.uniqueDirectors", { count: report.director_diversity })}
          items={report.top_directors.slice(0, 5)}
        />
      )}

      {/* Top Actors */}
      {report.top_actors.length > 0 && (
        <RankCard
          title={t("wrapup.topActors")}
          subtitle={t("wrapup.uniqueActors", { count: report.actor_diversity })}
          items={report.top_actors.slice(0, 5)}
          profilePaths={report.top_cast_profile_paths}
        />
      )}

      {/* Genres */}
      {report.top_genres.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">{t("wrapup.genres")}</CardTitle>
            <CardDescription>{t("wrapup.genresExplored", { count: report.genre_diversity })}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {report.top_genres.slice(0, 8).map((g) => {
              const max = report.top_genres[0]?.count ?? 1
              return (
                <div key={g.genre} className="flex items-center gap-2 text-sm">
                  <span className="w-20 truncate">{g.genre}</span>
                  <div className="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                    <div className="h-full rounded-full bg-primary" style={{ width: `${(g.count / max) * 100}%` }} />
                  </div>
                  <span className="w-6 text-right text-xs text-muted-foreground">{g.count}</span>
                </div>
              )
            })}
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
      )}

      {/* Highlights */}
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

      {/* Rewatches */}
      {report.total_rewatches > 0 && (
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
      )}

      {/* Poster Mosaic */}
      {report.poster_paths.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">{t("wrapup.posterMosaic")}</CardTitle>
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
      )}
    </div>
  )
}

function RankCard({ title, subtitle, items, profilePaths }: { title: string; subtitle: string; items: PersonStat[]; profilePaths?: string[] }) {
  const { t } = useTranslation()
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <Users className="size-4" /> {title}
        </CardTitle>
        <CardDescription>{subtitle}</CardDescription>
      </CardHeader>
      <CardContent>
        <ol className="space-y-2">
          {items.map((item, i) => {
            const profilePath = profilePaths?.[i]
            return (
              <li key={item.name}>
                {item.person_id ? (
                  <Link to="/people/$id" params={{ id: item.person_id }} className="flex items-center gap-3">
                    <span className="flex size-6 items-center justify-center rounded-full bg-muted text-xs font-bold">{i + 1}</span>
                    <Avatar className="size-8">
                      {profilePath && <AvatarImage src={tmdbProfileUrl(profilePath)} />}
                      <AvatarFallback className="text-xs">{item.name[0]}</AvatarFallback>
                    </Avatar>
                    <div className="flex-1">
                      <p className="text-sm font-medium">{item.name}</p>
                      <p className="text-xs text-muted-foreground">{t("common.filmsAvg", { count: item.count, avg: item.avg_rating.toFixed(1) })}★</p>
                    </div>
                  </Link>
                ) : (
                  <div className="flex items-center gap-3">
                    <span className="flex size-6 items-center justify-center rounded-full bg-muted text-xs font-bold">{i + 1}</span>
                    <Avatar className="size-8">
                      {profilePath && <AvatarImage src={tmdbProfileUrl(profilePath)} />}
                      <AvatarFallback className="text-xs">{item.name[0]}</AvatarFallback>
                    </Avatar>
                    <div className="flex-1">
                      <p className="text-sm font-medium">{item.name}</p>
                      <p className="text-xs text-muted-foreground">{t("common.filmsAvg", { count: item.count, avg: item.avg_rating.toFixed(1) })}★</p>
                    </div>
                  </div>
                )}
              </li>
            )
          })}
        </ol>
      </CardContent>
    </Card>
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
