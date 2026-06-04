import { createFileRoute, Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ChevronRight, Settings, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ProfileView, ProfileSkeleton } from "@/components/profile-view"
import { useAuth } from "@/components/auth-provider"
import { useWrapUps } from "@/hooks/use-wrapup"
import { useUserProfile } from "@/hooks/use-users"

export const Route = createFileRoute("/_app/profile")({
  component: ProfilePage,
})

function ProfilePage() {
  const { t } = useTranslation()
  const { auth } = useAuth()
  const { data, isPending } = useUserProfile(auth?.user_id ?? "", {
    view: "trends",
  })

  if (!auth) return null
  if (isPending) return <ProfileSkeleton />
  if (!data) return null

  return (
    <div className="p-4">
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-lg font-bold">{t("profile.title")}</h1>
        <Link to="/settings" className="text-muted-foreground">
          <Settings className="size-5" />
        </Link>
      </div>

      <ProfileView
        data={data}
        actions={
          <>
            <Link to="/social" className="block">
              <Button variant="outline" size="sm" className="w-full justify-between">
                <span>{t("profile.followingFollowers")}</span>
                <ChevronRight className="size-4 text-muted-foreground" />
              </Button>
            </Link>
            <WrapUpLink />
          </>
        }
      />
    </div>
  )
}

function WrapUpLink() {
  const { t } = useTranslation()
  const { data } = useWrapUps()
  const latest = data?.items?.find((w) => w.status === "completed")

  if (!latest) return null

  return (
    <Link to="/wrapup/$id" params={{ id: latest.id }}>
      <Button variant="outline" className="w-full justify-between">
        <span className="flex items-center gap-2">
          <Sparkles className="size-4" />
          {t("profile.yearInReview")}
        </span>
        <ChevronRight className="size-4 text-muted-foreground" />
      </Button>
    </Link>
  )
}
