import {
  createFileRoute,
  Outlet,
  redirect,
  useMatches,
} from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Toaster } from "@/components/ui/sonner"
import { BottomTabBar } from "@/components/bottom-tab-bar"
import { ReviewSheet } from "@/components/review-sheet"
import { getAuth } from "@/lib/auth"

export const Route = createFileRoute("/_app")({
  beforeLoad: () => {
    if (!getAuth()) throw redirect({ to: "/login" })
  },
  component: AppLayout,
  errorComponent: ErrorFallback,
})

function ErrorFallback({ error, reset }: { error: unknown; reset: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-4 p-6 text-center">
      <p className="text-lg font-semibold">{t("errors.somethingWrong")}</p>
      <p className="text-sm text-muted-foreground">
        {error instanceof Error ? error.message : t("errors.unknownError")}
      </p>
      <Button onClick={reset}>
        {t("common.tryAgain")}
      </Button>
    </div>
  )
}

function AppLayout() {
  const [logOpen, setLogOpen] = useState(false)
  const matches = useMatches()
  const routeKey = matches.at(-1)?.id ?? ""

  return (
    <div className="mx-auto min-h-svh max-w-lg">
      <main className="pb-20">
        <div key={routeKey} className="aero-page-in">
          <Outlet />
        </div>
      </main>
      <BottomTabBar onLogTap={() => setLogOpen(true)} />
      <ReviewSheet mode="log" open={logOpen} onOpenChange={setLogOpen} />
      <Toaster position="top-center" />
    </div>
  )
}
