import { useEffect, useRef } from "react"
import { Spinner } from "@/components/ui/spinner"

type InfiniteScrollProps = {
  hasMore: boolean
  isFetching: boolean
  onLoadMore: () => void
}

export function InfiniteScroll({ hasMore, isFetching, onLoadMore }: InfiniteScrollProps) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = ref.current
    if (!el || !hasMore || isFetching) return

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) onLoadMore()
      },
      { rootMargin: "200px" },
    )
    observer.observe(el)
    return () => observer.disconnect()
  }, [hasMore, isFetching, onLoadMore])

  if (!hasMore) return null

  return (
    <div ref={ref} className="flex justify-center py-4">
      {isFetching && <Spinner className="size-5" />}
    </div>
  )
}
