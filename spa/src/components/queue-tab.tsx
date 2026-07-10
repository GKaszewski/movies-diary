import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Inbox } from "lucide-react"
import { EmptyState } from "@/components/empty-state"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { StarRating } from "@/components/star-rating"
import { useWatchQueue, useConfirmWatch, useDismissWatch } from "@/features/webhooks"
import { FeedSkeleton } from "@/components/feed-tab"

export function QueueTab() {
  const { t } = useTranslation()
  const { data, isPending } = useWatchQueue()
  const confirmMutation = useConfirmWatch()
  const dismissMutation = useDismissWatch()
  const [ratings, setRatings] = useState<Record<string, number>>({})
  const [comments, setComments] = useState<Record<string, string>>({})

  if (isPending) return <FeedSkeleton />
  if (!data?.length)
    return <EmptyState icon={Inbox} title={t("feed.queueEmpty")} description={t("feed.queueEmptyDesc")} />

  return (
    <div className="space-y-3">
      {data.map((entry) => (
        <div key={entry.id} className="rounded-xl bg-card p-3">
          <p className="font-semibold">{entry.title}</p>
          <p className="text-xs text-muted-foreground">
            {entry.year && `${entry.year} · `}{entry.source} · {entry.watched_at}
          </p>
          <div className="mt-2">
            <StarRating
              value={ratings[entry.id] ?? 0}
              onChange={(v) => setRatings((p) => ({ ...p, [entry.id]: v }))}
              size="sm"
            />
          </div>
          <Textarea
            className="mt-2"
            placeholder={t("logReview.commentPlaceholder")}
            value={comments[entry.id] ?? ""}
            onChange={(e) => setComments((p) => ({ ...p, [entry.id]: e.target.value }))}
            rows={2}
          />
          <div className="mt-2 flex gap-2">
            <Button
              size="sm"
              disabled={!ratings[entry.id]}
              onClick={() =>
                confirmMutation.mutate({
                  confirmations: [{
                    watch_event_id: entry.id,
                    rating: ratings[entry.id]!,
                    comment: comments[entry.id] || undefined,
                  }],
                })
              }
            >
              {t("common.confirm")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => dismissMutation.mutate({ event_ids: [entry.id] })}
            >
              {t("common.dismiss")}
            </Button>
          </div>
        </div>
      ))}
    </div>
  )
}
