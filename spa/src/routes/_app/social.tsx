import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, UserCheck, UserMinus, UserPlus, UserX, Users } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Skeleton } from "@/components/ui/skeleton"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { EmptyState } from "@/components/empty-state"
import { toast } from "sonner"
import { useAuth } from "@/components/auth-provider"
import {
  useFollow,
  useFollowing,
  useFollowers,
  usePendingFollowers,
  useUnfollow,
  useAcceptFollower,
  useRejectFollower,
  useRemoveFollower,
  useUserFollowing,
  useUserFollowers,
} from "@/hooks/use-social"
import { useDocumentTitle } from "@/hooks/use-document-title"
import type { RemoteActorDto } from "@/lib/api/social"

type SearchParams = { user?: string }

export const Route = createFileRoute("/_app/social")({
  validateSearch: (search: Record<string, unknown>): SearchParams => ({
    user: typeof search.user === "string" ? search.user : undefined,
  }),
  component: SocialPage,
})

function SocialPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("social.title"))
  const { user: viewUserId } = Route.useSearch()
  const { auth } = useAuth()
  const isSelf = !viewUserId || viewUserId === auth?.user_id

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center gap-3">
        <Link to={isSelf ? "/profile" : "/users/$id"} params={{ id: viewUserId ?? "" }} className="text-muted-foreground">
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-lg font-bold">{t("social.title")}</h1>
      </div>

      {isSelf && <FollowByHandle />}

      {isSelf ? <OwnSocialTabs /> : <UserSocialTabs userId={viewUserId!} />}
    </div>
  )
}

function OwnSocialTabs() {
  const { t } = useTranslation()
  return (
    <Tabs defaultValue="following">
      <TabsList className="w-full">
        <TabsTrigger value="following">{t("social.following")}</TabsTrigger>
        <TabsTrigger value="followers">{t("social.followers")}</TabsTrigger>
        <TabsTrigger value="pending">{t("social.pending")}</TabsTrigger>
      </TabsList>
      <TabsContent value="following"><OwnFollowingTab /></TabsContent>
      <TabsContent value="followers"><OwnFollowersTab /></TabsContent>
      <TabsContent value="pending"><PendingTab /></TabsContent>
    </Tabs>
  )
}

function UserSocialTabs({ userId }: { userId: string }) {
  const { t } = useTranslation()
  return (
    <Tabs defaultValue="following">
      <TabsList className="w-full">
        <TabsTrigger value="following">{t("social.following")}</TabsTrigger>
        <TabsTrigger value="followers">{t("social.followers")}</TabsTrigger>
      </TabsList>
      <TabsContent value="following"><UserFollowingTab userId={userId} /></TabsContent>
      <TabsContent value="followers"><UserFollowersTab userId={userId} /></TabsContent>
    </Tabs>
  )
}

function OwnFollowingTab() {
  const { t } = useTranslation()
  const { data, isPending } = useFollowing()
  const unfollowMutation = useUnfollow()

  if (isPending) return <ListSkeleton />
  if (!data?.actors.length)
    return <EmptyState icon={Users} title={t("social.notFollowing")} description={t("social.notFollowingDesc")} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard
          key={actor.url}
          actor={actor}
          action={
            <Button
              variant="outline"
              size="sm"
              onClick={() => unfollowMutation.mutate({ actor_url: actor.url })}
              disabled={unfollowMutation.isPending}
            >
              <UserMinus className="mr-1 size-3.5" />
              {t("common.unfollow")}
            </Button>
          }
        />
      ))}
    </div>
  )
}

function OwnFollowersTab() {
  const { t } = useTranslation()
  const { data, isPending } = useFollowers()
  const removeMutation = useRemoveFollower()

  if (isPending) return <ListSkeleton />
  if (!data?.actors.length)
    return <EmptyState icon={Users} title={t("social.noFollowers")} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard
          key={actor.url}
          actor={actor}
          action={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => removeMutation.mutate({ actor_url: actor.url })}
              disabled={removeMutation.isPending}
              className="text-destructive hover:text-destructive"
            >
              <UserX className="mr-1 size-3.5" />
              {t("common.remove")}
            </Button>
          }
        />
      ))}
    </div>
  )
}

function PendingTab() {
  const { t } = useTranslation()
  const { data, isPending } = usePendingFollowers()
  const acceptMutation = useAcceptFollower()
  const rejectMutation = useRejectFollower()

  if (isPending) return <ListSkeleton />
  if (!data?.actors.length)
    return <EmptyState icon={UserCheck} title={t("social.noPending")} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard
          key={actor.url}
          actor={actor}
          action={
            <div className="flex gap-1">
              <Button size="sm" onClick={() => acceptMutation.mutate({ actor_url: actor.url })} disabled={acceptMutation.isPending}>
                {t("common.accept")}
              </Button>
              <Button variant="outline" size="sm" onClick={() => rejectMutation.mutate({ actor_url: actor.url })} disabled={rejectMutation.isPending}>
                {t("common.reject")}
              </Button>
            </div>
          }
        />
      ))}
    </div>
  )
}

function UserFollowingTab({ userId }: { userId: string }) {
  const { t } = useTranslation()
  const { data, isPending } = useUserFollowing(userId)

  if (isPending) return <ListSkeleton />
  if (!data?.actors.length)
    return <EmptyState icon={Users} title={t("social.notFollowing")} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard key={actor.url} actor={actor} />
      ))}
    </div>
  )
}

function UserFollowersTab({ userId }: { userId: string }) {
  const { t } = useTranslation()
  const { data, isPending } = useUserFollowers(userId)

  if (isPending) return <ListSkeleton />
  if (!data?.actors.length)
    return <EmptyState icon={Users} title={t("social.noFollowersOther")} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard key={actor.url} actor={actor} />
      ))}
    </div>
  )
}

function actorHandle(actor: RemoteActorDto): string {
  try {
    const host = new URL(actor.url).host
    return `@${actor.handle}@${host}`
  } catch {
    return `@${actor.handle}`
  }
}

function ActorCard({ actor, action }: { actor: RemoteActorDto; action?: React.ReactNode }) {
  const initial = (actor.display_name || actor.handle)[0]?.toUpperCase() ?? "?"

  return (
    <Card size="sm">
      <CardContent className="flex items-center gap-3">
        <Avatar>
          <AvatarFallback>{initial}</AvatarFallback>
        </Avatar>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold">{actor.display_name || actor.handle}</p>
          <p className="truncate text-xs text-muted-foreground">{actorHandle(actor)}</p>
        </div>
        {action}
      </CardContent>
    </Card>
  )
}

function FollowByHandle() {
  const { t } = useTranslation()
  const [handle, setHandle] = useState("")
  const followMutation = useFollow()

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!handle.trim()) return
    followMutation.mutate(
      { handle: handle.trim() },
      {
        onSuccess: () => {
          toast.success(t("social.followSent", { handle }))
          setHandle("")
        },
        onError: () => {
          toast.error(t("social.followError"))
        },
      },
    )
  }

  return (
    <Card size="sm">
      <CardContent>
        <form onSubmit={handleSubmit} className="flex items-center gap-2">
          <Input
            value={handle}
            onChange={(e) => setHandle(e.target.value)}
            placeholder={t("social.handlePlaceholder")}
            className="flex-1"
          />
          <Button type="submit" size="sm" disabled={!handle.trim() || followMutation.isPending}>
            <UserPlus className="mr-1 size-3.5" />
            {t("common.follow")}
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}

function ListSkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3].map((i) => (
        <Skeleton key={i} className="h-16 w-full rounded-xl" />
      ))}
    </div>
  )
}
