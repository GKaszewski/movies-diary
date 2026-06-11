import { createFileRoute, Link } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, Upload } from "lucide-react"
import { Button } from "@/components/ui/button"
import { API_URL } from "@/lib/api/client"
import { getToken } from "@/lib/auth"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/settings/export")({
  component: ExportPage,
})

function ExportPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("settings.export"))
  const [exporting, setExporting] = useState<string | null>(null)

  async function handleExport(format: "csv" | "json") {
    setExporting(format)
    try {
      const res = await fetch(`${API_URL}/api/v1/diary/export?format=${format}`, {
        headers: { Authorization: `Bearer ${getToken()}` },
      })
      const blob = await res.blob()
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `diary.${format}`
      a.click()
      URL.revokeObjectURL(url)
    } finally {
      setExporting(null)
    }
  }

  return (
    <div className="space-y-6 p-4">
      <div className="flex items-center gap-3">
        <Link to="/settings" className="text-muted-foreground">
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-lg font-bold">{t("settings.export")}</h1>
      </div>

      <div className="divide-y divide-border rounded-xl bg-card">
        <div className="flex items-center gap-3 p-3">
          <span className="text-muted-foreground">
            <Upload className="size-4" />
          </span>
          <div className="flex-1">
            <p className="text-sm font-medium">{t("settings.exportCsv")}</p>
            <p className="text-xs text-muted-foreground">
              {t("settings.exportDesc")}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleExport("csv")}
            disabled={exporting !== null}
          >
            {exporting === "csv" ? t("settings.exporting") : t("common.run")}
          </Button>
        </div>
        <div className="flex items-center gap-3 p-3">
          <span className="text-muted-foreground">
            <Upload className="size-4" />
          </span>
          <div className="flex-1">
            <p className="text-sm font-medium">{t("settings.exportJson")}</p>
            <p className="text-xs text-muted-foreground">
              {t("settings.exportDesc")}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleExport("json")}
            disabled={exporting !== null}
          >
            {exporting === "json" ? t("settings.exporting") : t("common.run")}
          </Button>
        </div>
      </div>
    </div>
  )
}
