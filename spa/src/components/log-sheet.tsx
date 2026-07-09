import { useState } from "react"
import { useTranslation } from "react-i18next"
import { VisuallyHidden } from "radix-ui"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { Button } from "@/components/ui/button"
import { ReviewFormFields } from "@/components/review-form-fields"
import { SearchOverlay } from "@/components/search-overlay"
import type { MovieSelection } from "@/components/search-overlay"
import { useLogReview } from "@/hooks/use-diary"
import { toast } from "sonner"
import { posterUrl } from "@/lib/api/client"
import { hapticMedium } from "@/lib/haptics"

type LogSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function LogSheet({ open, onOpenChange }: LogSheetProps) {
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
              <div className="mb-5 flex gap-3">
                <div className="h-24 w-16 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
                  {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />}
                </div>
                <div>
                  <p className="text-lg font-bold">{movie.title}</p>
                  <p className="text-sm text-muted-foreground">{movie.release_year}{movie.director && ` · ${movie.director}`}</p>
                  {movie.genres.length > 0 && <p className="mt-1 text-xs text-muted-foreground">{movie.genres.join(", ")}</p>}
                </div>
              </div>

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
