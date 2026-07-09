import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { WATCH_MEDIUMS } from "@/lib/watch-mediums"

type WatchMediumBadgeProps = {
  medium: string
  className?: string
}

export function WatchMediumBadge({ medium, className }: WatchMediumBadgeProps) {
  const { t } = useTranslation()
  const entry = WATCH_MEDIUMS.find((m) => m.value === medium)
  if (!entry) return null

  const Icon = entry.icon

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button type="button" variant="ghost" size="icon" className={cn("size-6", className)} aria-label={t(entry.labelKey)}>
          <Icon className="size-3.5 text-muted-foreground" />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{t(entry.labelKey)}</TooltipContent>
    </Tooltip>
  )
}
