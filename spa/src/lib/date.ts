import { formatDistanceToNow, parseISO, format } from "date-fns"

export function timeAgo(dateStr: string): string {
  try {
    const date = dateStr.includes("T") ? parseISO(dateStr) : new Date(dateStr.replace(" ", "T"))
    return formatDistanceToNow(date, { addSuffix: true })
  } catch {
    return dateStr.slice(0, 10)
  }
}

export function shortDate(dateStr: string): string {
  try {
    const date = dateStr.includes("T") ? parseISO(dateStr) : new Date(dateStr.replace(" ", "T"))
    return format(date, "MMM d, yyyy")
  } catch {
    return dateStr.slice(0, 10)
  }
}

// `watched_at` is stored as naive UTC ("YYYY-MM-DDTHH:MM:SS", no zone).
// Date-only imports carry no time and are stored at exactly 00:00:00; those
// are calendar dates, not instants, and must not be shifted between zones.

export function isDateOnly(s: string): boolean {
  return /^\d{4}-\d{2}-\d{2}([T ]00:00(:00)?)?$/.test(s)
}

export function parseWatchedAt(s: string): Date {
  const [y, m, d] = s.slice(0, 10).split("-").map(Number)
  if (isDateOnly(s)) return new Date(y!, m! - 1, d!)
  const parsed = new Date(`${s.replace(" ", "T")}Z`)
  return Number.isNaN(parsed.getTime()) ? new Date() : parsed
}

export function formatWatchedAt(d: Date, dateOnly = false): string {
  const pad = (n: number) => n.toString().padStart(2, "0")
  if (dateOnly) {
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T00:00:00`
  }
  return d.toISOString().split(".")[0]!
}

// Swap the calendar day of `current` for `picked` (the date picker yields local
// midnight), keeping the time of day so the stored UTC instant stays on the
// chosen day.
export function withPickedDay(current: Date, picked: Date): Date {
  const next = new Date(current)
  next.setFullYear(picked.getFullYear(), picked.getMonth(), picked.getDate())
  return next
}
