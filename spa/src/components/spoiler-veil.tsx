import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

type SpoilerVeilProps = {
  content: string
  /** Layout classes for the veiled text (line clamps, sizing, color). */
  className?: string
}

/**
 * Hides review text behind a blur with a reveal button — shown to everyone
 * except the review's author. Once revealed, the text stays visible.
 */
export function SpoilerVeil({ content, className }: SpoilerVeilProps) {
  const { t } = useTranslation()
  const [revealed, setRevealed] = useState(false)

  if (revealed) {
    return <span className={cn("block whitespace-pre-wrap", className)}>{content}</span>
  }

  return (
    <span className="relative block" aria-label={t("review.containsSpoilers")}>
      <span
        aria-hidden="true"
        className={cn("pointer-events-none block select-none blur-[5px]", className)}
      >
        {content}
      </span>
      <Button
        type="button"
        variant="secondary"
        size="sm"
        className="absolute inset-y-0 left-0 h-6 self-center px-2 text-[10px]"
        onClick={() => setRevealed(true)}
      >
        {t("review.showSpoilers")}
      </Button>
    </span>
  )
}
