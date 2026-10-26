import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Clock, ExternalLink, Trash2, UserCheck, UserPlus } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/confirm-dialog"
import { ProfileView, ProfileSkeleton } from "@/components/profile-view"
import { GoalCard } from "@/components/goal-card"
import { useAuth, useIsAdmin } from "@/components/auth-provider"
import { DeleteAccountDialog } from "@/components/delete-account-dialog"
import { toast } from "sonner"
import { useAdminDeleteUser, useUserProfile } from "@/features/users"
import { useFollow, useUnfollow, useCancelFollow, useRelationship } from "@/features/social"
import { useDocumentTitle } from "@/hooks/use-document-title"
import type { FollowState } from "@/features/social"

type FollowButtonContent = {
  icon: React.ReactNode
  label: string
  variant: "default" | "outline"
}

export const Route = createFileRoute("/_app/users/$id")({
  component: UserProfilePage,
})

function UserProfilePage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { auth } = useAuth()
  const { data, isPending } = useUserProfile(id, { view: "trends" })
  const { data: relation } = useRelationship(data?.actor_url ?? undefined)
  const followMutation = useFollow()
  const unfollowMutation = useUnfollow()
  const cancelFollowMutation = useCancelFollow()

  const [search, setSearch] = useState("")
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [deleteOpen, setDeleteOpen] = useState(false)
  const isAdmin = useIsAdmin()
  const navigate = useNavigate()
  const adminDelete = useAdminDeleteUser()
  useDocumentTitle(data?.username)

  if (isPending) return <ProfileSkeleton />
  if (!data) return null
  const isSelf = auth?.user_id === id
  const followState: FollowState = relation?.following ?? "none"

  // The follow endpoint requires the full "user@domain" handle — a bare
  // username 500s server-side (CompositeSocialAdapter.resolve_target_identity
  // only recognizes the local instance when the handle carries "@<host>").
  // Arrow consts, not function declarations: hoisted declarations would not see
  // the `if (!data) return null` narrowing above.
  const sendFollowRequest = () => {
    followMutation.mutate({ handle: data.handle ?? data.username })
  }

  const handleFollowButtonClick = () => {
    switch (followState) {
      case "pending":
        setConfirmOpen(true)
        return
      case "accepted":
        unfollowMutation.mutate({ actor_url: data.actor_url ?? "" })
        return
      case "rejected":
        // A rejected request leaves nothing pending to cancel — let the user ask again.
        sendFollowRequest()
        return
      case "none":
        sendFollowRequest()
        return
    }
  }

  function followButtonContent(): FollowButtonContent {
    switch (followState) {
      case "pending":
        return {
          icon: <Clock className="mr-1 size-3.5" />,
          label: t("social.requested", { defaultValue: "Requested" }),
          variant: "outline",
        }
      case "accepted":
        return {
          icon: <UserCheck className="mr-1 size-3.5" />,
          label: t("common.following"),
          variant: "outline",
        }
      case "rejected":
        return {
          icon: <UserPlus className="mr-1 size-3.5" />,
          label: t("common.follow"),
          variant: "default",
        }
      case "none":
        return {
          icon: <UserPlus className="mr-1 size-3.5" />,
          label: t("common.follow"),
          variant: "default",
        }
    }
  }

  const { icon: followIcon, label: followLabel, variant: followVariant } = followButtonContent()

  return (
    <div className="p-4">
      <div className="mb-4"><BackButton /></div>

      <ProfileView
        data={data}
        userId={id}
        search={data.is_federated ? undefined : search}
        onSearchChange={data.is_federated ? undefined : setSearch}
        isFederated={data.is_federated}
        bio={data.bio}
        handle={data.handle}
        actorUrl={data.actor_url}
        actions={
          !data.is_federated && data.goals?.length ? (
            <div className="space-y-2">
              {data.goals.map((g) => (
                <GoalCard key={g.year} goal={g} />
              ))}
            </div>
          ) : undefined
        }
        headerRight={
          !isSelf ? (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant={followVariant}
                onClick={handleFollowButtonClick}
                disabled={followMutation.isPending || unfollowMutation.isPending}
              >
                {followIcon}
                {followLabel}
              </Button>
              {data.is_federated && data.actor_url && (
                <a href={data.actor_url} target="_blank" rel="noopener noreferrer">
                  <Button size="sm" variant="outline">
                    <ExternalLink className="mr-1 size-3.5" />
                    {t("common.viewOnRemote", { defaultValue: "Remote" })}
                  </Button>
                </a>
              )}
              {isAdmin && !data.is_federated && !data.is_service_account && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setDeleteOpen(true)}
                  className="border-red-400/60 text-red-400 hover:text-red-300"
                >
                  <Trash2 className="mr-1 size-3.5" />
                  {t("deleteUser.action")}
                </Button>
              )}
              <DeleteAccountDialog
                open={deleteOpen}
                onOpenChange={(o) => {
                  if (!o) adminDelete.reset()
                  setDeleteOpen(o)
                }}
                title={t("deleteUser.title", { name: data.display_name ?? data.username })}
                description={t("deleteUser.warning")}
                passwordLabel={t("deleteUser.adminPassword")}
                confirmLabel={t("deleteUser.confirm")}
                pending={adminDelete.isPending}
                error={adminDelete.error}
                onSubmit={(password) =>
                  adminDelete.mutate(
                    { id, password },
                    {
                      onSuccess: () => {
                        toast.success(t("deleteUser.done"))
                        setDeleteOpen(false)
                        navigate({ to: "/social" })
                      },
                    },
                  )
                }
              />
              <ConfirmDialog
                open={confirmOpen}
                onOpenChange={setConfirmOpen}
                title={t("social.cancelFollowRequest", { defaultValue: "Cancel follow request?" })}
                confirmLabel={t("social.cancelRequest", { defaultValue: "Cancel request" })}
                onConfirm={() => {
                  cancelFollowMutation.mutate({ actor_url: data.actor_url ?? "" })
                  setConfirmOpen(false)
                }}
              />
            </div>
          ) : undefined
        }
      />
    </div>
  )
}
