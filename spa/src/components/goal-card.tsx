import { useTranslation } from "react-i18next"
import { Check, MoreHorizontal, Pencil, Target, Trash2 } from "lucide-react"
import { Card, CardContent } from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Button } from "@/components/ui/button"
import type { GoalDto } from "@/features/users"

type GoalCardProps = {
  goal: GoalDto
  editable?: boolean
  onEdit?: () => void
  onDelete?: () => void
}

export function GoalCard({ goal, editable, onEdit, onDelete }: GoalCardProps) {
  const { t } = useTranslation()

  return (
    <Card>
      <CardContent className="space-y-2 py-3 px-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Target className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">
              {t("goals.yearGoal", { year: goal.year })}
            </span>
          </div>
          {editable && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="size-7 p-0">
                  <MoreHorizontal className="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={onEdit}>
                  <Pencil className="mr-2 size-3.5" />
                  {t("common.edit")}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={onDelete} className="text-destructive">
                  <Trash2 className="mr-2 size-3.5" />
                  {t("common.delete")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
        <Progress value={goal.percentage} className="h-2" />
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>
            {goal.current_count} / {goal.target_count} {t("goals.movies")}
          </span>
          <span>{Math.round(goal.percentage)}%</span>
        </div>
        {goal.is_complete && (
          <p className="text-xs text-green-500 flex items-center gap-1">
            <Check className="size-3" />
            {t("goals.reached")}
          </p>
        )}
      </CardContent>
    </Card>
  )
}
