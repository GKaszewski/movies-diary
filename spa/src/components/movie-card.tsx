import { Link } from "@tanstack/react-router"
import { StarDisplay } from "@/components/star-display"
import { Card, CardContent } from "@/components/ui/card"
import { posterUrl } from "@/lib/api/client"
import type { MovieDto } from "@/lib/api/common"

type MovieCardProps = {
  movie: MovieDto
  rating?: number
  comment?: string
  subtitle?: React.ReactNode
  variant?: "compact" | "full"
  action?: React.ReactNode
  onShowDetail?: () => void
}

export function MovieCard({ movie, rating, comment, subtitle, variant = "full", action, onShowDetail }: MovieCardProps) {
  if (variant === "compact") {
    return (
      <Link to="/movies/$id" params={{ id: movie.id }} className="glass flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors active:bg-muted/50">
        <div className="size-9 w-9 flex-shrink-0 overflow-hidden rounded-md bg-muted">
          {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="size-full object-cover" />}
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold">{movie.title}</p>
          {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
          {comment && (
            <p
              className="truncate text-xs text-muted-foreground/70"
              role={onShowDetail ? "button" : undefined}
              tabIndex={onShowDetail ? 0 : undefined}
              onClick={onShowDetail ? (e) => { e.preventDefault(); onShowDetail() } : undefined}
              onKeyDown={onShowDetail ? (e) => e.key === "Enter" && onShowDetail() : undefined}
            >{comment}</p>
          )}
        </div>
        {rating != null && <StarDisplay rating={rating} size="xs" />}
      </Link>
    )
  }

  return (
    <Link to="/movies/$id" params={{ id: movie.id }} className="block transition-colors active:bg-muted/50">
      <Card size="sm">
        <CardContent className="flex gap-3">
          <div className="h-[84px] w-14 flex-shrink-0 overflow-hidden rounded-lg bg-muted">
            {movie.poster_path && <img src={posterUrl(movie.poster_path)} alt="" className="aero-poster-hover size-full object-cover" />}
          </div>
          <div className="min-w-0 flex-1">
            <p className="font-semibold">{movie.title}</p>
            <p className="text-xs text-muted-foreground">{movie.release_year}{movie.director && ` · ${movie.director}`}</p>
            {rating != null && <div className="mt-1"><StarDisplay rating={rating} /></div>}
            {comment && (
              <p
                className="mt-1 line-clamp-2 text-xs text-muted-foreground"
                role={onShowDetail ? "button" : undefined}
                tabIndex={onShowDetail ? 0 : undefined}
                onClick={onShowDetail ? (e) => { e.preventDefault(); onShowDetail() } : undefined}
                onKeyDown={onShowDetail ? (e) => e.key === "Enter" && onShowDetail() : undefined}
              >{comment}</p>
            )}
          </div>
          {action && <div className="flex items-center" onClick={(e) => e.preventDefault()}>{action}</div>}
        </CardContent>
      </Card>
    </Link>
  )
}
