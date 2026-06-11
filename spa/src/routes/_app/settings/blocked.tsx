import { useState } from "react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ArrowLeft, ShieldBan } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { EmptyState } from "@/components/empty-state"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import { Skeleton } from "@/components/ui/skeleton"
import { useIsAdmin } from "@/components/auth-provider"
import {
  useBlockedActors,
  useUnblockActor,
  useBlockedDomains,
  useAddBlockedDomain,
  useRemoveBlockedDomain,
} from "@/hooks/use-social"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/settings/blocked")({
  component: BlockedPage,
})

function BlockedPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("blocked.title"))
  const isAdmin = useIsAdmin()

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center gap-3">
        <Link to="/settings" className="text-muted-foreground">
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-lg font-bold">{t("blocked.title")}</h1>
      </div>

      {isAdmin ? (
        <Tabs defaultValue="users">
          <TabsList className="w-full">
            <TabsTrigger value="users">{t("blocked.users")}</TabsTrigger>
            <TabsTrigger value="domains">{t("blocked.domains")}</TabsTrigger>
          </TabsList>
          <TabsContent value="users"><UsersTab /></TabsContent>
          <TabsContent value="domains"><DomainsTab /></TabsContent>
        </Tabs>
      ) : (
        <UsersTab />
      )}
    </div>
  )
}

function UsersTab() {
  const { t } = useTranslation()
  const { data: actors, isPending } = useBlockedActors()
  const unblock = useUnblockActor()

  if (isPending) {
    return (
      <div className="space-y-2">
        {[1, 2].map((i) => (
          <Skeleton key={i} className="h-12 rounded-xl" />
        ))}
      </div>
    )
  }

  if (!actors?.length) {
    return <EmptyState icon={ShieldBan} title={t("blocked.noBlockedUsers")} />
  }

  return (
    <div className="space-y-2">
      {actors.map((a) => (
        <div
          key={a.url}
          className="flex items-center justify-between rounded-xl bg-card p-3"
        >
          <div>
            <p className="text-sm font-medium">
              {a.display_name || a.handle}
            </p>
            {a.display_name && (
              <p className="text-xs text-muted-foreground">{a.handle}</p>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => unblock.mutate({ actor_url: a.url })}
          >
            {t("common.unblock")}
          </Button>
        </div>
      ))}
    </div>
  )
}

function DomainsTab() {
  const { t } = useTranslation()
  const { data: domains, isPending } = useBlockedDomains()
  const addDomain = useAddBlockedDomain()
  const removeDomain = useRemoveBlockedDomain()
  const [newDomain, setNewDomain] = useState("")

  const handleAdd = () => {
    if (!newDomain.trim()) return
    addDomain.mutate(
      { domain: newDomain.trim() },
      { onSuccess: () => setNewDomain("") },
    )
  }

  if (isPending) {
    return (
      <div className="space-y-2">
        {[1, 2].map((i) => (
          <Skeleton key={i} className="h-12 rounded-xl" />
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <Input
          value={newDomain}
          onChange={(e) => setNewDomain(e.target.value)}
          placeholder={t("blocked.domainPlaceholder")}
          className="flex-1"
          onKeyDown={(e) => e.key === "Enter" && handleAdd()}
        />
        <Button onClick={handleAdd} disabled={addDomain.isPending} size="sm">
          {t("common.block")}
        </Button>
      </div>

      {!domains?.length ? (
        <EmptyState icon={ShieldBan} title={t("blocked.noBlockedDomains")} />
      ) : (
        <div className="space-y-2">
          {domains.map((d) => (
            <div
              key={d.domain}
              className="flex items-center justify-between rounded-xl bg-card p-3"
            >
              <p className="text-sm font-medium">{d.domain}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => removeDomain.mutate(d.domain)}
              >
                {t("common.remove")}
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
