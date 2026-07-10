import type { LucideIcon } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
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

function ListSkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3].map((i) => (
        <Skeleton key={i} className="h-16 w-full rounded-xl" />
      ))}
    </div>
  )
}
