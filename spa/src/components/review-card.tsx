import { Link } from "@tanstack/react-router"
import { Globe } from "lucide-react"
import { timeAgo } from "@/lib/date"
import { StarDisplay } from "@/components/star-display"
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
}

export function ReviewCard({ movie, review, userName, userId, isFederated, actorUrl }: ReviewCardProps) {
  return (
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
              <span>{timeAgo(review.watched_at)}</span>
            </div>
          )}
          <Link to="/movies/$id" params={{ id: movie.id }} className="font-semibold hover:underline">
            {movie.title}
          </Link>
          <StarDisplay rating={review.rating} />
          {review.comment && <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{review.comment}</p>}
        </div>
      </CardContent>
    </Card>
  )
}
