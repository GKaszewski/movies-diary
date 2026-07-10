import { useMemo } from "react"
import { Calendar } from "@/components/ui/calendar"
import { posterUrl } from "@/lib/api/client"
import type { DiaryEntryDto } from "@/lib/api/common"
import type { DayButton } from "react-day-picker"

type DiaryCalendarProps = {
  entries: DiaryEntryDto[]
  onSelectDate?: (entries: DiaryEntryDto[]) => void
}

function parseDate(dateStr: string): string {
  return dateStr.slice(0, 10)
}

export function DiaryCalendar({ entries, onSelectDate }: DiaryCalendarProps) {
  const byDate = useMemo(() => {
    const map = new Map<string, DiaryEntryDto[]>()
    for (const e of entries) {
      const key = parseDate(e.review.watched_at)
      const list = map.get(key)
      if (list) list.push(e)
      else map.set(key, [e])
    }
    return map
  }, [entries])

  const watchedDates = useMemo(
    () => [...byDate.keys()].map((d) => new Date(d + "T00:00:00")),
    [byDate],
  )

  return (
    <Calendar
      mode="single"
      classNames={{
        month: "flex w-full flex-col gap-2",
        day: "group/day relative aspect-square h-full w-full rounded-(--cell-radius) p-0 text-center select-none",
      }}
      modifiers={{ watched: watchedDates }}
      modifiersClassNames={{ watched: "bg-primary/20 rounded-(--cell-radius)" }}
      onDayClick={(day) => {
        const key = day.toISOString().slice(0, 10)
        const dayEntries = byDate.get(key)
        if (dayEntries?.length && onSelectDate) onSelectDate(dayEntries)
      }}
      components={{
        DayButton: (props) => (
          <CalendarDay {...props} byDate={byDate} />
        ),
      }}
    />
  )
}

function CalendarDay({
  day,
  byDate,
  ...props
}: React.ComponentProps<typeof DayButton> & {
  byDate: Map<string, DiaryEntryDto[]>
}) {
  const key = day.date.toISOString().slice(0, 10)
  const dayEntries = byDate.get(key)
  const poster = dayEntries?.[0]?.movie.poster_path

  return (
    <button
      type="button"
      className="relative flex size-full min-h-10 flex-col items-center justify-start rounded-(--cell-radius) p-0.5 text-xs transition-colors hover:bg-muted/50"
      {...props}
    >
      <span className="leading-tight">{day.date.getDate()}</span>
      {poster && (
        <img
          src={posterUrl(poster)}
          alt=""
          className="mt-0.5 aspect-[2/3] w-full max-w-6 rounded-sm object-cover opacity-80"
        />
      )}
      {dayEntries && dayEntries.length > 1 && (
        <span className="absolute bottom-0.5 right-0.5 flex size-3.5 items-center justify-center rounded-full bg-primary text-[8px] font-bold text-primary-foreground">
          {dayEntries.length}
        </span>
      )}
      {dayEntries && dayEntries.length === 1 && !poster && (
        <span className="mt-1 size-1.5 rounded-full bg-primary" />
      )}
    </button>
  )
}
