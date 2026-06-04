import { createFileRoute, Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ArrowLeft, Bookmark, BookmarkCheck, Globe, Star, TrendingUp, User, Users } from "lucide-react"
import { StarDisplay } from "@/components/star-display"
import { RatingHistogram } from "@/components/rating-histogram"
import { EmptyState } from "@/components/empty-state"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { posterUrl, tmdbProfileUrl } from "@/lib/api/client"
import { useMovie, useMovieHistory, useMovieProfile } from "@/hooks/use-movies"
import {
  useWatchlistStatus,
  useAddToWatchlist,
  useRemoveFromWatchlist,
} from "@/hooks/use-watchlist"
import type { CastMemberDto, CrewMemberDto } from "@/lib/api/movies"

export const Route = createFileRoute("/_app/movies/$id")({
  component: MovieDetailPage,
})

function MovieDetailPage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { data, isPending } = useMovie(id)
  const { data: profile } = useMovieProfile(id)
  const { data: history } = useMovieHistory(id)

  if (isPending) return <DetailSkeleton />
  if (!data) return null

  const { movie, stats, reviews } = data
  const hasStats = profile && (profile.budget_usd != null || profile.revenue_usd != null || profile.vote_average != null)

  return (
    <div className="space-y-5 p-4">
      <Link to="/" className="inline-flex items-center gap-1 text-sm text-muted-foreground">
        <ArrowLeft className="size-4" /> {t("common.back")}
      </Link>

      <HeroSection movie={movie} stats={stats} movieId={id} tagline={profile?.tagline} />

      {(profile?.overview ?? movie.overview) && (
        <p className="text-sm leading-relaxed text-muted-foreground">{profile?.overview ?? movie.overview}</p>
      )}

      {hasStats && (
        <div className="flex gap-2">
          {profile.budget_usd != null && (
            <div className="flex-1 rounded-xl bg-card p-2.5 text-center">
              <p className="text-sm font-semibold">${(profile.budget_usd / 1e6).toFixed(0)}M</p>
              <p className="text-[10px] text-muted-foreground">{t("movie.budget")}</p>
            </div>
          )}
          {profile.revenue_usd != null && (
            <div className="flex-1 rounded-xl bg-card p-2.5 text-center">
              <p className="text-sm font-semibold">${(profile.revenue_usd / 1e6).toFixed(0)}M</p>
              <p className="text-[10px] text-muted-foreground">{t("movie.revenue")}</p>
            </div>
          )}
          {profile.vote_average != null && (
            <div className="flex-1 rounded-xl bg-card p-2.5 text-center">
              <p className="text-sm font-semibold">{profile.vote_average.toFixed(1)}</p>
              <p className="text-[10px] text-muted-foreground">{t("movie.tmdb")}</p>
            </div>
          )}
        </div>
      )}

      {stats.rating_histogram.length > 0 && (
        <div className="rounded-xl bg-card p-3">
          <p className="mb-2 text-xs font-medium text-muted-foreground">{t("movie.ratingDistribution")}</p>
          <RatingHistogram histogram={stats.rating_histogram} />
        </div>
      )}

      {profile && profile.cast.length > 0 && (
        <section className="overflow-hidden">
          <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.cast")}</h3>
          <PersonStrip items={profile.cast} type="cast" />
        </section>
      )}

      {profile && profile.crew.length > 0 && (
        <section className="overflow-hidden">
          <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.crew")}</h3>
          <PersonStrip items={profile.crew} type="crew" />
        </section>
      )}

      {profile && profile.keywords.length > 0 && (
        <section>
          <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.keywords")}</h3>
          <div className="flex flex-wrap gap-1.5">
            {profile.keywords.map((k) => (
              <Badge key={k.tmdb_id} variant="outline">{k.name}</Badge>
            ))}
          </div>
        </section>
      )}

      <section>
        <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.community")}</h3>
        {!reviews.items.length ? (
          <EmptyState icon={Users} title={t("movie.noReviews")} description={t("movie.beFirst")} />
        ) : (
          <div className="space-y-2">
            {reviews.items.map((r, i) => (
              <Card key={i} size="sm">
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle className="flex items-center gap-1.5 text-sm">
                        {r.user_display}
                        {r.is_federated && <Globe className="size-3 text-muted-foreground/60" />}
                      </CardTitle>
                      <CardDescription className="text-[10px]">{r.watched_at.slice(0, 10)}</CardDescription>
                    </div>
                    <StarDisplay rating={r.rating} size="xs" />
                  </div>
                </CardHeader>
                {r.comment && (
                  <CardContent>
                    <p className="text-xs text-muted-foreground">{r.comment}</p>
                  </CardContent>
                )}
              </Card>
            ))}
          </div>
        )}
      </section>

      {history && history.viewings.length > 0 && (
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
                  <p className="text-sm font-medium">{v.watched_at}</p>
                  {v.comment && (
                    <p className="mt-0.5 text-xs text-muted-foreground line-clamp-1">{v.comment}</p>
                  )}
                </div>
                <StarDisplay rating={v.rating} size="xs" />
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}

function HeroSection({
  movie,
  stats,
  movieId,
  tagline,
}: {
  movie: { title: string; release_year: number; director?: string; poster_path?: string; genres: string[]; runtime_minutes?: number }
  stats: { total_count: number; avg_rating?: number; federated_count: number }
  movieId: string
  tagline?: string
}) {
  const { t } = useTranslation()
  const { data: watchlistData } = useWatchlistStatus(movieId)
  const addWatchlist = useAddToWatchlist()
  const removeWatchlist = useRemoveFromWatchlist()
  const onWatchlist = watchlistData?.on_watchlist ?? false

  return (
    <div className="flex gap-4">
      <div className="h-[150px] w-[100px] flex-shrink-0 overflow-hidden rounded-xl bg-muted">
        {movie.poster_path && (
          <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />
        )}
      </div>
      <div className="min-w-0 flex-1 space-y-2">
        <h1 className="text-xl font-bold leading-tight">{movie.title}</h1>
        <p className="text-sm text-muted-foreground">
          {movie.release_year}
          {movie.director && ` · ${movie.director}`}
          {movie.runtime_minutes && ` · ${movie.runtime_minutes}m`}
        </p>
        {tagline && <p className="text-xs italic text-muted-foreground">{tagline}</p>}
        {movie.genres.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {movie.genres.map((g) => (
              <Badge key={g} variant="secondary" className="text-[10px]">{g}</Badge>
            ))}
          </div>
        )}
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          {stats.avg_rating != null && (
            <span className="flex items-center gap-1">
              <Star className="size-3 fill-amber-500 text-amber-500" />
              {stats.avg_rating.toFixed(1)}
            </span>
          )}
          <span>{t("common.reviews", { count: stats.total_count })}</span>
        </div>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant={onWatchlist ? "secondary" : "outline"}
            onClick={() =>
              onWatchlist
                ? removeWatchlist.mutate(movieId)
                : addWatchlist.mutate({ movie_id: movieId })
            }
          >
            {onWatchlist ? <BookmarkCheck className="mr-1 size-3.5" /> : <Bookmark className="mr-1 size-3.5" />}
            {onWatchlist ? t("movie.saved") : t("movie.watchlist")}
          </Button>
        </div>
      </div>
    </div>
  )
}

function PersonStrip({ items, type }: { items: (CastMemberDto | CrewMemberDto)[]; type: "cast" | "crew" }) {
  return (
    <div className="-mx-4 flex gap-2.5 overflow-x-auto overscroll-x-contain px-4 pb-2" style={{ scrollbarWidth: "thin", scrollbarColor: "rgba(255,255,255,0.15) transparent" }}>
      {items.map((person, i) => {
        const subtitle = type === "cast"
          ? (person as CastMemberDto).character
          : (person as CrewMemberDto).job

        return (
          <Link key={`${person.tmdb_person_id}-${i}`} to="/people/$id" params={{ id: person.person_id }} className="w-[72px] flex-shrink-0">
            <div className="aspect-[2/3] overflow-hidden rounded-lg bg-muted">
              {person.profile_path ? (
                <img src={tmdbProfileUrl(person.profile_path)} alt="" className="size-full object-cover" loading="lazy" />
              ) : (
                <div className="flex size-full items-center justify-center">
                  <User className="size-5 text-muted-foreground/40" />
                </div>
              )}
            </div>
            <p className="mt-1 truncate text-[11px] font-semibold leading-tight">{person.name}</p>
            <p className="truncate text-[10px] italic text-muted-foreground">{subtitle}</p>
          </Link>
        )
      })}
    </div>
  )
}

function DetailSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <Skeleton className="h-5 w-16" />
      <div className="flex gap-4">
        <Skeleton className="h-[150px] w-[100px] rounded-xl" />
        <div className="flex-1 space-y-2">
          <Skeleton className="h-6 w-40" />
          <Skeleton className="h-4 w-28" />
          <Skeleton className="h-4 w-20" />
        </div>
      </div>
      <Skeleton className="h-12 w-full rounded-xl" />
      <Skeleton className="h-24 w-full rounded-xl" />
      <div className="space-y-2">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-16 rounded-xl" />
        ))}
      </div>
    </div>
  )
}
