import { useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { Download, Share2, X } from "lucide-react"
import html2canvas from "html2canvas-pro"
import { Button } from "@/components/ui/button"
import { posterUrl } from "@/lib/api/client"
import type { WrapUpReport } from "@/features/wrapup"
const logoSrc = `${import.meta.env.BASE_URL}logo.webp`
const bgSrc = `${import.meta.env.BASE_URL}shareable_bg.jpg`

type Props = {
  report: WrapUpReport
  onClose: () => void
}

export function WrapUpShareCard({ report, onClose }: Props) {
  const { t } = useTranslation()
  const cardRef = useRef<HTMLDivElement>(null)
  const [exporting, setExporting] = useState(false)

  const watchHours = Math.round(report.total_watch_time_minutes / 60)
  const topGenre = report.top_genres[0]?.genre
  const topDirector = report.top_directors[0]?.name
  const topActor = report.top_actors[0]?.name
  const cols = 5
  const rows = 3
  const posters = report.poster_paths.slice(0, cols * rows)

  async function exportImage() {
    if (!cardRef.current) return
    setExporting(true)
    try {
      const canvas = await html2canvas(cardRef.current, {
        scale: 2,
        useCORS: true,
        backgroundColor: null,
      })
      const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, "image/png"))
      if (!blob) return

      const file = new File([blob], "wrapup.png", { type: "image/png" })
      if (navigator.share && navigator.canShare?.({ files: [file] })) {
        await navigator.share({ files: [file] })
      } else {
        const url = URL.createObjectURL(blob)
        const a = document.createElement("a")
        a.href = url
        a.download = "year-in-review.png"
        a.click()
        URL.revokeObjectURL(url)
      }
    } finally {
      setExporting(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-black/80 p-4">
      <div className="mb-4 flex w-full max-w-sm items-center justify-between">
        <Button variant="ghost" size="icon" onClick={onClose} className="text-white">
          <X className="size-5" />
        </Button>
        <Button onClick={exportImage} disabled={exporting} size="sm" className="gap-2">
          {"share" in navigator ? <Share2 className="size-4" /> : <Download className="size-4" />}
          {exporting ? t("common.saving") : t("wrapup.shareExport")}
        </Button>
      </div>

      <div className="max-h-[75vh] overflow-y-auto rounded-2xl">
        <div
          ref={cardRef}
          className="relative w-[360px] overflow-hidden rounded-2xl"
          style={{ aspectRatio: "9/16" }}
        >
          {/* Layer 1: background image */}
          <img src={bgSrc} alt="" className="absolute inset-0 size-full object-cover" />

          {/* Layer 2: poster collage */}
          <div className="absolute inset-0 grid gap-0.5 opacity-30" style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
            {posters.map((p, i) => (
              <div key={i} className="overflow-hidden">
                <img src={posterUrl(p)} alt="" className="size-full object-cover" />
              </div>
            ))}
          </div>

          {/* Layer 3: dark blur to make text readable */}
          <div className="absolute inset-0 bg-black/50 backdrop-blur-[2px]" />

          {/* Layer 4: content */}
          <div className="relative flex h-full flex-col p-6">
            {/* Header */}
            <div>
              <p className="text-sm font-semibold uppercase tracking-[0.15em] text-white drop-shadow">
                {t("wrapup.heroSubtitle")}
              </p>
              <p className="text-4xl font-black drop-shadow-lg" style={{ color: "oklch(0.852 0.199 91.936)" }}>
                {report.date_range.start.slice(0, 4)}
              </p>
            </div>

            {/* Center hero — grows to fill middle */}
            <div className="flex flex-1 flex-col items-center justify-center space-y-6">
              <div className="text-center">
                <p className="text-8xl font-black tracking-tight text-white drop-shadow-lg">{report.total_movies}</p>
                <p className="text-base font-medium text-white/80">{t("wrapup.moviesWatched")}</p>
              </div>

              <div className="flex gap-10">
                <div className="text-center">
                  <p className="text-3xl font-bold drop-shadow" style={{ color: "oklch(0.852 0.199 91.936)" }}>{report.avg_rating?.toFixed(1) ?? "-"}★</p>
                  <p className="text-xs text-white/60">{t("wrapup.averageRating")}</p>
                </div>
                <div className="text-center">
                  <p className="text-3xl font-bold text-white drop-shadow">{watchHours}h</p>
                  <p className="text-xs text-white/60">{t("wrapup.watchTime")}</p>
                </div>
              </div>
            </div>

            {/* Bottom stats */}
            <div className="space-y-2">
              {topGenre && <StatLine label={t("wrapup.topGenre")} value={topGenre} />}
              {topDirector && <StatLine label={t("wrapup.topDirectorLabel")} value={topDirector} />}
              {topActor && <StatLine label={t("wrapup.topActorLabel")} value={topActor} />}
              {report.busiest_month && <StatLine label={t("wrapup.busiestMonthLabel")} value={report.busiest_month} />}

              <div className="flex items-center justify-center gap-2 pt-3">
                <img src={logoSrc} alt="" className="size-5 rounded" />
                <p className="text-xs font-medium text-white/40">Movies Diary</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function StatLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between">
      <span className="text-[11px] text-white/40">{label}</span>
      <span className="max-w-[60%] truncate text-right text-sm font-semibold text-white">{value}</span>
    </div>
  )
}
