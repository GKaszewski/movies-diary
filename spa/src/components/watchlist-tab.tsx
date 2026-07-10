import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { Clapperboard, Plus } from "lucide-react"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeToDelete } from "@/components/swipe-to-delete"
import { VirtualList } from "@/components/virtual-list"
import { Button } from "@/components/ui/button"
import { SearchOverlay } from "@/components/search-overlay"
import type { MovieSelection } from "@/components/search-overlay"
import { useInfiniteWatchlist, useAddToWatchlist, useRemoveFromWatchlist } from "@/features/watchlist"
import { FeedSkeleton } from "@/components/feed-tab"

export function WatchlistTab() {
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
