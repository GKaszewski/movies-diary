import { useState } from "react"
import { useTranslation } from "react-i18next"
import { VisuallyHidden } from "radix-ui"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { Button } from "@/components/ui/button"
import { ReviewFormFields } from "@/components/review-form-fields"
import { SearchOverlay } from "@/components/search-overlay"
import type { MovieSelection } from "@/components/search-overlay"
import { useLogReview, useEditReview } from "@/features/diary"
import { toast } from "sonner"
import { posterUrl } from "@/lib/api/client"
import { hapticMedium } from "@/lib/haptics"
import { parseLocalDate, formatLocalDateTime } from "@/lib/date"
import type { EditReviewRequest } from "@/features/diary"
import type { MovieDto, ReviewDto } from "@/lib/api/common"

type LogMode = {
  mode: "log"
}

type EditMode = {
  mode: "edit"
  movie: MovieDto
  review: ReviewDto
}

type ReviewSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
} & (LogMode | EditMode)

export function ReviewSheet(props: ReviewSheetProps) {
  if (props.mode === "log") {
    return <LogMode open={props.open} onOpenChange={props.onOpenChange} />
  }
  return (
    <EditMode
      open={props.open}
      onOpenChange={props.onOpenChange}
      movie={props.movie}
      review={props.review}
    />
  )
}

function LogMode({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation()
  const [movie, setMovie] = useState<MovieSelection | null>(null)
  const [rating, setRating] = useState(0)
  const [comment, setComment] = useState("")
  const [watchedAt, setWatchedAt] = useState<Date>(new Date())
  const [watchMedium, setWatchMedium] = useState<string | undefined>()
  const logMutation = useLogReview()

  function reset() {
    setMovie(null)
    setRating(0)
    setComment("")
    setWatchedAt(new Date())
    setWatchMedium(undefined)
  }

  function handleClose() {
    onOpenChange(false)
    reset()
  }

  function handleSubmit() {
    if (!movie || !rating) return
    logMutation.mutate(
      {
        external_metadata_id: movie.external_metadata_id,
        manual_title: movie.title,
        manual_release_year: movie.release_year,
        manual_director: movie.director,
        rating,
        comment: comment || undefined,
        watched_at: watchedAt.toISOString().replace("Z", "").split(".")[0]!,
        watch_medium: watchMedium,
      },
      {
        onSuccess: () => {
          hapticMedium()
          toast.success(t("logReview.logged", { title: movie.title }))
          handleClose()
        },
      },
    )
  }

  if (open && !movie) {
    return <SearchOverlay open onClose={handleClose} onSelect={(m) => setMovie(m)} />
  }

  return (
    <Drawer open={open && !!movie} onOpenChange={(o) => !o && handleClose()}>
      <DrawerContent className="mx-auto max-w-lg">
        <VisuallyHidden.Root><DrawerTitle>{t("logReview.title")}</DrawerTitle></VisuallyHidden.Root>
        <div className="p-5 pb-8">
          {movie && (
            <>
              <MovieHeader
                title={movie.title}
                releaseYear={movie.release_year}
                director={movie.director}
                posterPath={movie.poster_path}
                genres={movie.genres}
              />

              <ReviewFormFields
                rating={rating}
                onRatingChange={setRating}
                comment={comment}
                onCommentChange={setComment}
                watchedAt={watchedAt}
                onWatchedAtChange={setWatchedAt}
                watchMedium={watchMedium}
                onWatchMediumChange={setWatchMedium}
              />

              <Button onClick={handleSubmit} disabled={!rating || logMutation.isPending} className="w-full" size="lg">
                {logMutation.isPending ? t("logReview.logging") : t("logReview.logReview")}
              </Button>
            </>
          )}
        </div>
      </DrawerContent>
    </Drawer>
  )
}

function EditMode({
  open,
  onOpenChange,
  movie,
  review,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  movie: MovieDto
  review: ReviewDto
}) {
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
          <MovieHeader
            title={movie.title}
            releaseYear={movie.release_year}
            director={movie.director}
            posterPath={movie.poster_path}
          />

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

function MovieHeader({
  title,
  releaseYear,
  director,
  posterPath,
  genres,
}: {
  title: string
  releaseYear?: number
  director?: string | null
  posterPath?: string | null
  genres?: string[]
}) {
  return (
    <div className="mb-5 flex gap-3">
      <div className="h-24 w-16 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
        {posterPath && <img src={posterUrl(posterPath)} alt="" className="size-full object-cover" />}
      </div>
      <div>
        <p className="text-lg font-bold">{title}</p>
        <p className="text-sm text-muted-foreground">{releaseYear}{director && ` · ${director}`}</p>
        {genres && genres.length > 0 && <p className="mt-1 text-xs text-muted-foreground">{genres.join(", ")}</p>}
      </div>
    </div>
  )
}
