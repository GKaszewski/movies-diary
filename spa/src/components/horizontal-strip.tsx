import { useRef, useState, useEffect, useCallback } from "react"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { Button } from "@/components/ui/button"

type HorizontalStripProps = {
  children: React.ReactNode
  className?: string
  gap?: string
}

export function HorizontalStrip({ children, className, gap = "gap-3" }: HorizontalStripProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [canScrollLeft, setCanScrollLeft] = useState(false)
  const [canScrollRight, setCanScrollRight] = useState(false)

  const update = useCallback(() => {
    const el = ref.current
    if (!el) return
    setCanScrollLeft(el.scrollLeft > 0)
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 1)
  }, [])

  useEffect(() => {
    const el = ref.current
    if (!el) return
    update()
    el.addEventListener("scroll", update, { passive: true })
    const ro = new ResizeObserver(update)
    ro.observe(el)
    return () => {
      el.removeEventListener("scroll", update)
      ro.disconnect()
    }
  }, [update])

  function scroll(dir: -1 | 1) {
    ref.current?.scrollBy({ left: dir * ref.current.clientWidth * 0.75, behavior: "smooth" })
  }

  return (
    <div className={`group relative ${className ?? ""}`}>
      {canScrollLeft && (
        <Button
          variant="secondary"
          size="icon"
          className="absolute -left-1 top-1/3 z-10 size-8 rounded-full opacity-0 shadow-md transition-opacity group-hover:opacity-100"
          onClick={() => scroll(-1)}
        >
          <ChevronLeft className="size-4" />
        </Button>
      )}
      <div
        ref={ref}
        className={`-mx-4 flex ${gap} overflow-x-auto overscroll-x-contain px-4 pb-2`}
        style={{ scrollbarWidth: "thin", scrollbarColor: "rgba(255,255,255,0.15) transparent" }}
      >
        {children}
      </div>
      {canScrollRight && (
        <Button
          variant="secondary"
          size="icon"
          className="absolute -right-1 top-1/3 z-10 size-8 rounded-full opacity-0 shadow-md transition-opacity group-hover:opacity-100"
          onClick={() => scroll(1)}
        >
          <ChevronRight className="size-4" />
        </Button>
      )}
    </div>
  )
}
