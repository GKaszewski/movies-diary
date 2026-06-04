import { createFileRoute } from "@tanstack/react-router"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { Clapperboard, Film, Inbox, Plus } from "lucide-react"
import { ReviewCard } from "@/components/review-card"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeTabs } from "@/components/swipe-tabs"
import { SwipeToDelete } from "@/components/swipe-to-delete"
import { VirtualList } from "@/components/virtual-list"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { StarRating } from "@/components/star-rating"
import { useAuth } from "@/components/auth-provider"
import { useInfiniteActivityFeed, useDeleteReview } from "@/hooks/use-diary"
import { SearchOverlay } from "@/components/search-overlay"
import type { MovieSelection } from "@/components/search-overlay"
import { useInfiniteWatchlist, useAddToWatchlist, useRemoveFromWatchlist } from "@/hooks/use-watchlist"
import { useWatchQueue, useConfirmWatch, useDismissWatch } from "@/hooks/use-webhooks"

export const Route = createFileRoute("/_app/")({
  component: HomePage,
})

function HomePage() {
  const { t } = useTranslation()
  const homeTabs = [
    { value: "feed", label: t("feed.tab") },
    { value: "watchlist", label: t("feed.watchlist") },
    { value: "queue", label: t("feed.queue") },
  ] as const

  return (
    <div className="p-4">
      <div className="mb-3 flex items-center justify-between">
        <h1 className="text-lg font-bold">{t("feed.title")}</h1>
      </div>
      <SwipeTabs tabs={homeTabs} defaultValue="feed" tabsListClassName="w-full">
        {(tab) => (
          <>
            {tab === "feed" && <FeedTab />}
            {tab === "watchlist" && <WatchlistTab />}
            {tab === "queue" && <QueueTab />}
          </>
        )}
      </SwipeTabs>
    </div>
  )
}

function FeedTab() {
  const { t } = useTranslation()
  const { auth } = useAuth()
  const [sortBy, setSortBy] = useState("date")
  const feedSortOptions = [
    { value: "date", label: t("feed.sortLatest") },
    { value: "date_asc", label: t("feed.sortOldest") },
    { value: "rating", label: t("feed.sortTopRated") },
    { value: "rating_asc", label: t("feed.sortLowestRated") },
  ] as const
  const { data, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } =
    useInfiniteActivityFeed({ sort_by: sortBy })
  const deleteReview = useDeleteReview()
  const items = data?.pages.flatMap((p) => p.items) ?? []
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  return (
    <div className="space-y-2">
      <div className="flex justify-end">
        <Select value={sortBy} onValueChange={setSortBy}>
          <SelectTrigger className="w-36">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {feedSortOptions.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {isPending && <FeedSkeleton />}

      {!isPending && !items.length && (
        <EmptyState icon={Film} title={t("feed.noActivity")} description={t("feed.noActivityDesc")} />
      )}

      {items.length > 0 && (
        <VirtualList
          items={items}
          estimateSize={120}
          hasMore={!!hasNextPage}
          isFetching={isFetchingNextPage}
          onLoadMore={loadMore}
          renderItem={(entry) => {
            const card = (
              <ReviewCard
                movie={entry.movie}
                review={entry.review}
                userName={entry.user_display_name}
                userId={entry.user_id}
              />
            )
            return entry.user_id === auth?.user_id ? (
              <SwipeToDelete
                onDelete={() => deleteReview.mutate(entry.review.id)}
                confirmTitle={t("feed.deleteReview")}
                confirmDescription={entry.movie.title}
              >
                {card}
              </SwipeToDelete>
            ) : (
              card
            )
          }}
        />
      )}
    </div>
  )
}

function WatchlistTab() {
  const { t } = useTranslation()
  const { data, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } =
    useInfiniteWatchlist()
  const items = data?.pages.flatMap((p) => p.items) ?? []
  const addMutation = useAddToWatchlist()
  const removeMutation = useRemoveFromWatchlist()
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])
  const [searchOpen, setSearchOpen] = useState(false)

  function handleAdd(movie: MovieSelection) {
    setSearchOpen(false)
    addMutation.mutate(
      movie.id
        ? { movie_id: movie.id }
        : {
            external_metadata_id: movie.external_metadata_id,
            manual_title: movie.title,
            manual_release_year: movie.release_year,
          },
    )
  }

  return (
    <div className="space-y-2">
      <Button variant="outline" size="sm" className="w-full" onClick={() => setSearchOpen(true)}>
        <Plus className="mr-1 size-4" />
        {t("feed.addToWatchlist")}
      </Button>

      {searchOpen && (
        <SearchOverlay open onClose={() => setSearchOpen(false)} onSelect={handleAdd} />
      )}

      {isPending && <FeedSkeleton />}

      {!isPending && !items.length && (
        <EmptyState icon={Clapperboard} title={t("feed.watchlistEmpty")} description={t("feed.watchlistEmptyDesc")} />
      )}

      {items.length > 0 && (
        <VirtualList
          items={items}
          estimateSize={110}
          hasMore={!!hasNextPage}
          isFetching={isFetchingNextPage}
          onLoadMore={loadMore}
          renderItem={(entry) => (
            <SwipeToDelete
              onDelete={() => removeMutation.mutate(entry.movie.id)}
              confirmTitle={t("feed.removeFromWatchlist")}
              confirmDescription={entry.movie.title}
            >
              <MovieCard movie={entry.movie} variant="full" />
            </SwipeToDelete>
          )}
        />
      )}
    </div>
  )
}

function QueueTab() {
  const { t } = useTranslation()
  const { data, isPending } = useWatchQueue()
  const confirmMutation = useConfirmWatch()
  const dismissMutation = useDismissWatch()
  const [ratings, setRatings] = useState<Record<string, number>>({})

  if (isPending) return <FeedSkeleton />
  if (!data?.length)
    return <EmptyState icon={Inbox} title={t("feed.queueEmpty")} description={t("feed.queueEmptyDesc")} />

  return (
    <div className="space-y-3">
      {data.map((entry) => (
        <div key={entry.id} className="rounded-xl bg-card p-3">
          <p className="font-semibold">{entry.title}</p>
          <p className="text-xs text-muted-foreground">
            {entry.year && `${entry.year} · `}{entry.source} · {entry.watched_at}
          </p>
          <div className="mt-2">
            <StarRating
              value={ratings[entry.id] ?? 0}
              onChange={(v) => setRatings((p) => ({ ...p, [entry.id]: v }))}
              size="sm"
            />
          </div>
          <div className="mt-2 flex gap-2">
            <Button
              size="sm"
              disabled={!ratings[entry.id]}
              onClick={() =>
                confirmMutation.mutate({
                  confirmations: [{ watch_event_id: entry.id, rating: ratings[entry.id]! }],
                })
              }
            >
              {t("common.confirm")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => dismissMutation.mutate({ event_ids: [entry.id] })}
            >
              {t("common.dismiss")}
            </Button>
          </div>
        </div>
      ))}
    </div>
  )
}

function FeedSkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3].map((i) => (
        <div key={i} className="flex gap-3 rounded-xl bg-card p-3">
          <Skeleton className="h-[84px] w-14 rounded-lg" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3 w-20" />
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-3 w-24" />
          </div>
        </div>
      ))}
    </div>
  )
}
