import { useRef, useState } from "react"
import { useDrag } from "@use-gesture/react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"

type SwipeTabsProps = {
  tabs: readonly { value: string; label: string }[]
  defaultValue?: string
  value?: string
  onValueChange?: (value: string) => void
  children: (value: string) => React.ReactNode
  tabsListClassName?: string
}

export function SwipeTabs({
  tabs,
  defaultValue,
  value: controlledValue,
  onValueChange,
  children,
  tabsListClassName,
}: SwipeTabsProps) {
  const [internalValue, setInternalValue] = useState(defaultValue ?? tabs[0]!.value)
  const value = controlledValue ?? internalValue
  const setValue = onValueChange ?? setInternalValue

  const containerRef = useRef<HTMLDivElement>(null)
  const [offsetX, setOffsetX] = useState(0)
  const [swiping, setSwiping] = useState(false)

  const currentIndex = tabs.findIndex((t) => t.value === value)

  const bind = useDrag(
    ({ movement: [mx], direction: [dx], active }) => {
      if (active) {
        setSwiping(true)
        setOffsetX(mx)
      } else {
        setSwiping(false)
        setOffsetX(0)
        if (Math.abs(mx) > 50) {
          const nextIndex = dx < 0
            ? Math.min(currentIndex + 1, tabs.length - 1)
            : Math.max(currentIndex - 1, 0)
          if (nextIndex !== currentIndex) {
            setValue(tabs[nextIndex]!.value)
          }
        }
      }
    },
    { axis: "x", filterTaps: true, pointer: { touch: true } },
  )

  return (
    <Tabs value={value} onValueChange={setValue}>
      <TabsList className={tabsListClassName}>
        {tabs.map((tab) => (
          <TabsTrigger key={tab.value} value={tab.value}>
            {tab.label}
          </TabsTrigger>
        ))}
      </TabsList>
      <div
        ref={containerRef}
        {...bind()}
        className="touch-pan-y"
        style={{
          transform: swiping ? `translateX(${offsetX * 0.3}px)` : undefined,
          transition: swiping ? "none" : "transform 200ms ease-out",
        }}
      >
        {children(value)}
      </div>
    </Tabs>
  )
}
