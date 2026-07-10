import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, Key, Plus, Trash2 } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
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
} from "@/features/webhooks"
import { API_URL } from "@/lib/api/client"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/settings/webhooks")({
  component: WebhooksPage,
})

function WebhooksPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("webhooks.title"))
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
        <Button variant="ghost" size="icon" onClick={() => setOpen(true)} className="text-primary">
          <Plus className="size-5" />
        </Button>
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
              <Button
                variant="ghost"
                size="icon"
                onClick={() => remove.mutate(t.id)}
                className="text-destructive"
              >
                <Trash2 className="size-4" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <SetupInstructions />

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

function SetupInstructions() {
  const { t } = useTranslation()
  const baseUrl = API_URL || window.location.origin

  return (
    <div className="space-y-2">
      <p className="px-1 text-xs font-medium text-muted-foreground">{t("webhooks.setup")}</p>

      <Card size="sm">
        <CardContent className="space-y-2">
          <p className="text-sm font-medium">{t("webhooks.jellyfin")}</p>
          <div className="rounded-lg bg-muted p-2">
            <p className="text-[10px] text-muted-foreground">{t("webhooks.webhookUrl")}</p>
            <code className="break-all text-xs">{baseUrl}/api/v1/webhooks/jellyfin</code>
          </div>
          <details className="text-xs text-muted-foreground">
            <summary className="cursor-pointer font-medium text-foreground">{t("webhooks.setupSteps")}</summary>
            <ol className="mt-2 list-inside list-decimal space-y-1 pl-1">
              <li>{t("webhooks.jellyfinStep1")}</li>
              <li>{t("webhooks.jellyfinStep2")}</li>
              <li>{t("webhooks.jellyfinStep3")}</li>
              <li>{t("webhooks.jellyfinStep4")}</li>
              <li>{t("webhooks.jellyfinStep5")}</li>
              <li>{t("webhooks.jellyfinStep6")}</li>
            </ol>
          </details>
        </CardContent>
      </Card>

      <Card size="sm">
        <CardContent className="space-y-2">
          <p className="text-sm font-medium">{t("webhooks.plex")}</p>
          <div className="rounded-lg bg-muted p-2">
            <p className="text-[10px] text-muted-foreground">{t("webhooks.webhookUrl")}</p>
            <code className="break-all text-xs">{baseUrl}/api/v1/webhooks/plex?token=YOUR_TOKEN</code>
          </div>
          <details className="text-xs text-muted-foreground">
            <summary className="cursor-pointer font-medium text-foreground">{t("webhooks.setupSteps")}</summary>
            <ol className="mt-2 list-inside list-decimal space-y-1 pl-1">
              <li>{t("webhooks.plexStep1")}</li>
              <li>{t("webhooks.plexStep2")}</li>
              <li>{t("webhooks.plexStep3")}</li>
            </ol>
          </details>
        </CardContent>
      </Card>
    </div>
  )
}
