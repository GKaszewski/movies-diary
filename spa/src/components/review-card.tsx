import { Link } from "@tanstack/react-router"
import { Globe, Pencil } from "lucide-react"
import { TimeAgo } from "@/components/time-ago"
import { StarDisplay } from "@/components/star-display"
import { WatchMediumBadge } from "@/components/watch-medium-badge"
import { EditableContextMenu } from "@/components/editable-context-menu"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { posterUrl } from "@/lib/api/client"
import type { MovieDto, ReviewDto } from "@/lib/api/common"

type ReviewCardProps = {
  movie: MovieDto
  review: ReviewDto
  userName?: string
  userId?: string
  isFederated?: boolean
  actorUrl?: string
  onEdit?: () => void
  onShowDetail?: () => void
}

export function ReviewCard({ movie, review, userName, userId, isFederated, actorUrl, onEdit, onShowDetail }: ReviewCardProps) {
  const card = (
    <Card size="sm">
      <CardContent className="flex gap-3">
        <Link to="/movies/$id" params={{ id: movie.id }} className="h-[84px] w-14 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
          {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />}
        </Link>
        <div className="min-w-0 flex-1">
          {userName && (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              {userId ? (
                <Link to="/users/$id" params={{ id: userId }} className="relative z-10 font-semibold text-primary">
                  {userName}
                </Link>
              ) : actorUrl ? (
                <a href={actorUrl} target="_blank" rel="noopener noreferrer" className="relative z-10 font-semibold text-primary">
                  {userName}
                </a>
              ) : (
                <span>{userName}</span>
              )}
              {isFederated && <Globe className="size-3 text-muted-foreground/60" />}
              <span>·</span>
              <TimeAgo date={review.watched_at} />
            </div>
          )}
          <div className="flex items-center justify-between">
            <Link to="/movies/$id" params={{ id: movie.id }} className="font-semibold hover:underline">
              {movie.title}
            </Link>
            {onEdit && (
              <Button variant="ghost" size="icon" className="hidden size-7 md:inline-flex" onClick={onEdit}>
                <Pencil className="size-3.5" />
              </Button>
            )}
          </div>
          <div className="flex items-center gap-1.5">
            <StarDisplay rating={review.rating} />
            {review.watch_medium && <WatchMediumBadge medium={review.watch_medium} />}
          </div>
          {review.comment && (
            <p
              className="mt-1 line-clamp-2 text-xs text-muted-foreground"
              role={onShowDetail ? "button" : undefined}
              tabIndex={onShowDetail ? 0 : undefined}
              onClick={onShowDetail}
              onKeyDown={onShowDetail ? (e) => e.key === "Enter" && onShowDetail() : undefined}
            >
              {review.comment}
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  )

  if (!onEdit) return card

  return <EditableContextMenu onEdit={onEdit}>{card}</EditableContextMenu>
}
