import { Link } from "@tanstack/react-router"
import { useCallback } from "react"
import { useTranslation } from "react-i18next"
import { Bar, BarChart, XAxis, YAxis } from "recharts"
import { Globe, Search, User } from "lucide-react"
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from "@/components/ui/chart"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { Input } from "@/components/ui/input"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeTabs } from "@/components/swipe-tabs"
import { VirtualList } from "@/components/virtual-list"
import { useInfiniteDiary } from "@/hooks/use-diary"
import { timeAgo } from "@/lib/date"
import type { UserProfileResponse } from "@/lib/api/users"

type ProfileViewProps = {
  data: UserProfileResponse
  actions?: React.ReactNode
  headerRight?: React.ReactNode
  userId?: string
  search?: string
  onSearchChange?: (value: string) => void
  isFederated?: boolean
  bio?: string
  handle?: string
  actorUrl?: string
}

export function ProfileView({
  data,
  actions,
  headerRight,
  userId,
  search,
  onSearchChange,
  isFederated,
  bio,
  handle,
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
          {avatar && <AvatarImage src={avatar} />}
          <AvatarFallback>{initial}</AvatarFallback>
        </Avatar>
        <div className="min-w-0 flex-1">
          <p className="font-semibold">{data.username}</p>
          {isFederated && handle && (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Globe className="size-3" />
              <span>{handle}</span>
            </div>
          )}
          {bio && <p className="mt-1 text-sm text-muted-foreground">{bio}</p>}
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

      {onSearchChange && (
        <div className="relative">
          <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder={t("profile.searchPlaceholder")}
            value={search ?? ""}
            onChange={(e) => onSearchChange(e.target.value)}
            className="pl-9"
          />
        </div>
      )}

      {actions}

      <SwipeTabs
        tabs={profileTabs}
        defaultValue="recent"
        tabsListClassName="w-full"
      >
        {(tab) => (
          <>
            {tab === "recent" && (
              <DiaryTab key="date_desc" sortBy="date_desc" userId={userId} search={search} />
            )}
            {tab === "top_rated" && (
              <DiaryTab
                key="rating_desc"
                sortBy="rating_desc"
                userId={userId}
                search={search}
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

function DiaryTab({ sortBy, search }: { sortBy: string; userId?: string; search?: string }) {
  const { t } = useTranslation()
  const { data, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } =
    useInfiniteDiary({ sort_by: sortBy, movie_id: undefined })
  const items = data?.pages.flatMap((p) => p.items) ?? []
  const filtered = search
    ? items.filter((e) =>
        e.movie.title.toLowerCase().includes(search.toLowerCase())
      )
    : items
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  if (isPending) return <Skeleton className="h-40 w-full rounded-xl" />
  if (!filtered.length) return <EmptyState icon={User} title={t("profile.noEntries")} />

  return (
    <VirtualList
      items={filtered}
      estimateSize={52}
      hasMore={!!hasNextPage}
      isFetching={isFetchingNextPage}
      onLoadMore={loadMore}
      renderItem={(e) => (
        <MovieCard
          movie={e.movie}
          rating={e.review.rating}
          comment={e.review.comment}
          subtitle={t("profile.watchedAgo", { when: timeAgo(e.review.watched_at) })}
          variant="compact"
        />
      )}
    />
  )
}

const trendChartConfig = {
  count: { label: "Movies", color: "var(--primary)" },
} satisfies ChartConfig

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
            <ChartContainer config={trendChartConfig} className="aspect-[2/1] w-full">
              <BarChart data={data.trends.monthly_ratings} margin={{ top: 8, right: 0, bottom: 0, left: -20 }}>
                <XAxis dataKey="month_label" tickFormatter={(v: string) => v.slice(0, 3)} tick={{ fontSize: 10, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} />
                <YAxis allowDecimals={false} tick={{ fontSize: 10, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} width={30} />
                <ChartTooltip content={<ChartTooltipContent />} />
                <Bar dataKey="count" fill="var(--color-count)" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ChartContainer>
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
