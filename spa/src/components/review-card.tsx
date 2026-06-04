import { Link } from "@tanstack/react-router"
import { StarDisplay } from "@/components/star-display"
import { Card, CardContent } from "@/components/ui/card"
import { posterUrl } from "@/lib/api/client"
import type { MovieDto, ReviewDto } from "@/lib/api/common"

type ReviewCardProps = {
  movie: MovieDto
  review: ReviewDto
  userName?: string
  userId?: string
}

export function ReviewCard({ movie, review, userName, userId }: ReviewCardProps) {
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
              ) : (
                <span>{userName}</span>
              )}
              <span>·</span>
              <span>{review.watched_at.slice(0, 10)}</span>
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
