import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { VisuallyHidden } from "radix-ui"
import {
  startOfMonth,
  endOfMonth,
  startOfWeek,
  endOfWeek,
  addMonths,
  subMonths,
  eachDayOfInterval,
  isSameMonth,
  isToday,
  format,
} from "date-fns"
import { Button } from "@/components/ui/button"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { StarDisplay } from "@/components/star-display"
import { WATCH_MEDIUMS } from "@/lib/watch-mediums"
import { posterUrl } from "@/lib/api/client"
import { shortDate } from "@/lib/date"
import type { DiaryEntryDto } from "@/lib/api/common"

type DiaryCalendarProps = {
  entries: DiaryEntryDto[]
  onSelectEntry?: (entry: DiaryEntryDto) => void
}

const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]

export function DiaryCalendar({ entries, onSelectEntry }: DiaryCalendarProps) {
  const [month, setMonth] = useState(() => startOfMonth(new Date()))
  const [dayEntries, setDayEntries] = useState<DiaryEntryDto[] | null>(null)

  const byDate = useMemo(() => {
    const map = new Map<string, DiaryEntryDto[]>()
    for (const e of entries) {
      const key = e.review.watched_at.slice(0, 10)
      const list = map.get(key)
      if (list) list.push(e)
      else map.set(key, [e])
    }
    return map
  }, [entries])

  const days = useMemo(() => {
    const start = startOfWeek(startOfMonth(month))
    const end = endOfWeek(endOfMonth(month))
    return eachDayOfInterval({ start, end })
  }, [month])

  function handleDayClick(clicked: DiaryEntryDto[]) {
    if (clicked.length === 1) {
      onSelectEntry?.(clicked[0]!)
    } else {
      setDayEntries(clicked)
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setMonth((m) => subMonths(m, 1))}
        >
          <ChevronLeft className="size-4" />
        </Button>
        <span className="text-sm font-medium">
          {format(month, "MMMM yyyy")}
        </span>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setMonth((m) => addMonths(m, 1))}
        >
          <ChevronRight className="size-4" />
        </Button>
      </div>

      <div className="grid grid-cols-7 gap-px">
        {WEEKDAYS.map((d) => (
          <div
            key={d}
            className="py-1 text-center text-xs font-medium text-muted-foreground"
          >
            {d}
          </div>
        ))}

        {days.map((day) => {
          const key = format(day, "yyyy-MM-dd")
          const de = byDate.get(key)
          const inMonth = isSameMonth(day, month)
          const today = isToday(day)
          const poster = de?.[0]?.movie.poster_path

          return (
            <Button
              key={key}
              variant="ghost"
              disabled={!de?.length}
              onClick={() => de?.length && handleDayClick(de)}
              className={`relative flex h-auto min-h-12 flex-col items-center gap-0.5 rounded-lg p-1 text-xs ${
                !inMonth ? "opacity-30" : ""
              } ${today ? "ring-1 ring-primary/50" : ""}`}
            >
              <span
                className={
                  de?.length
                    ? "font-semibold text-primary"
                    : "text-muted-foreground"
                }
              >
                {day.getDate()}
              </span>
              {poster && (
                <img
                  src={posterUrl(poster)}
                  alt=""
                  className="aspect-2/3 w-7 rounded-sm object-cover"
                />
              )}
              {de && de.length > 1 && (
                <span className="absolute top-0.5 right-0.5 flex size-4 items-center justify-center rounded-full bg-primary text-[9px] font-bold text-primary-foreground">
                  {de.length}
                </span>
              )}
              {de && de.length === 1 && !poster && (
                <span className="size-1.5 rounded-full bg-primary" />
              )}
            </Button>
          )
        })}
      </div>

      <DayDrawer
        entries={dayEntries}
        onClose={() => setDayEntries(null)}
        onSelect={(e) => {
          setDayEntries(null)
          onSelectEntry?.(e)
        }}
      />
    </div>
  )
}

function DayDrawer({
  entries,
  onClose,
  onSelect,
}: {
  entries: DiaryEntryDto[] | null
  onClose: () => void
  onSelect: (entry: DiaryEntryDto) => void
}) {
  const { t } = useTranslation()
  if (!entries) return null

  const dateLabel = shortDate(entries[0]!.review.watched_at)

  return (
    <Drawer open onOpenChange={(open) => !open && onClose()}>
      <DrawerContent className="mx-auto max-w-lg">
        <VisuallyHidden.Root>
          <DrawerTitle>{dateLabel}</DrawerTitle>
        </VisuallyHidden.Root>
        <div className="p-4 pb-8">
          <p className="mb-3 text-sm font-semibold">
            {dateLabel} &middot; {t("common.films", { count: entries.length })}
          </p>
          <div className="space-y-2">
            {entries.map((e) => (
              <Button
                key={e.review.id}
                variant="ghost"
                onClick={() => onSelect(e)}
                className="flex h-auto w-full items-center justify-start gap-3 rounded-xl p-2"
              >
                <div className="h-16 w-11 shrink-0 overflow-hidden rounded-lg bg-muted">
                  {e.movie.poster_path && (
                    <img
                      src={posterUrl(e.movie.poster_path)}
                      alt=""
                      className="size-full object-cover"
                    />
                  )}
                </div>
                <div className="min-w-0 flex-1 text-left">
                  <p className="truncate text-sm font-semibold">
                    {e.movie.title}
                  </p>
                  <div className="mt-0.5 flex items-center gap-1.5">
                    <StarDisplay rating={e.review.rating} size="xs" />
                    {e.review.watch_medium && (() => {
                      const Icon = WATCH_MEDIUMS.find((d) => d.value === e.review.watch_medium)?.icon
                      return Icon ? <Icon className="size-3.5 text-muted-foreground" /> : null
                    })()}
                  </div>
                  {e.review.comment && (
                    <p className="mt-0.5 truncate text-xs text-muted-foreground">
                      {e.review.comment}
                    </p>
                  )}
                </div>
              </Button>
            ))}
          </div>
        </div>
      </DrawerContent>
    </Drawer>
  )
}
