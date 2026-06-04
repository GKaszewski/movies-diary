import { Link, useMatchRoute } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { Home, Search, BookOpen, User } from "lucide-react"
import { cn } from "@/lib/utils"

export function BottomTabBar({ onLogTap }: { onLogTap: () => void }) {
  const { t } = useTranslation()
  const matchRoute = useMatchRoute()

  const tabs = [
    { to: "/", icon: Home, label: t("nav.home") },
    { to: "/search", icon: Search, label: t("nav.search") },
    { to: "/diary", icon: BookOpen, label: t("nav.diary") },
    { to: "/profile", icon: User, label: t("nav.profile") },
  ] as const

  return (
    <nav className="glass-heavy fixed bottom-0 left-0 right-0 z-50 border-t border-border pb-[env(safe-area-inset-bottom)]">
      <div className="mx-auto flex max-w-lg items-center justify-around px-2 py-1">
        {tabs.slice(0, 2).map((tab) => {
          const active = matchRoute({ to: tab.to, fuzzy: tab.to !== "/" })
          return (
            <Link
              key={tab.to}
              to={tab.to}
              className={cn(
                "flex flex-col items-center gap-0.5 px-3 py-1.5 transition-colors",
                active ? "text-foreground" : "text-muted-foreground",
              )}
            >
              <tab.icon className="size-5" strokeWidth={active ? 2.5 : 2} />
              <span className="text-[10px] font-medium">{tab.label}</span>
            </Link>
          )
        })}

        {/* Center FAB */}
        <button
          onClick={onLogTap}
          className="flex flex-col items-center gap-0.5 px-3 py-1.5"
        >
          <div className="-mt-4 flex size-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg shadow-primary/30 transition-transform active:scale-90">
            <span className="text-xl font-light">+</span>
          </div>
        </button>

        {tabs.slice(2).map((tab) => {
          const active = matchRoute({ to: tab.to, fuzzy: true })
          return (
            <Link
              key={tab.to}
              to={tab.to}
              className={cn(
                "flex flex-col items-center gap-0.5 px-3 py-1.5 transition-colors",
                active ? "text-foreground" : "text-muted-foreground",
              )}
            >
              <tab.icon className="size-5" strokeWidth={active ? 2.5 : 2} />
              <span className="text-[10px] font-medium">{tab.label}</span>
            </Link>
          )
        })}
      </div>
    </nav>
  )
}
