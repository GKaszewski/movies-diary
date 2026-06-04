import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, ChevronRight, Sparkles, Trash2 } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Skeleton } from "@/components/ui/skeleton"
import { EmptyState } from "@/components/empty-state"
import { useAuth, useIsAdmin } from "@/components/auth-provider"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import {
  useWrapUps,
  useGenerateWrapUp,
  useDeleteWrapUp,
} from "@/hooks/use-wrapup"
import { useUsers } from "@/hooks/use-users"

export const Route = createFileRoute("/_app/settings/wrapup")({
  component: WrapupPage,
})

function WrapupPage() {
  const { t } = useTranslation()
  const { auth } = useAuth()
  const isAdmin = useIsAdmin()
  const { data, isPending } = useWrapUps()
  const generate = useGenerateWrapUp()
  const remove = useDeleteWrapUp()

  const { data: usersData } = useUsers()
  const [open, setOpen] = useState(false)
  const [startDate, setStartDate] = useState("")
  const [endDate, setEndDate] = useState("")
  const [targetUserId, setTargetUserId] = useState<string>("self")

  const handleGenerate = () => {
    const user_id = targetUserId === "global" ? undefined : targetUserId === "self" ? auth?.user_id : targetUserId
    generate.mutate(
      { start_date: startDate, end_date: endDate, user_id },
      {
        onSuccess: () => {
          setOpen(false)
          setStartDate("")
          setEndDate("")
        },
      },
    )
  }

  const items = data?.items ?? []

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link to="/settings" className="text-muted-foreground">
            <ArrowLeft className="size-5" />
          </Link>
          <h1 className="text-lg font-bold">{t("wrapup.title")}</h1>
        </div>
        {isAdmin && (
          <Button variant="ghost" size="icon" onClick={() => setOpen(true)}>
            <Sparkles className="size-5" />
          </Button>
        )}
      </div>

      {isPending ? (
        <div className="space-y-2">
          {[1, 2].map((i) => (
            <Skeleton key={i} className="h-14 rounded-xl" />
          ))}
        </div>
      ) : !items.length ? (
        <EmptyState icon={Sparkles} title={t("wrapup.noWrapUps")} />
      ) : (
        <div className="space-y-2">
          {items.map((w) => (
            <Card key={w.id} size="sm">
              <CardContent className="flex items-center justify-between">
                {w.status === "Ready" ? (
                  <Link to="/wrapup/$id" params={{ id: w.id }} className="flex flex-1 items-center justify-between">
                    <div>
                      <p className="text-sm font-medium">{w.start_date} — {w.end_date}</p>
                      <Badge className="mt-1 text-[10px]">{w.status}</Badge>
                    </div>
                    <ChevronRight className="size-4 text-muted-foreground" />
                  </Link>
                ) : (
                  <div>
                    <p className="text-sm font-medium">{w.start_date} — {w.end_date}</p>
                    <Badge variant="secondary" className="mt-1 text-[10px]">{w.status}</Badge>
                  </div>
                )}
                {isAdmin && (
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => remove.mutate(w.id)}
                    className="ml-2 text-destructive hover:text-destructive"
                  >
                    <Trash2 className="size-4" />
                  </Button>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Drawer open={open} onOpenChange={setOpen}>
        <DrawerContent className="mx-auto max-w-lg">
          <DrawerHeader>
            <DrawerTitle>{t("wrapup.generateWrapUp")}</DrawerTitle>
          </DrawerHeader>
          <div className="space-y-3 p-4 pb-8">
            <div className="space-y-1.5">
              <Label>{t("wrapup.startDate")}</Label>
              <Input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>{t("wrapup.endDate")}</Label>
              <Input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
              />
            </div>
            {isAdmin && usersData?.users && (
              <div className="space-y-1.5">
                <Label>{t("wrapup.generateFor")}</Label>
                <Select value={targetUserId} onValueChange={setTargetUserId}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="self">{t("wrapup.forSelf")}</SelectItem>
                    <SelectItem value="global">{t("wrapup.forGlobal")}</SelectItem>
                    {usersData.users.map((u) => (
                      <SelectItem key={u.id} value={u.id}>
                        {u.display_name ?? u.username ?? u.email}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
            <Button
              onClick={handleGenerate}
              disabled={generate.isPending || !startDate || !endDate}
              className="w-full"
            >
              {generate.isPending ? t("common.generating") : t("common.generate")}
            </Button>
          </div>
        </DrawerContent>
      </Drawer>
    </div>
  )
}
