import { Link } from "@tanstack/react-router"
import { useCallback } from "react"
import { useTranslation } from "react-i18next"
import { User } from "lucide-react"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeTabs } from "@/components/swipe-tabs"
import { VirtualList } from "@/components/virtual-list"
import { posterUrl } from "@/lib/api/client"
import { useInfiniteDiary } from "@/hooks/use-diary"
import type { UserProfileResponse } from "@/lib/api/users"

type ProfileViewProps = {
  data: UserProfileResponse
  actions?: React.ReactNode
  headerRight?: React.ReactNode
  userId?: string
}

export function ProfileView({
  data,
  actions,
  headerRight,
  userId,
}: ProfileViewProps) {
  const { t } = useTranslation()
  const initial = (data.username || "?")[0]?.toUpperCase() ?? "?"
  const avatar = data.avatar_url

  const profileTabs = [
    { value: "recent", label: t("profile.recent") },
    { value: "top_rated", label: t("profile.topRated") },
    { value: "trends", label: t("profile.trends") },
  ] as const

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <Avatar size="lg">
          {avatar && <AvatarImage src={posterUrl(avatar)} />}
          <AvatarFallback>{initial}</AvatarFallback>
        </Avatar>
        <div className="flex-1">
          <p className="font-semibold">{data.username}</p>
        </div>
        {headerRight}
      </div>

      <div className="grid grid-cols-4 gap-2 text-center">
        <StatCell label={t("profile.movies")} value={data.stats.total_movies} />
        <StatCell
          label={t("profile.avg")}
          value={data.stats.avg_rating?.toFixed(1) ?? "-"}
        />
        <Link
          to="/social"
          search={userId ? { user: userId } : {}}
          className="block"
        >
          <StatCell label={t("profile.followingStat")} value={data.following_count} />
        </Link>
        <Link
          to="/social"
          search={userId ? { user: userId } : {}}
          className="block"
        >
          <StatCell label={t("profile.followers")} value={data.followers_count} />
        </Link>
      </div>

      {actions}

      <SwipeTabs
        tabs={profileTabs}
        defaultValue="recent"
        tabsListClassName="w-full"
      >
        {(tab) => (
          <>
            {tab === "recent" && (
              <DiaryTab key="date_desc" sortBy="date_desc" userId={userId} />
            )}
            {tab === "top_rated" && (
              <DiaryTab
                key="rating_desc"
                sortBy="rating_desc"
                userId={userId}
              />
            )}
            {tab === "trends" && <TrendsView data={data} />}
          </>
        )}
      </SwipeTabs>
    </div>
  )
}

function StatCell({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-xl bg-card py-2">
      <p className="text-sm font-bold">{value}</p>
      <p className="text-[10px] text-muted-foreground">{label}</p>
    </div>
  )
}

function DiaryTab({ sortBy }: { sortBy: string; userId?: string }) {
  const { t } = useTranslation()
  const { data, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } =
    useInfiniteDiary({ sort_by: sortBy, movie_id: undefined })
  const items = data?.pages.flatMap((p) => p.items) ?? []
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  if (isPending) return <Skeleton className="h-40 w-full rounded-xl" />
  if (!items.length) return <EmptyState icon={User} title={t("profile.noEntries")} />

  return (
    <VirtualList
      items={items}
      estimateSize={52}
      hasMore={!!hasNextPage}
      isFetching={isFetchingNextPage}
      onLoadMore={loadMore}
      renderItem={(e) => (
        <MovieCard
          movie={e.movie}
          rating={e.review.rating}
          comment={e.review.comment}
          subtitle={e.review.watched_at.slice(0, 10)}
          variant="compact"
        />
      )}
    />
  )
}

function TrendsView({
  data,
}: {
  data: {
    trends?: {
      top_directors: { director: string; count: number }[]
      monthly_ratings: {
        month_label: string
        avg_rating: number
        count: number
      }[]
    }
  }
}) {
  const { t } = useTranslation()

  if (!data.trends) return <EmptyState icon={User} title={t("profile.noTrends")} />

  return (
    <div className="space-y-3">
      {data.trends.top_directors.length > 0 && (
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("profile.topDirectors")}</CardTitle>
          </CardHeader>
          <CardContent>
            {data.trends.top_directors.map((d) => (
              <div
                key={d.director}
                className="flex items-center justify-between py-1 text-sm"
              >
                <span>{d.director}</span>
                <span className="text-xs text-muted-foreground">
                  {t("common.films", { count: d.count })}
                </span>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {data.trends.monthly_ratings.length > 0 && (
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("profile.monthlyActivity")}</CardTitle>
          </CardHeader>
          <CardContent>
            {data.trends.monthly_ratings.map((m) => (
              <div
                key={m.month_label}
                className="flex items-center justify-between py-1 text-sm"
              >
                <span>{m.month_label}</span>
                <span className="text-xs text-muted-foreground">
                  {t("common.filmsAvg", { count: m.count, avg: m.avg_rating.toFixed(1) })}
                </span>
              </div>
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  )
}

export function ProfileSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <Skeleton className="h-6 w-20" />
        <Skeleton className="size-5 rounded" />
      </div>
      <div className="flex items-center gap-4">
        <Skeleton className="size-14 rounded-full" />
        <Skeleton className="h-5 w-28" />
      </div>
      <div className="grid grid-cols-4 gap-2">
        {[1, 2, 3, 4].map((i) => (
          <Skeleton key={i} className="h-12 rounded-xl" />
        ))}
      </div>
      <Skeleton className="h-9 rounded-xl" />
      <div className="space-y-2">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-10 rounded-lg" />
        ))}
      </div>
    </div>
  )
}
