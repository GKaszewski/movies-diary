import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { Film, RefreshCw } from "lucide-react"
import { ReviewCard } from "@/components/review-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeToDelete } from "@/components/swipe-to-delete"
import { VirtualList } from "@/components/virtual-list"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { useAuth } from "@/components/auth-provider"
import { useQueryClient } from "@tanstack/react-query"
import { ReviewSheet } from "@/components/review-sheet"
import { ReviewDetailSheet } from "@/components/review-detail-sheet"
import { useInfiniteActivityFeed, useDeleteReview } from "@/features/diary"
import type { FeedEntryDto } from "@/features/diary"

export function FeedSkeleton() {
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

export function FeedTab() {
  const { t } = useTranslation()
  const { auth } = useAuth()
  const qc = useQueryClient()
  const [refreshing, setRefreshing] = useState(false)
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
  const [editingEntry, setEditingEntry] = useState<FeedEntryDto | null>(null)
  const [detailEntry, setDetailEntry] = useState<FeedEntryDto | null>(null)
  const items = data?.pages.flatMap((p) => p.items) ?? []
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={async () => {
            setRefreshing(true)
            await qc.refetchQueries({ queryKey: ["activity-feed"] })
            setRefreshing(false)
          }}
        >
          <RefreshCw className={`size-4 ${refreshing ? "animate-spin" : ""}`} />
        </Button>
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
            const isOwn = entry.user_id === auth?.user_id
            const card = (
              <ReviewCard
                movie={entry.movie}
                review={entry.review}
                userName={entry.user_display_name}
                userId={entry.user_id}
                isFederated={entry.is_federated}
                actorUrl={entry.actor_url}
                onEdit={isOwn ? () => setEditingEntry(entry) : undefined}
                onShowDetail={entry.review.comment ? () => setDetailEntry(entry) : undefined}
              />
            )
            return isOwn ? (
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

      {editingEntry && (
        <ReviewSheet
          key={editingEntry.review.id}
          mode="edit"
          open={!!editingEntry}
          onOpenChange={(open) => !open && setEditingEntry(null)}
          movie={editingEntry.movie}
          review={editingEntry.review}
        />
      )}

      {detailEntry && (
        <ReviewDetailSheet
          open={!!detailEntry}
          onOpenChange={(open) => !open && setDetailEntry(null)}
          movie={detailEntry.movie}
          review={detailEntry.review}
          userName={detailEntry.user_display_name}
        />
      )}
    </div>
  )
}
