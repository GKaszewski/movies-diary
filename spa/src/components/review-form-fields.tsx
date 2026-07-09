import { useCallback, useRef } from "react"
import { useTranslation } from "react-i18next"
import { CalendarIcon } from "lucide-react"
import { format } from "date-fns"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Calendar } from "@/components/ui/calendar"
import { StarRating } from "@/components/star-rating"
import { WatchMediumPicker } from "@/components/watch-medium-picker"

type ReviewFormFieldsProps = {
  rating: number
  onRatingChange: (v: number) => void
  comment: string
  onCommentChange: (v: string) => void
  watchedAt: Date
  onWatchedAtChange: (v: Date) => void
  watchMedium?: string
  onWatchMediumChange: (v: string | undefined) => void
}

export function ReviewFormFields({
  rating,
  onRatingChange,
  comment,
  onCommentChange,
  watchedAt,
  onWatchedAtChange,
  watchMedium,
  onWatchMediumChange,
}: ReviewFormFieldsProps) {
  const { t } = useTranslation()

  return (
    <>
      <div className="mb-5 text-center">
        <p className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">{t("logReview.yourRating")}</p>
        <div className="flex justify-center"><StarRating value={rating} onChange={onRatingChange} /></div>
      </div>

      <AutoGrowTextarea value={comment} onChange={onCommentChange} placeholder={t("logReview.commentPlaceholder")} className="mb-5" />

      <div className="mb-5">
        <p className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">{t("logReview.watchedAt")}</p>
        <Popover modal>
          <PopoverTrigger asChild>
            <Button variant="outline" className="w-full justify-start text-left font-normal">
              <CalendarIcon className="mr-2 size-4" />
              {format(watchedAt, "PPP")}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-auto p-0" align="start">
            <Calendar
              mode="single"
              fixedWeeks
              selected={watchedAt}
              onSelect={(d) => d && onWatchedAtChange(d)}
              disabled={(d) => d > new Date()}
              autoFocus
            />
          </PopoverContent>
        </Popover>
      </div>

      <div className="mb-5">
        <WatchMediumPicker value={watchMedium} onChange={onWatchMediumChange} />
      </div>
    </>
  )
}

function AutoGrowTextarea({
  value,
  onChange,
  placeholder,
  className,
}: {
  value: string
  onChange: (v: string) => void
  placeholder?: string
  className?: string
}) {
  const ref = useRef<HTMLTextAreaElement>(null)
  const handleInput = useCallback(() => {
    const el = ref.current
    if (!el) return
    el.style.height = "auto"
    el.style.height = `${el.scrollHeight}px`
  }, [])

  return (
    <Textarea
      ref={ref}
      value={value}
      onChange={(e) => {
        onChange(e.target.value)
        handleInput()
      }}
      placeholder={placeholder}
      className={className}
      rows={2}
      style={{ overflow: "hidden", resize: "none" }}
    />
  )
}
