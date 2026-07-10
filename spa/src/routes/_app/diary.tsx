import { createFileRoute } from "@tanstack/react-router"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { BookOpen, ChevronLeft, ChevronRight, Pencil } from "lucide-react"
import { format, startOfMonth, subMonths } from "date-fns"
import { ReviewSheet } from "@/components/review-sheet"
import { EditableContextMenu } from "@/components/editable-context-menu"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeToDelete } from "@/components/swipe-to-delete"
import { WatchMediumBadge } from "@/components/watch-medium-badge"
import { VirtualList } from "@/components/virtual-list"
import { Button } from "@/components/ui/button"
import { Skeleton } from "@/components/ui/skeleton"
import { useAuth } from "@/components/auth-provider"
import { useInfiniteDiary, useDeleteReview } from "@/features/diary"
import { useDocumentTitle } from "@/hooks/use-document-title"
import type { DiaryEntryDto } from "@/lib/api/common"

export const Route = createFileRoute("/_app/diary")({
  component: DiaryPage,
})

function groupByDate(items: DiaryEntryDto[]) {
  const groups: Record<string, DiaryEntryDto[]> = {}
  for (const entry of items) {
    const date = entry.review.watched_at.slice(0, 10)
    ;(groups[date] ??= []).push(entry)
  }
  return Object.entries(groups).sort(([a], [b]) => b.localeCompare(a))
}

function DiaryPage() {
  const { t } = useTranslation()
  const { auth } = useAuth()
  useDocumentTitle(t("diary.title"))
  const [month, setMonth] = useState(() => startOfMonth(new Date()))
  const { data, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } =
    useInfiniteDiary({ sort_by: "desc", user_id: auth?.user_id })
  const deleteReview = useDeleteReview()
  const [editingEntry, setEditingEntry] = useState<DiaryEntryDto | null>(null)

  const monthLabel = format(month, "MMMM yyyy")
  const monthStr = format(month, "yyyy-MM")

  const allItems = data?.pages.flatMap((p) => p.items) ?? []
  const filtered = allItems.filter((e) => e.review.watched_at.startsWith(monthStr))
  const grouped = groupByDate(filtered)
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  type FlatItem =
    | { type: "header"; date: string }
    | { type: "entry"; entry: DiaryEntryDto }

  const flatItems: FlatItem[] = grouped.flatMap(([date, entries]) => [
    { type: "header" as const, date },
    ...entries.map((entry) => ({ type: "entry" as const, entry })),
  ])

  const activeMonths = [...new Set(allItems.map((e) => e.review.watched_at.slice(0, 7)))].sort()

  const prevMonth = activeMonths.filter((m) => m < monthStr).at(-1)
  const nextMonth = activeMonths.filter((m) => m > monthStr).find(() => true)

  const canGoBack = hasNextPage || !!prevMonth
  const canGoForward = !!nextMonth && startOfMonth(new Date(nextMonth + "-01")) <= startOfMonth(new Date())

  function goBack() {
    if (prevMonth) {
      setMonth(startOfMonth(new Date(prevMonth + "-01")))
    } else {
      setMonth((m) => subMonths(m, 1))
    }
  }

  function goForward() {
    if (nextMonth) {
      setMonth(startOfMonth(new Date(nextMonth + "-01")))
    }
  }

  return (
    <div className="space-y-4 p-4">
      <h1 className="text-lg font-bold">{t("diary.title")}</h1>

      <div className="flex items-center justify-between rounded-xl bg-card px-3 py-2">
        <Button variant="ghost" size="icon" onClick={goBack} disabled={!canGoBack}>
          <ChevronLeft className="size-5" />
        </Button>
        <span className="text-sm font-medium">{monthLabel}</span>
        <Button variant="ghost" size="icon" onClick={goForward} disabled={!canGoForward}>
          <ChevronRight className="size-5" />
        </Button>
      </div>

      {isPending && <DiarySkeleton />}

      {!isPending && grouped.length === 0 && (
        <EmptyState icon={BookOpen} title={t("diary.noEntries")} description={t("diary.nothingLogged")} />
      )}

      {flatItems.length > 0 && (
        <VirtualList
          items={flatItems}
          estimateSize={80}
          hasMore={!!hasNextPage}
          isFetching={isFetchingNextPage}
          onLoadMore={loadMore}
          renderItem={(item) =>
            item.type === "header" ? (
              <h2 className="pt-2 text-xs font-medium text-muted-foreground">{item.date}</h2>
            ) : (
              <SwipeToDelete
                onDelete={() => deleteReview.mutate(item.entry.review.id)}
                confirmTitle={t("diary.deleteReview")}
                confirmDescription={`${item.entry.movie.title} — ${item.entry.review.watched_at.slice(0, 10)}`}
              >
                <EditableContextMenu onEdit={() => setEditingEntry(item.entry)}>
                  <MovieCard
                    movie={item.entry.movie}
                    rating={item.entry.review.rating}
                    comment={item.entry.review.comment}
                    variant="full"
                    action={
                      <div className="flex items-center gap-1">
                        {item.entry.review.watch_medium && (
                          <WatchMediumBadge medium={item.entry.review.watch_medium} />
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="hidden size-7 md:inline-flex"
                          onClick={(e) => { e.preventDefault(); setEditingEntry(item.entry) }}
                        >
                          <Pencil className="size-3.5" />
                        </Button>
                      </div>
                    }
                  />
                </EditableContextMenu>
              </SwipeToDelete>
            )
          }
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
    </div>
  )
}

function DiarySkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3].map((i) => (
        <div key={i} className="flex gap-3 rounded-xl bg-card p-3">
          <Skeleton className="h-[84px] w-14 rounded-lg" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-3 w-24" />
          </div>
        </div>
      ))}
    </div>
  )
}
