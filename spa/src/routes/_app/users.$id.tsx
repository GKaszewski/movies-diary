import { createFileRoute, Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ArrowLeft, UserCheck, UserPlus } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ProfileView, ProfileSkeleton } from "@/components/profile-view"
import { useAuth } from "@/components/auth-provider"
import { useUserProfile } from "@/hooks/use-users"
import { useFollow, useUnfollow, useFollowing } from "@/hooks/use-social"

export const Route = createFileRoute("/_app/users/$id")({
  component: UserProfilePage,
})

function UserProfilePage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { auth } = useAuth()
  const { data, isPending } = useUserProfile(id, { view: "trends" })
  const { data: followingData } = useFollowing()
  const followMutation = useFollow()
  const unfollowMutation = useUnfollow()

  if (isPending) return <ProfileSkeleton />
  if (!data) return null

  const isSelf = auth?.user_id === id
  const isFollowing = followingData?.actors.some((a) => a.handle === data.username) ?? false

  return (
    <div className="p-4">
      <Link to="/" className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground">
        <ArrowLeft className="size-4" /> {t("common.back")}
      </Link>

      <ProfileView
        data={data}
        userId={id}
        headerRight={
          !isSelf ? (
            isFollowing ? (
              <Button
                size="sm"
                variant="outline"
                onClick={() => unfollowMutation.mutate({ actor_url: followingData?.actors.find((a) => a.handle === data.username)?.url ?? "" })}
                disabled={unfollowMutation.isPending}
              >
                <UserCheck className="mr-1 size-3.5" />
                {t("common.following")}
              </Button>
            ) : (
              <Button
                size="sm"
                onClick={() => followMutation.mutate({ handle: data.username })}
                disabled={followMutation.isPending}
              >
                <UserPlus className="mr-1 size-3.5" />
                {t("common.follow")}
              </Button>
            )
          ) : undefined
        }
      />
    </div>
  )
}
