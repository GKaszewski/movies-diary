import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { timeAgo, shortDate } from "@/lib/date"

type TimeAgoProps = {
  date: string
  className?: string
}

export function TimeAgo({ date, className }: TimeAgoProps) {
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <time dateTime={date} className={className}>{timeAgo(date)}</time>
        </TooltipTrigger>
        <TooltipContent>{shortDate(date)}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}
