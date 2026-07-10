import { createFileRoute } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { SwipeTabs } from "@/components/swipe-tabs"
import { FeedTab } from "@/components/feed-tab"
import { WatchlistTab } from "@/components/watchlist-tab"
import { QueueTab } from "@/components/queue-tab"

export const Route = createFileRoute("/_app/")({
  component: HomePage,
})

function HomePage() {
  const { t } = useTranslation()
  const homeTabs = [
    { value: "feed", label: t("feed.tab") },
    { value: "watchlist", label: t("feed.watchlist") },
    { value: "queue", label: t("feed.queue") },
  ] as const

  return (
    <div className="p-4">
      <div className="mb-3 flex items-center justify-between">
        <h1 className="text-lg font-bold">{t("feed.title")}</h1>
      </div>
      <SwipeTabs tabs={homeTabs} defaultValue="feed" tabsListClassName="w-full">
        {(tab) => (
          <>
            {tab === "feed" && <FeedTab />}
            {tab === "watchlist" && <WatchlistTab />}
            {tab === "queue" && <QueueTab />}
          </>
        )}
      </SwipeTabs>
    </div>
  )
}
