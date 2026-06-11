import { createFileRoute, Link } from "@tanstack/react-router"
import { useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, CheckCircle, Trash2, Upload } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import {
  useCreateImportSession,
  useApplyMapping,
  useApplyImportProfile,
  useConfirmImport,
  useImportPreview,
  useImportProfiles,
  useSaveImportProfile,
  useDeleteImportProfile,
} from "@/hooks/use-imports"
import { useDocumentTitle } from "@/hooks/use-document-title"
import type { SessionCreatedResponse } from "@/lib/api/imports"

export const Route = createFileRoute("/_app/settings/import")({
  component: ImportPage,
})

function ImportPage() {
  const { t } = useTranslation()
  useDocumentTitle(t("import.title"))

  const DOMAIN_FIELDS = [
    { value: "title", label: t("import.fieldTitle") },
    { value: "release_year", label: t("import.fieldReleaseYear") },
    { value: "director", label: t("import.fieldDirector") },
    { value: "rating", label: t("import.fieldRating") },
    { value: "watched_at", label: t("import.fieldWatchedAt") },
    { value: "comment", label: t("import.fieldComment") },
    { value: "external_metadata_id", label: t("import.fieldExternalId") },
    { value: "skip", label: t("import.fieldSkip") },
  ]

  const RATING_SCALES = [
    { value: "1", label: t("import.scale1to5") },
    { value: "0.5", label: t("import.scale1to10") },
    { value: "0.05", label: t("import.scale1to100") },
    { value: "1.25", label: t("import.scaleLetterboxd") },
  ]
  const [step, setStep] = useState(0)
  const [session, setSession] = useState<SessionCreatedResponse | null>(null)
  const [mappings, setMappings] = useState<Record<string, string>>({})
  const [ratingScale, setRatingScale] = useState("1")
  const [dateFormat, setDateFormat] = useState("")
  const fileRef = useRef<HTMLInputElement>(null)

  const createSession = useCreateImportSession()
  const applyMapping = useApplyMapping()
  const applyProfile = useApplyImportProfile()
  const confirmImport = useConfirmImport()
  const { data: profiles } = useImportProfiles()
  const deleteProfile = useDeleteImportProfile()

  const handleFile = (file: File) => {
    createSession.mutate(file, {
      onSuccess: (data) => {
        setSession(data)
        const initial: Record<string, string> = {}
        for (const col of data.columns) initial[col] = "skip"
        setMappings(initial)
        setStep(1)
      },
    })
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    const file = e.dataTransfer.files[0]
    if (file) handleFile(file)
  }

  const handleApplyMapping = () => {
    if (!session) return
    const mapped = Object.entries(mappings)
      .filter(([, v]) => v !== "skip")
      .map(([source_column, domain_field]) => ({
        source_column,
        domain_field,
        rating_scale: domain_field === "rating" && ratingScale !== "1"
          ? parseFloat(ratingScale)
          : undefined,
        date_format: domain_field === "watched_at" && dateFormat
          ? dateFormat
          : undefined,
      }))

    applyMapping.mutate(
      { sessionId: session.session_id, data: { mappings: mapped } },
      { onSuccess: () => setStep(2) },
    )
  }

  const handleConfirm = () => {
    if (!session) return
    const indices = Array.from({ length: session.sample_rows.length }, (_, i) => i)
    confirmImport.mutate(
      { sessionId: session.session_id, data: { confirmed_indices: indices } },
      { onSuccess: () => setStep(3) },
    )
  }

  const handleApplyProfile = (profileId: string) => {
    if (!session) return
    applyProfile.mutate(
      { sessionId: session.session_id, profileId },
      { onSuccess: () => setStep(2) },
    )
  }

  const hasRatingMapping = Object.values(mappings).includes("rating")

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center gap-3">
        {step === 0 || step === 3 ? (
          <Link to="/settings" className="text-muted-foreground">
            <ArrowLeft className="size-5" />
          </Link>
        ) : (
          <Button variant="ghost" size="icon" onClick={() => setStep((s) => s - 1)}>
            <ArrowLeft className="size-5" />
          </Button>
        )}
        <h1 className="text-lg font-bold">{t("import.title")}</h1>
        <span className="ml-auto text-xs text-muted-foreground">
          {step < 3 && t("import.step", { current: step + 1, total: 3 })}
        </span>
      </div>

      {/* Progress */}
      <div className="flex gap-1">
        {[0, 1, 2, 3].map((s) => (
          <div
            key={s}
            className={`h-1 flex-1 rounded-full ${s <= step ? "bg-primary" : "bg-muted"}`}
          />
        ))}
      </div>

      {/* Step 0: Upload */}
      {step === 0 && (
        <Card>
          <CardContent>
            <div
              onDrop={handleDrop}
              onDragOver={(e) => e.preventDefault()}
              onClick={() => fileRef.current?.click()}
              className="flex cursor-pointer flex-col items-center gap-3 rounded-xl border-2 border-dashed border-muted-foreground/30 p-10 text-center"
            >
              <Upload className="size-8 text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                {t("import.dropCsv")}
              </p>
              <input
                ref={fileRef}
                type="file"
                accept=".csv,.json"
                className="hidden"
                onChange={(e) => {
                  const file = e.target.files?.[0]
                  if (file) handleFile(file)
                }}
              />
              {createSession.isPending && (
                <p className="text-xs text-muted-foreground">{t("import.uploading")}</p>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 1: Mapping */}
      {step === 1 && session && (
        <div className="space-y-4">
          {/* Preview */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">{t("import.preview")}</CardTitle>
              <CardDescription>
                {t("import.rowsCols", { rows: session.sample_rows.length, cols: session.columns.length })}
              </CardDescription>
            </CardHeader>
            <CardContent className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    {session.columns.map((col) => (
                      <TableHead key={col} className="whitespace-nowrap text-xs">
                        {col}
                      </TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {session.sample_rows.slice(0, 3).map((row, i) => (
                    <TableRow key={i}>
                      {row.map((cell, j) => (
                        <TableCell key={j} className="max-w-32 truncate whitespace-nowrap text-xs">
                          {cell}
                        </TableCell>
                      ))}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {/* Presets */}
          {profiles && profiles.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm">{t("import.presets")}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {profiles.map((p) => (
                  <div key={p.id} className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      className="flex-1 justify-start"
                      onClick={() => handleApplyProfile(p.id)}
                      disabled={applyProfile.isPending}
                    >
                      {p.name}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-8 text-muted-foreground"
                      onClick={() => deleteProfile.mutate(p.id, {
                        onSuccess: () => toast.success(t("import.presetDeleted")),
                      })}
                    >
                      <Trash2 className="size-3.5" />
                    </Button>
                  </div>
                ))}
              </CardContent>
            </Card>
          )}

          {/* Column mapping */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">{t("import.columnMapping")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {session.columns.map((col) => (
                <div key={col} className="flex items-center gap-2">
                  <span className="min-w-24 truncate text-sm font-medium">{col}</span>
                  <Select
                    value={mappings[col] ?? "skip"}
                    onValueChange={(v) =>
                      setMappings((prev) => ({ ...prev, [col]: v }))
                    }
                  >
                    <SelectTrigger className="flex-1">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {DOMAIN_FIELDS.map((f) => (
                        <SelectItem key={f.value} value={f.value}>
                          {f.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              ))}
            </CardContent>
          </Card>

          {/* Rating scale */}
          {hasRatingMapping && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm">{t("import.ratingScale")}</CardTitle>
                <CardDescription>{t("import.ratingScaleDesc")}</CardDescription>
              </CardHeader>
              <CardContent>
                <Select value={ratingScale} onValueChange={setRatingScale}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {RATING_SCALES.map((s) => (
                      <SelectItem key={s.value} value={s.value}>
                        {s.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </CardContent>
            </Card>
          )}

          {/* Date format */}
          {Object.values(mappings).includes("watched_at") && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm">{t("import.dateFormat")}</CardTitle>
                <CardDescription>{t("import.dateFormatDesc")}</CardDescription>
              </CardHeader>
              <CardContent>
                <Input
                  value={dateFormat}
                  onChange={(e) => setDateFormat(e.target.value)}
                  placeholder={t("import.dateFormatPlaceholder")}
                />
              </CardContent>
            </Card>
          )}

          <Button
            onClick={handleApplyMapping}
            disabled={applyMapping.isPending}
            className="w-full"
          >
            {applyMapping.isPending ? t("import.applying") : t("common.continue")}
          </Button>
        </div>
      )}

      {/* Step 2: Confirm */}
      {step === 2 && session && (
        <ConfirmStep
          sessionId={session.session_id}
          onConfirm={handleConfirm}
          isPending={confirmImport.isPending}
        />
      )}

      {/* Step 3: Done */}
      {step === 3 && (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-8 text-center">
            <CheckCircle className="size-12 text-green-500" />
            <p className="font-medium">{t("import.importComplete")}</p>
            <Link to="/diary" className="text-sm text-primary underline">
              {t("import.viewDiary")}
            </Link>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function ConfirmStep({
  sessionId,
  onConfirm,
  isPending,
}: {
  sessionId: string
  onConfirm: () => void
  isPending: boolean
}) {
  const { t } = useTranslation()
  const { data, isPending: previewLoading } = useImportPreview(sessionId)
  const saveProfile = useSaveImportProfile()
  const [presetName, setPresetName] = useState("")

  if (previewLoading) return <Skeleton className="h-40 w-full rounded-xl" />

  const rows = data?.rows ?? []
  const valid = rows.filter((r) => r.status === "valid")
  const duplicates = rows.filter((r) => r.status === "duplicate")
  const invalid = rows.filter((r) => r.status === "invalid")

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("import.importSummary")}</CardTitle>
          <CardDescription>
            {t("import.summaryDesc", { valid: valid.length, duplicates: duplicates.length, invalid: invalid.length })}
          </CardDescription>
        </CardHeader>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("import.previewRows", { count: rows.length })}</CardTitle>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="text-xs">{t("import.status")}</TableHead>
                <TableHead className="text-xs">{t("import.fieldTitle")}</TableHead>
                <TableHead className="text-xs">{t("import.year")}</TableHead>
                <TableHead className="text-xs">{t("import.fieldDirector")}</TableHead>
                <TableHead className="text-xs">{t("import.fieldRating")}</TableHead>
                <TableHead className="text-xs">{t("import.watched")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((row) => (
                <TableRow key={row.index} className={row.status === "invalid" ? "text-destructive" : row.status === "duplicate" ? "opacity-50" : ""}>
                  <TableCell>
                    <Badge variant={row.status === "valid" ? "default" : row.status === "duplicate" ? "secondary" : "destructive"} className="text-[10px]">
                      {row.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="max-w-32 truncate text-xs">{row.title ?? row.errors?.join(", ")}</TableCell>
                  <TableCell className="text-xs">{row.release_year}</TableCell>
                  <TableCell className="max-w-24 truncate text-xs">{row.director}</TableCell>
                  <TableCell className="text-xs">{row.rating}</TableCell>
                  <TableCell className="text-xs">{row.watched_at}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">{t("import.savePreset")}</CardTitle>
        </CardHeader>
        <CardContent className="flex gap-2">
          <Input
            value={presetName}
            onChange={(e) => setPresetName(e.target.value)}
            placeholder={t("import.presetNamePlaceholder")}
            className="flex-1"
          />
          <Button
            variant="outline"
            size="sm"
            disabled={!presetName.trim() || saveProfile.isPending || saveProfile.isSuccess}
            onClick={() =>
              saveProfile.mutate(
                { session_id: sessionId, name: presetName.trim() },
                { onSuccess: () => toast.success(t("import.presetSaved")) },
              )
            }
          >
            {saveProfile.isSuccess ? t("import.presetSaved") : t("common.save")}
          </Button>
        </CardContent>
      </Card>

      <Button onClick={onConfirm} disabled={isPending || valid.length === 0} className="w-full">
        {isPending ? t("import.importing") : t("import.importRows", { count: valid.length })}
      </Button>
    </div>
  )
}
