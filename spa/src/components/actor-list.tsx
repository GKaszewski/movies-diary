import type { LucideIcon } from "lucide-react"
import { Link } from "@tanstack/react-router"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Card, CardContent } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { EmptyState } from "@/components/empty-state"
import type { ActorListResponse, RemoteActorDto } from "@/features/social"

type ActorListProps = {
  data: ActorListResponse | undefined
  isPending: boolean
  emptyIcon: LucideIcon
  emptyTitle: string
  emptyDescription?: string
  renderAction?: (actor: RemoteActorDto) => React.ReactNode
}

export function ActorList({ data, isPending, emptyIcon, emptyTitle, emptyDescription, renderAction }: ActorListProps) {
  if (isPending) return <ListSkeleton />
  if (!data?.actors.length) return <EmptyState icon={emptyIcon} title={emptyTitle} description={emptyDescription} />

  return (
    <div className="space-y-2">
      {data.actors.map((actor) => (
        <ActorCard key={actor.url} actor={actor} action={renderAction?.(actor)} />
      ))}
    </div>
  )
}

function actorHandle(actor: RemoteActorDto): string {
  if (actor.handle.startsWith("@")) return actor.handle
  try {
    return `@${actor.handle}@${new URL(actor.url).host}`
  } catch {
    return `@${actor.handle}`
  }
}

function ActorCard({ actor, action }: { actor: RemoteActorDto; action?: React.ReactNode }) {
  const initial = (actor.display_name || actor.handle)[0]?.toUpperCase() ?? "?"

  const identity = (
    <div className="flex min-w-0 flex-1 items-center gap-3">
      <Avatar>
        {actor.avatar_url && <AvatarImage src={actor.avatar_url} />}
        <AvatarFallback>{initial}</AvatarFallback>
      </Avatar>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-semibold">{actor.display_name || actor.handle}</p>
        <p className="truncate text-xs text-muted-foreground">{actorHandle(actor)}</p>
      </div>
    </div>
  )

  return (
    <Card size="sm">
      <CardContent className="flex items-center gap-3">
        {actor.user_id ? (
          <Link to="/users/$id" params={{ id: actor.user_id }} className="min-w-0 flex-1">
            {identity}
          </Link>
        ) : (
          <a href={actor.url} target="_blank" rel="noopener noreferrer" className="min-w-0 flex-1">
            {identity}
          </a>
        )}
        {action}
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
