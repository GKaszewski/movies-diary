import {
  createFileRoute,
  Outlet,
  redirect,
} from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Toaster } from "@/components/ui/sonner"
import { BottomTabBar } from "@/components/bottom-tab-bar"
import { LogSheet } from "@/components/log-sheet"
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
      <button
        onClick={reset}
        className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
      >
        {t("common.tryAgain")}
      </button>
    </div>
  )
}

function AppLayout() {
  const [logOpen, setLogOpen] = useState(false)

  return (
    <div className="mx-auto min-h-svh max-w-lg">
      <main className="pb-20">
        <Outlet />
      </main>
      <BottomTabBar onLogTap={() => setLogOpen(true)} />
      <LogSheet open={logOpen} onOpenChange={setLogOpen} />
      <Toaster position="top-center" />
    </div>
  )
}
