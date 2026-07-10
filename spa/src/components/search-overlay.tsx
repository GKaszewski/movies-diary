import { useState } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { PenLine, Search, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import { useSearch } from "@/features/search"
import { useDebounce } from "@/hooks/use-debounce"
import { posterUrl } from "@/lib/api/client"

export type MovieSelection = {
  id: string
  title: string
  release_year: number
  director?: string
  poster_path?: string
  genres: string[]
  external_metadata_id?: string
}

type SearchOverlayProps = {
  open: boolean
  onClose: () => void
  onSelect: (movie: MovieSelection) => void
}

const IMDB_RE = /^tt\d{4,}$/i

export function SearchOverlay({ open, onClose, onSelect }: SearchOverlayProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState("")
  const [manual, setManual] = useState(false)
  const [manualTitle, setManualTitle] = useState("")
  const [manualYear, setManualYear] = useState("")
  const [manualDirector, setManualDirector] = useState("")
  const [manualImdbId, setManualImdbId] = useState("")
  const debouncedQuery = useDebounce(query, 300)
  const { data, isPending } = useSearch({ q: debouncedQuery || undefined })

  function handleQueryChange(value: string) {
    setQuery(value)
    if (IMDB_RE.test(value.trim())) {
      setManualImdbId(value.trim())
      setManual(true)
      setQuery("")
    }
  }

  if (!open) return null

  const hasImdbId = manualImdbId.trim().length > 0
  const hasTitleYear = manualTitle.trim().length > 0 && manualYear.trim().length > 0
  const canSubmitManual = hasImdbId || hasTitleYear

  function handleManualSubmit() {
    if (!canSubmitManual) return
    onSelect({
      id: "",
      title: manualTitle || manualImdbId,
      release_year: manualYear ? parseInt(manualYear, 10) : 0,
      director: manualDirector || undefined,
      external_metadata_id: manualImdbId || undefined,
      genres: [],
    })
  }

  const hasResults = (data?.movies?.items?.length ?? 0) > 0
  const searched = debouncedQuery.length > 0 && !isPending

  const content = manual ? (
    <div className="fixed inset-0 z-50 flex flex-col glass-heavy">
      <div className="flex items-center justify-between p-4">
        <Button variant="ghost" size="sm" onClick={() => setManual(false)}>
          {t("searchOverlay.backToSearch")}
        </Button>
        <Button variant="ghost" size="sm" onClick={onClose}>
          {t("common.cancel")}
        </Button>
      </div>
      <div className="flex-1 overflow-auto px-4">
        <Card>
          <CardHeader>
            <CardTitle>{t("searchOverlay.addManuallyTitle")}</CardTitle>
            <CardDescription>{t("searchOverlay.addManuallyDesc")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="manual-imdb">{t("searchOverlay.imdbId")}</Label>
              <Input id="manual-imdb" value={manualImdbId} onChange={(e) => setManualImdbId(e.target.value)} placeholder={t("searchOverlay.imdbPlaceholder")} autoFocus />
              <p className="text-xs text-muted-foreground">{t("searchOverlay.imdbHelp")}</p>
            </div>
            <Separator />
            <p className="text-xs text-muted-foreground">{t("searchOverlay.orSearchByTitle")}</p>
            <div className="space-y-1.5">
              <Label htmlFor="manual-title">{t("searchOverlay.titleLabel")} {!hasImdbId && "*"}</Label>
              <Input id="manual-title" value={manualTitle} onChange={(e) => setManualTitle(e.target.value)} placeholder={t("searchOverlay.titlePlaceholder")} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="manual-year">{t("searchOverlay.releaseYear")} {!hasImdbId && "*"}</Label>
              <Input id="manual-year" type="number" value={manualYear} onChange={(e) => setManualYear(e.target.value)} placeholder={t("searchOverlay.yearPlaceholder")} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="manual-director">{t("searchOverlay.director")}</Label>
              <Input id="manual-director" value={manualDirector} onChange={(e) => setManualDirector(e.target.value)} placeholder={t("searchOverlay.directorPlaceholder")} />
            </div>
            <Button onClick={handleManualSubmit} disabled={!canSubmitManual} className="w-full">
              {t("common.continue")}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  ) : (
    <div className="fixed inset-0 z-50 flex flex-col glass-heavy">
      <div className="flex items-center gap-3 p-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input value={query} onChange={(e) => handleQueryChange(e.target.value)} placeholder={t("searchOverlay.searchPlaceholder")} className="pl-9" autoFocus />
          {query && (
            <Button variant="ghost" size="icon" onClick={() => setQuery("")} className="absolute right-3 top-1/2 size-6 -translate-y-1/2">
              <X className="size-4 text-muted-foreground" />
            </Button>
          )}
        </div>
        <Button variant="ghost" size="sm" onClick={onClose}>{t("common.cancel")}</Button>
      </div>

      <div className="flex-1 overflow-auto px-4">
        {isPending && debouncedQuery && (
          <div className="space-y-2">
            {[1, 2, 3].map((i) => (
              <div key={i} className="flex items-center gap-3 p-2">
                <Skeleton className="h-14 w-10 rounded-md" />
                <div className="space-y-1"><Skeleton className="h-4 w-28" /><Skeleton className="h-3 w-20" /></div>
              </div>
            ))}
          </div>
        )}

        {data?.movies?.items?.map((hit) => (
          <button
            key={hit.movie_id}
            onClick={() => {
              onSelect({
                id: hit.movie_id,
                title: hit.title,
                release_year: hit.release_year ?? 0,
                director: hit.director,
                poster_path: hit.poster_path,
                genres: hit.genres,
              })
              setQuery("")
            }}
            className="flex w-full items-center gap-3 rounded-lg p-2 text-left transition-colors active:bg-muted"
          >
            <div className="h-14 w-10 shrink-0 overflow-hidden rounded-md bg-muted">
              {hit.poster_path && <img src={posterUrl(hit.poster_path)} alt="" className="size-full object-cover" />}
            </div>
            <div>
              <p className="font-semibold">{hit.title}</p>
              <p className="text-xs text-muted-foreground">{hit.release_year}{hit.director && ` · ${hit.director}`}</p>
            </div>
          </button>
        ))}

        {searched && !hasResults && (
          <p className="py-6 text-center text-sm text-muted-foreground">{t("searchOverlay.noMoviesFound")}</p>
        )}

        {searched && (
          <Button
            variant="outline"
            className="mt-2 w-full justify-start gap-3 border-dashed"
            onClick={() => {
              setManualTitle(query)
              setManual(true)
            }}
          >
            <PenLine className="size-4" />
            <div className="text-left">
              <p className="text-sm font-medium">{t("searchOverlay.addManually")}</p>
              <p className="text-xs text-muted-foreground">{t("searchOverlay.addManuallySubtitle")}</p>
            </div>
          </Button>
        )}
      </div>
    </div>
  )

  return createPortal(content, document.body)
}
