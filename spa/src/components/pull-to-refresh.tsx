import { useRef, useState } from "react"
import { useDrag } from "@use-gesture/react"
import { Spinner } from "@/components/ui/spinner"

type PullToRefreshProps = {
  onRefresh: () => Promise<unknown>
  children: React.ReactNode
}

export function PullToRefresh({ onRefresh, children }: PullToRefreshProps) {
  const [pullY, setPullY] = useState(0)
  const [refreshing, setRefreshing] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  const bind = useDrag(
    ({ movement: [, my], active, first, memo }) => {
      if (first) {
        const scrollTop = containerRef.current?.closest("[data-scroll]")?.scrollTop
          ?? document.documentElement.scrollTop
          ?? 0
        memo = scrollTop <= 0
      }
      if (!memo) return memo

      if (active) {
        setPullY(Math.max(0, Math.min(my * 0.4, 80)))
      } else {
        if (my > 80 && !refreshing) {
          setRefreshing(true)
          setPullY(40)
          onRefresh().finally(() => {
            setRefreshing(false)
            setPullY(0)
          })
        } else {
          setPullY(0)
        }
      }
      return memo
    },
    { axis: "y", filterTaps: true },
  )

  return (
    <div ref={containerRef} {...bind()} className="touch-pan-y">
      <div
        className="flex items-center justify-center overflow-hidden transition-[height] duration-200"
        style={{ height: pullY }}
      >
        {(pullY > 0 || refreshing) && (
          <Spinner className="size-5" />
        )}
      </div>
      {children}
    </div>
  )
}
