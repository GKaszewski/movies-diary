import { useState } from "react"
import { useTranslation } from "react-i18next"
import { VisuallyHidden } from "radix-ui"
import { Drawer, DrawerContent, DrawerTitle } from "@/components/ui/drawer"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useCreateGoal, useUpdateGoal } from "@/features/goals"
import { toast } from "sonner"

type GoalSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  editYear?: number
  editTarget?: number
}

export function GoalSheet({
  open,
  onOpenChange,
  editYear,
  editTarget,
}: GoalSheetProps) {
  const { t } = useTranslation()
  const isEditing = editYear !== undefined
  const currentYear = new Date().getFullYear()

  const [year, setYear] = useState(editYear ?? currentYear)
  const [target, setTarget] = useState(editTarget ?? 52)
  const createMutation = useCreateGoal()
  const updateMutation = useUpdateGoal()

  function handleClose() {
    onOpenChange(false)
    if (!isEditing) {
      setYear(currentYear)
      setTarget(52)
    }
  }

  function handleSubmit() {
    if (target < 1) return

    if (isEditing) {
      updateMutation.mutate(
        { year, data: { target_count: target } },
        {
          onSuccess: () => {
            toast.success(t("goals.updated"))
            handleClose()
          },
        },
      )
    } else {
      createMutation.mutate(
        { year, target_count: target },
        {
          onSuccess: () => {
            toast.success(t("goals.created"))
            handleClose()
          },
        },
      )
    }
  }

  const isPending = createMutation.isPending || updateMutation.isPending

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="px-4 pb-8">
        <VisuallyHidden.Root>
          <DrawerTitle>
            {isEditing ? t("goals.editGoal") : t("goals.setGoal")}
          </DrawerTitle>
        </VisuallyHidden.Root>

        <div className="mx-auto w-full max-w-sm space-y-6 pt-4">
          <h2 className="text-lg font-semibold text-center">
            {isEditing ? t("goals.editGoal") : t("goals.setGoal")}
          </h2>

          <div className="space-y-2">
            <Label>{t("goals.year")}</Label>
            <Input
              type="number"
              min={2020}
              max={2100}
              value={year}
              onChange={(e) => setYear(Number(e.target.value))}
              disabled={isEditing}
            />
          </div>

          <div className="space-y-2">
            <Label>{t("goals.targetMovies")}</Label>
            <Input
              type="number"
              min={1}
              max={9999}
              value={target}
              onChange={(e) => setTarget(Number(e.target.value))}
            />
          </div>

          <Button
            className="w-full"
            onClick={handleSubmit}
            disabled={isPending || target < 1}
          >
            {isPending
              ? t("common.saving")
              : isEditing
                ? t("common.save")
                : t("goals.setGoal")}
          </Button>
        </div>
      </DrawerContent>
    </Drawer>
  )
}
