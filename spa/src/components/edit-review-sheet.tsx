import { useState } from "react"
import { useTranslation } from "react-i18next"
import { VisuallyHidden } from "radix-ui"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { Button } from "@/components/ui/button"
import { ReviewFormFields } from "@/components/review-form-fields"
import { useEditReview } from "@/hooks/use-diary"
import { toast } from "sonner"
import { posterUrl } from "@/lib/api/client"
import { hapticMedium } from "@/lib/haptics"
import type { EditReviewRequest } from "@/lib/api/diary"
import type { MovieDto, ReviewDto } from "@/lib/api/common"

type EditReviewSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  movie: MovieDto
  review: ReviewDto
}

function parseLocalDate(s: string): Date {
  const [datePart, timePart] = s.split("T")
  if (!datePart) return new Date()
  const [y, m, d] = datePart.split("-").map(Number)
  if (timePart) {
    const [h, min, sec] = timePart.split(":").map(Number)
    return new Date(y!, m! - 1, d!, h, min, sec)
  }
  return new Date(y!, m! - 1, d!)
}

function formatLocalDateTime(d: Date): string {
  const pad = (n: number) => n.toString().padStart(2, "0")
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

export function EditReviewSheet({ open, onOpenChange, movie, review }: EditReviewSheetProps) {
  const { t } = useTranslation()
  const [rating, setRating] = useState(review.rating)
  const [comment, setComment] = useState(review.comment ?? "")
  const [watchedAt, setWatchedAt] = useState<Date>(() => parseLocalDate(review.watched_at))
  const [dateChanged, setDateChanged] = useState(false)
  const [watchMedium, setWatchMedium] = useState<string | undefined>(review.watch_medium)
  const editMutation = useEditReview()

  function handleDateChange(d: Date) {
    setWatchedAt(d)
    setDateChanged(true)
  }

  function handleSubmit() {
    if (!rating) return

    const data: Partial<EditReviewRequest> = {}
    if (rating !== review.rating) data.rating = rating
    const newComment = comment || null
    if (newComment !== (review.comment ?? null)) data.comment = newComment
    if (dateChanged) data.watched_at = formatLocalDateTime(watchedAt)
    if (watchMedium !== review.watch_medium) data.watch_medium = watchMedium ?? null

    if (Object.keys(data).length === 0) {
      toast.info(t("editReview.noChanges"))
      onOpenChange(false)
      return
    }

    editMutation.mutate(
      { id: review.id, data },
      {
        onSuccess: () => {
          hapticMedium()
          toast.success(t("editReview.saved", { title: movie.title }))
          onOpenChange(false)
        },
      },
    )
  }

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="mx-auto max-w-lg">
        <VisuallyHidden.Root><DrawerTitle>{t("editReview.title")}</DrawerTitle></VisuallyHidden.Root>
        <div className="p-5 pb-8">
          <div className="mb-5 flex gap-3">
            <div className="h-24 w-16 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
              {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />}
            </div>
            <div>
              <p className="text-lg font-bold">{movie.title}</p>
              <p className="text-sm text-muted-foreground">{movie.release_year}{movie.director && ` · ${movie.director}`}</p>
            </div>
          </div>

          <ReviewFormFields
            rating={rating}
            onRatingChange={setRating}
            comment={comment}
            onCommentChange={setComment}
            watchedAt={watchedAt}
            onWatchedAtChange={handleDateChange}
            watchMedium={watchMedium}
            onWatchMediumChange={setWatchMedium}
          />

          <Button onClick={handleSubmit} disabled={!rating || editMutation.isPending} className="w-full" size="lg">
            {editMutation.isPending ? t("editReview.saving") : t("editReview.save")}
          </Button>
        </div>
      </DrawerContent>
    </Drawer>
  )
}
