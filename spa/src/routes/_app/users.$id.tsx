import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { UserCheck, UserPlus } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { Button } from "@/components/ui/button"
import { ProfileView, ProfileSkeleton } from "@/components/profile-view"
import { GoalCard } from "@/components/goal-card"
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

  const [search, setSearch] = useState("")

  if (isPending) return <ProfileSkeleton />
  if (!data) return null

  const isSelf = auth?.user_id === id
  const isFollowing = followingData?.actors.some((a) => a.handle === data.username) ?? false

  return (
    <div className="p-4">
      <div className="mb-4"><BackButton /></div>

      <ProfileView
        data={data}
        userId={id}
        search={search}
        onSearchChange={setSearch}
        actions={
          data.goals?.length ? (
            <div className="space-y-2">
              {data.goals.map((g) => (
                <GoalCard key={g.year} goal={g} />
              ))}
            </div>
          ) : undefined
        }
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
