import { useEffect, useRef } from "react"
import { useWindowVirtualizer } from "@tanstack/react-virtual"
import { Spinner } from "@/components/ui/spinner"

type VirtualListProps<T> = {
  items: T[]
  estimateSize: number
  renderItem: (item: T, index: number) => React.ReactNode
  hasMore?: boolean
  isFetching?: boolean
  onLoadMore?: () => void
  overscan?: number
}

export function VirtualList<T>({
  items,
  estimateSize,
  renderItem,
  hasMore = false,
  isFetching = false,
  onLoadMore,
  overscan = 5,
}: VirtualListProps<T>) {
  const listRef = useRef<HTMLDivElement>(null)

  const virtualizer = useWindowVirtualizer({
    count: items.length,
    estimateSize: () => estimateSize,
    overscan,
    scrollMargin: listRef.current?.offsetTop ?? 0,
  })

  const virtualItems = virtualizer.getVirtualItems()
  const lastItem = virtualItems.at(-1)

  useEffect(() => {
    if (!lastItem || !hasMore || isFetching || !onLoadMore) return
    if (lastItem.index >= items.length - 5) {
      onLoadMore()
    }
  }, [lastItem?.index, items.length, hasMore, isFetching, onLoadMore])

  return (
    <div ref={listRef}>
      <div
        className="relative w-full"
        style={{ height: virtualizer.getTotalSize() }}
      >
        {virtualItems.map((virtualRow) => (
          <div
            key={virtualRow.key}
            data-index={virtualRow.index}
            ref={virtualizer.measureElement}
            className="absolute left-0 top-0 w-full"
            style={{ transform: `translateY(${virtualRow.start - (virtualizer.options.scrollMargin ?? 0)}px)` }}
          >
            <div className="pb-2">
              {renderItem(items[virtualRow.index]!, virtualRow.index)}
            </div>
          </div>
        ))}
      </div>
      {isFetching && (
        <div className="flex justify-center py-4">
          <Spinner className="size-5" />
        </div>
      )}
    </div>
  )
}
