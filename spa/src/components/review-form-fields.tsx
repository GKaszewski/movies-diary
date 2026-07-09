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

      <Textarea value={comment} onChange={(e) => onCommentChange(e.target.value)} placeholder={t("logReview.commentPlaceholder")} className="mb-5" rows={3} />

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
