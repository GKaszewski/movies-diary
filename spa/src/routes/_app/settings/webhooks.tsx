import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, Key, Plus, Trash2 } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer"
import { Skeleton } from "@/components/ui/skeleton"
import { EmptyState } from "@/components/empty-state"
import {
  useWebhookTokens,
  useGenerateToken,
  useDeleteToken,
} from "@/hooks/use-webhooks"

export const Route = createFileRoute("/_app/settings/webhooks")({
  component: WebhooksPage,
})

function WebhooksPage() {
  const { t } = useTranslation()
  const { data: tokens, isPending } = useWebhookTokens()
  const generate = useGenerateToken()
  const remove = useDeleteToken()

  const [open, setOpen] = useState(false)
  const [provider, setProvider] = useState("jellyfin")
  const [label, setLabel] = useState("")

  const handleGenerate = () => {
    generate.mutate(
      { provider, label: label || undefined },
      {
        onSuccess: (data) => {
          navigator.clipboard.writeText(data.webhook_url)
          toast.success(t("webhooks.copied"))
          setOpen(false)
          setLabel("")
        },
      },
    )
  }

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link to="/settings" className="text-muted-foreground">
            <ArrowLeft className="size-5" />
          </Link>
          <h1 className="text-lg font-bold">{t("webhooks.title")}</h1>
        </div>
        <button onClick={() => setOpen(true)} className="text-primary">
          <Plus className="size-5" />
        </button>
      </div>

      {isPending ? (
        <div className="space-y-2">
          {[1, 2].map((i) => (
            <Skeleton key={i} className="h-14 rounded-xl" />
          ))}
        </div>
      ) : !tokens?.length ? (
        <EmptyState icon={Key} title={t("webhooks.noTokens")} description={t("webhooks.noTokensDesc")} />
      ) : (
        <div className="space-y-2">
          {tokens.map((t) => (
            <div
              key={t.id}
              className="flex items-center justify-between rounded-xl bg-card p-3"
            >
              <div>
                <p className="text-sm font-medium">
                  {t.provider}
                  {t.label && ` — ${t.label}`}
                </p>
                <p className="text-xs text-muted-foreground">
                  {new Date(t.created_at).toLocaleDateString()}
                </p>
              </div>
              <button
                onClick={() => remove.mutate(t.id)}
                className="text-destructive"
              >
                <Trash2 className="size-4" />
              </button>
            </div>
          ))}
        </div>
      )}

      <Drawer open={open} onOpenChange={setOpen}>
        <DrawerContent>
          <DrawerHeader>
            <DrawerTitle>{t("webhooks.generateToken")}</DrawerTitle>
          </DrawerHeader>
          <div className="space-y-3 p-4">
            <div className="space-y-1.5">
              <Label>{t("webhooks.provider")}</Label>
              <Select value={provider} onValueChange={setProvider}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="jellyfin">{t("webhooks.jellyfin")}</SelectItem>
                  <SelectItem value="plex">{t("webhooks.plex")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>{t("webhooks.labelOptional")}</Label>
              <Input
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder={t("webhooks.labelPlaceholder")}
              />
            </div>
            <Button
              onClick={handleGenerate}
              disabled={generate.isPending}
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
