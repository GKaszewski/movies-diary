import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import {
  ArrowLeft,
  ChevronRight,
  Download,
  Globe,
  Key,
  LogOut,
  ShieldBan,
  Sparkles,
  User,
} from "lucide-react"
import { useAuth, useIsAdmin } from "@/components/auth-provider"

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
      label: t("settings.blockedUsers"),
      description: isAdmin ? t("settings.blockedUsersDescAdmin") : t("settings.blockedUsersDesc"),
      to: "/settings/blocked",
      icon: <ShieldBan className="size-4" />,
    },
  ]

  if (isAdmin) {
    social.push({
      label: t("settings.blockedDomains"),
      description: t("settings.blockedDomainsDesc"),
      to: "/settings/blocked",
      icon: <Globe className="size-4" />,
    })
  }

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
