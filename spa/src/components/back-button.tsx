import { useRouter } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ArrowLeft } from "lucide-react"

export function BackButton() {
  const { t } = useTranslation()
  const router = useRouter()

  return (
    <button
      onClick={() => router.history.back()}
      className="inline-flex items-center gap-1 text-sm text-muted-foreground"
    >
      <ArrowLeft className="size-4" /> {t("common.back")}
    </button>
  )
}
