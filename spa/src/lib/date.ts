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
