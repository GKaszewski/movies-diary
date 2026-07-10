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

export function parseLocalDate(s: string): Date {
  const [datePart, timePart] = s.split("T")
  if (!datePart) return new Date()
  const [y, m, d] = datePart.split("-").map(Number)
  if (timePart) {
    const [h, min, sec] = timePart.split(":").map(Number)
    return new Date(y!, m! - 1, d!, h, min, sec)
  }
  return new Date(y!, m! - 1, d!)
}

export function formatLocalDateTime(d: Date): string {
  const pad = (n: number) => n.toString().padStart(2, "0")
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}
