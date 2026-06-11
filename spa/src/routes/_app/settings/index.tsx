import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { useMutation } from "@tanstack/react-query"
import {
  ArrowLeft,
  ChevronRight,
  Download,
  Key,
  LogOut,
  RefreshCw,
  ShieldBan,
  Sparkles,
  Target,
  Upload,
  User,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { useAuth, useIsAdmin } from "@/components/auth-provider"
import { reindexSearch } from "@/lib/api/users"
import { useSettings, useUpdateSettings } from "@/hooks/use-goals"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/settings/")({
  component: SettingsPage,
})

type SettingsItem = {
  label: string
  description?: string
  to: string
  icon: React.ReactNode
}

function SettingsPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("settings.title"))
  const { logout } = useAuth()
  const isAdmin = useIsAdmin()
  const navigate = useNavigate()

  const account: SettingsItem[] = [
    {
      label: t("settings.editProfile"),
      description: t("settings.editProfileDesc"),
      to: "/settings/edit-profile",
      icon: <User className="size-4" />,
    },
  ]

  const data: SettingsItem[] = [
    {
      label: t("settings.import"),
      description: t("settings.importDesc"),
      to: "/settings/import",
      icon: <Upload className="size-4" />,
    },
    {
      label: t("settings.export"),
      description: t("settings.exportDesc"),
      to: "/settings/export",
      icon: <Download className="size-4" />,
    },
    {
      label: t("settings.yearWrapUp"),
      description: t("settings.yearWrapUpDesc"),
      to: "/settings/wrapup",
      icon: <Sparkles className="size-4" />,
    },
  ]

  const integrations: SettingsItem[] = [
    {
      label: t("settings.webhookTokens"),
      description: t("settings.webhookTokensDesc"),
      to: "/settings/webhooks",
      icon: <Key className="size-4" />,
    },
  ]

  const social: SettingsItem[] = [
    {
      label: isAdmin ? t("settings.blockedUsersAndDomains") : t("settings.blockedUsers"),
      description: isAdmin ? t("settings.blockedUsersDescAdmin") : t("settings.blockedUsersDesc"),
      to: "/settings/blocked",
      icon: <ShieldBan className="size-4" />,
    },
  ]

  const handleLogout = () => {
    logout()
    navigate({ to: "/login" })
  }

  return (
    <div className="space-y-6 p-4">
      <div className="flex items-center gap-3">
        <Link to="/profile" className="text-muted-foreground">
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-lg font-bold">{t("settings.title")}</h1>
      </div>

      <SettingsGroup label={t("settings.account")} items={account} />
      <SettingsGroup label={t("settings.data")} items={data} />
      <SettingsGroup label={t("settings.integrations")} items={integrations} />
      <SettingsGroup label={t("settings.socialGroup")} items={social} />

      <PrivacySection />

      {isAdmin && <AdminActions />}

      <button
        onClick={handleLogout}
        className="w-full rounded-xl bg-card p-3 text-sm font-medium text-red-400"
      >
        <div className="flex items-center gap-3">
          <LogOut className="size-4" />
          {t("settings.logOut")}
        </div>
      </button>
    </div>
  )
}

function PrivacySection() {
  const { t } = useTranslation()
  const { data: settings } = useSettings()
  const updateMutation = useUpdateSettings()

  return (
    <div>
      <p className="mb-1.5 px-1 text-xs font-medium text-muted-foreground">
        {t("settings.privacy")}
      </p>
      <div className="divide-y divide-border rounded-xl bg-card">
        <div className="flex items-center gap-3 p-3">
          <span className="text-muted-foreground">
            <Target className="size-4" />
          </span>
          <div className="flex-1">
            <p className="text-sm font-medium">{t("settings.federateGoals")}</p>
            <p className="text-xs text-muted-foreground">
              {t("settings.federateGoalsDesc")}
            </p>
          </div>
          <Switch
            checked={settings?.federate_goals ?? false}
            onCheckedChange={(checked) =>
              updateMutation.mutate({ federate_goals: checked })
            }
            disabled={updateMutation.isPending}
          />
        </div>
      </div>
    </div>
  )
}

function AdminActions() {
  const { t } = useTranslation()
  const reindex = useMutation({
    mutationFn: reindexSearch,
  })

  return (
    <div>
      <p className="mb-1.5 px-1 text-xs font-medium text-muted-foreground">
        {t("settings.admin")}
      </p>
      <div className="divide-y divide-border rounded-xl bg-card">
        <div className="flex items-center gap-3 p-3">
          <span className="text-muted-foreground">
            <RefreshCw className={`size-4 ${reindex.isPending ? "animate-spin" : ""}`} />
          </span>
          <div className="flex-1">
            <p className="text-sm font-medium">{t("settings.rebuildSearch")}</p>
            <p className="text-xs text-muted-foreground">
              {reindex.isSuccess ? t("settings.rebuildSearchDone") : t("settings.rebuildSearchDesc")}
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={() => reindex.mutate()} disabled={reindex.isPending}>
            {reindex.isPending ? t("common.generating") : t("common.run")}
          </Button>
        </div>
      </div>
    </div>
  )
}

function SettingsGroup({
  label,
  items,
}: {
  label: string
  items: SettingsItem[]
}) {
  return (
    <div>
      <p className="mb-1.5 px-1 text-xs font-medium text-muted-foreground">
        {label}
      </p>
      <div className="divide-y divide-border rounded-xl bg-card">
        {items.map((item) => (
          <Link
            key={item.label}
            to={item.to}
            className="flex items-center gap-3 p-3"
          >
            <span className="text-muted-foreground">{item.icon}</span>
            <div className="flex-1">
              <p className="text-sm font-medium">{item.label}</p>
              {item.description && (
                <p className="text-xs text-muted-foreground">
                  {item.description}
                </p>
              )}
            </div>
            <ChevronRight className="size-4 text-muted-foreground" />
          </Link>
        ))}
      </div>
    </div>
  )
}
