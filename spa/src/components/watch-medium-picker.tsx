import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { WATCH_MEDIUMS } from "@/lib/watch-mediums"

type WatchMediumPickerProps = {
  value?: string
  onChange: (value: string | undefined) => void
}

export function WatchMediumPicker({ value, onChange }: WatchMediumPickerProps) {
  const { t } = useTranslation()

  return (
    <div>
      <p className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">
        {t("watchMedium.label")}
      </p>
      <TooltipProvider>
        <div className="flex flex-wrap gap-1.5">
          {WATCH_MEDIUMS.map(({ value: val, icon: Icon, labelKey }) => {
            const selected = value === val
            return (
              <Tooltip key={val}>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className={cn(
                      "size-8",
                      selected && "border-[var(--aero-primary)] bg-[var(--aero-primary)] text-white shadow-[0_0_8px_var(--aero-primary-glow)]",
                    )}
                    aria-label={t(labelKey)}
                    aria-pressed={selected}
                    onClick={() => onChange(selected ? undefined : val)}
                  >
                    <Icon className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent sideOffset={4}>{t(labelKey)}</TooltipContent>
              </Tooltip>
            )
          })}
        </div>
      </TooltipProvider>
    </div>
  )
}
