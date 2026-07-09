import { useRouter } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { ArrowLeft } from "lucide-react"
import { Button } from "@/components/ui/button"

export function BackButton() {
  const { t } = useTranslation()
  const router = useRouter()

  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => router.history.back()}
      className="gap-1 text-muted-foreground"
    >
      <ArrowLeft className="size-4" /> {t("common.back")}
    </Button>
  )
}
