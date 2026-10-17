import { useTranslation } from "react-i18next"
import { Globe, Users } from "lucide-react"
import { StarDisplay } from "@/components/star-display"
import { SpoilerVeil } from "@/components/spoiler-veil"
import { WatchMediumBadge } from "@/components/watch-medium-badge"
import { EmptyState } from "@/components/empty-state"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { timeAgo } from "@/lib/date"
import type { SocialReviewDto } from "@/features/movies"

type CommunityReviewsProps = {
  reviews: { items: SocialReviewDto[] }
  /** Authenticated viewer id — own reviews are never veiled. */
  currentUserId?: string
  onShowDetail?: (review: SocialReviewDto) => void
}

export function CommunityReviews({ reviews, currentUserId, onShowDetail }: CommunityReviewsProps) {
  const { t } = useTranslation()

  return (
    <section>
      <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{t("movie.community")}</h3>
      {!reviews.items.length ? (
        <EmptyState icon={Users} title={t("movie.noReviews")} description={t("movie.beFirst")} />
      ) : (
        <div className="space-y-2">
          {reviews.items.map((r, i) => {
            const veiled = !!r.spoilers && currentUserId !== r.user_id
            return (
              <Card key={i} size="sm">
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle className="flex items-center gap-1.5 text-sm">
                        {r.user_display}
                        {r.is_federated && <Globe className="size-3 text-muted-foreground/60" />}
                      </CardTitle>
                      <CardDescription className="text-[10px]">{timeAgo(r.watched_at)}</CardDescription>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <StarDisplay rating={r.rating} size="xs" />
                      {r.watch_medium && <WatchMediumBadge medium={r.watch_medium} />}
                    </div>
                  </div>
                </CardHeader>
                {r.comment && (
                  <CardContent>
                    {veiled ? (
                      <SpoilerVeil content={r.comment} className="text-xs text-muted-foreground" />
                    ) : (
                      <p
                        className="text-xs text-muted-foreground"
                        role={onShowDetail ? "button" : undefined}
                        tabIndex={onShowDetail ? 0 : undefined}
                        onClick={veiled ? undefined : onShowDetail ? () => onShowDetail(r) : undefined}
                        onKeyDown={onShowDetail ? (e) => e.key === "Enter" && onShowDetail(r) : undefined}
                      >{r.comment}</p>
                    )}
                  </CardContent>
                )}
              </Card>
            )
          })}
        </div>
      )}
    </section>
  )
}
