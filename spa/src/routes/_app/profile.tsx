import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ChevronDown, ChevronRight, Plus, Settings, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ProfileView, ProfileSkeleton } from "@/components/profile-view"
import { useAuth } from "@/components/auth-provider"
import { useWrapUps } from "@/hooks/use-wrapup"
import { useUserProfile } from "@/hooks/use-users"
import { useDeleteGoal } from "@/hooks/use-goals"
import { GoalCard } from "@/components/goal-card"
import { GoalSheet } from "@/components/goal-sheet"
import { toast } from "sonner"
import type { GoalDto } from "@/lib/api/users"

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
            <GoalSection goals={data.goals ?? []} />
            <Link to="/social" className="block">
              <Button variant="outline" size="sm" className="w-full justify-between">
                <span>{t("profile.followingFollowers")}</span>
                <ChevronRight className="size-4 text-muted-foreground" />
              </Button>
            </Link>
            <WrapUpLinks />
          </>
        }
      />
    </div>
  )
}

function GoalSection({ goals }: { goals: GoalDto[] }) {
  const { t } = useTranslation()
  const [sheetOpen, setSheetOpen] = useState(false)
  const [editGoal, setEditGoal] = useState<GoalDto | null>(null)
  const deleteMutation = useDeleteGoal()

  function handleEdit(goal: GoalDto) {
    setEditGoal(goal)
    setSheetOpen(true)
  }

  function handleDelete(year: number) {
    deleteMutation.mutate(year, {
      onSuccess: () => toast.success(t("goals.deleted")),
    })
  }

  function handleSheetClose(open: boolean) {
    setSheetOpen(open)
    if (!open) setEditGoal(null)
  }

  return (
    <div className="space-y-2">
      {goals.map((g) => (
        <GoalCard
          key={g.year}
          goal={g}
          editable
          onEdit={() => handleEdit(g)}
          onDelete={() => handleDelete(g.year)}
        />
      ))}
      <Button
        variant="outline"
        size="sm"
        className="w-full"
        onClick={() => setSheetOpen(true)}
      >
        <Plus className="mr-1.5 size-3.5" />
        {t("goals.setGoal")}
      </Button>
      <GoalSheet
        open={sheetOpen}
        onOpenChange={handleSheetClose}
        editYear={editGoal?.year}
        editTarget={editGoal?.target_count}
      />
    </div>
  )
}

function wrapupYear(startDate: string): string {
  return startDate.slice(0, 4)
}

function WrapUpLinks() {
  const { t } = useTranslation()
  const { data } = useWrapUps()
  const ready = (data?.items?.filter((w) => w.status === "Ready") ?? [])
    .sort((a, b) => b.start_date.localeCompare(a.start_date))
  const [expanded, setExpanded] = useState(false)

  if (!ready.length) return null

  if (ready.length === 1) {
    return (
      <Link to="/wrapup/$id" params={{ id: ready[0].id }}>
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

  return (
    <div className="space-y-1.5">
      <Button variant="outline" className="w-full justify-between" onClick={() => setExpanded(!expanded)}>
        <span className="flex items-center gap-2">
          <Sparkles className="size-4" />
          {t("profile.yearInReview")}
        </span>
        <ChevronDown className={`size-4 text-muted-foreground transition-transform ${expanded ? "rotate-180" : ""}`} />
      </Button>
      {expanded && ready.map((w) => (
        <Link key={w.id} to="/wrapup/$id" params={{ id: w.id }}>
          <Button variant="ghost" size="sm" className="w-full justify-between">
            <span>{wrapupYear(w.start_date)}</span>
            <ChevronRight className="size-4 text-muted-foreground" />
          </Button>
        </Link>
      ))}
    </div>
  )
}
