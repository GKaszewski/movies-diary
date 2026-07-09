import { VisuallyHidden } from "radix-ui"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { StarDisplay } from "@/components/star-display"
import { WatchMediumBadge } from "@/components/watch-medium-badge"
import { shortDate } from "@/lib/date"
import { posterUrl } from "@/lib/api/client"
import type { MovieDto, ReviewDto } from "@/lib/api/common"

type ReviewDetailSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  movie: MovieDto
  review: ReviewDto
  userName?: string
}

export function ReviewDetailSheet({ open, onOpenChange, movie, review, userName }: ReviewDetailSheetProps) {
  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="mx-auto max-w-lg">
        <VisuallyHidden.Root><DrawerTitle>{movie.title}</DrawerTitle></VisuallyHidden.Root>
        <div className="p-5 pb-8">
          <div className="mb-4 flex gap-3">
            <div className="h-24 w-16 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
              {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />}
            </div>
            <div>
              <p className="text-lg font-bold">{movie.title}</p>
              <p className="text-sm text-muted-foreground">
                {movie.release_year}{movie.director && ` · ${movie.director}`}
              </p>
              {userName && <p className="mt-1 text-xs text-muted-foreground">{userName}</p>}
            </div>
          </div>

          <div className="mb-4 flex items-center gap-2">
            <StarDisplay rating={review.rating} />
            {review.watch_medium && <WatchMediumBadge medium={review.watch_medium} />}
            <span className="text-xs text-muted-foreground">{shortDate(review.watched_at)}</span>
          </div>

          {review.comment && (
            <p className="select-text whitespace-pre-wrap text-sm leading-relaxed">
              {review.comment}
            </p>
          )}
        </div>
      </DrawerContent>
    </Drawer>
  )
}
