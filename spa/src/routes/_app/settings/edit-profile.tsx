import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowLeft, Camera, Plus, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Separator } from "@/components/ui/separator"
import { Textarea } from "@/components/ui/textarea"
import { Skeleton } from "@/components/ui/skeleton"
import { useProfile, useUpdateProfile, useUpdateProfileFields } from "@/hooks/use-users"

export const Route = createFileRoute("/_app/settings/edit-profile")({
  component: EditProfilePage,
})

function EditProfilePage() {
  const { t } = useTranslation()
  const { data, isPending } = useProfile()
  const update = useUpdateProfile()
  const updateFields = useUpdateProfileFields()
  const navigate = useNavigate()

  const [displayName, setDisplayName] = useState("")
  const [bio, setBio] = useState("")
  const [alsoKnownAs, setAlsoKnownAs] = useState("")
  const [fields, setFields] = useState<{ name: string; value: string }[]>([])
  const [avatarFile, setAvatarFile] = useState<File | null>(null)
  const [bannerFile, setBannerFile] = useState<File | null>(null)
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null)
  const [bannerPreview, setBannerPreview] = useState<string | null>(null)
  const avatarInputRef = useRef<HTMLInputElement>(null)
  const bannerInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (data) {
      setDisplayName(data.display_name ?? "")
      setBio(data.bio ?? "")
      setAlsoKnownAs(data.also_known_as ?? "")
      setFields(data.fields?.length ? data.fields.map((f) => ({ ...f })) : [])
    }
  }, [data])

  function handleFileSelect(
    e: React.ChangeEvent<HTMLInputElement>,
    setFile: (f: File | null) => void,
    setPreview: (url: string | null) => void,
  ) {
    const file = e.target.files?.[0]
    if (!file) return
    setFile(file)
    setPreview(URL.createObjectURL(file))
  }

  function updateField(index: number, key: "name" | "value", val: string) {
    setFields((prev) => prev.map((f, i) => (i === index ? { ...f, [key]: val } : f)))
  }

  function removeField(index: number) {
    setFields((prev) => prev.filter((_, i) => i !== index))
  }

  function addField() {
    setFields((prev) => [...prev, { name: "", value: "" }])
  }

  async function handleSave() {
    await update.mutateAsync({
      display_name: displayName,
      bio,
      also_known_as: alsoKnownAs,
      avatar: avatarFile ?? undefined,
      banner: bannerFile ?? undefined,
    })

    const validFields = fields.filter((f) => f.name.trim() && f.value.trim())
    if (validFields.length > 0 || (data?.fields?.length ?? 0) > 0) {
      await updateFields.mutateAsync({ fields: validFields })
    }

    navigate({ to: "/settings" })
  }

  if (isPending) {
    return (
      <div className="space-y-4 p-4">
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-32 w-full rounded-xl" />
        <Skeleton className="h-10 w-full rounded-lg" />
      </div>
    )
  }

  const currentAvatar = avatarPreview ?? data?.avatar_url
  const currentBanner = bannerPreview ?? data?.banner_url
  const saving = update.isPending || updateFields.isPending

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center gap-3">
        <Link to="/settings" className="text-muted-foreground">
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-lg font-bold">{t("editProfile.title")}</h1>
      </div>

      {/* Banner */}
      <div className="space-y-1.5">
        <Label>{t("editProfile.banner")}</Label>
        <button
          onClick={() => bannerInputRef.current?.click()}
          className="relative w-full overflow-hidden rounded-xl bg-muted"
          style={{ aspectRatio: "3/1" }}
        >
          {currentBanner ? (
            <img src={currentBanner} alt="" className="size-full object-cover" />
          ) : (
            <div className="flex size-full items-center justify-center text-muted-foreground">
              <Camera className="size-6" />
            </div>
          )}
          <div className="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 transition-opacity hover:opacity-100">
            <Camera className="size-6 text-white" />
          </div>
        </button>
        <input
          ref={bannerInputRef}
          type="file"
          accept="image/*"
          onChange={(e) => handleFileSelect(e, setBannerFile, setBannerPreview)}
          className="hidden"
        />
      </div>

      {/* Avatar */}
      <div className="space-y-1.5">
        <Label>{t("editProfile.avatar")}</Label>
        <button
          onClick={() => avatarInputRef.current?.click()}
          className="relative size-20 overflow-hidden rounded-full bg-muted"
        >
          {currentAvatar ? (
            <img src={currentAvatar} alt="" className="size-full object-cover" />
          ) : (
            <div className="flex size-full items-center justify-center text-2xl font-bold text-muted-foreground">
              {displayName.charAt(0).toUpperCase() || data?.username?.charAt(0).toUpperCase() || "?"}
            </div>
          )}
          <div className="absolute inset-0 flex items-center justify-center rounded-full bg-black/30 opacity-0 transition-opacity hover:opacity-100">
            <Camera className="size-5 text-white" />
          </div>
        </button>
        <input
          ref={avatarInputRef}
          type="file"
          accept="image/*"
          onChange={(e) => handleFileSelect(e, setAvatarFile, setAvatarPreview)}
          className="hidden"
        />
      </div>

      <div className="space-y-3">
        <div className="space-y-1.5">
          <Label htmlFor="display-name">{t("editProfile.displayName")}</Label>
          <Input
            id="display-name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder={t("editProfile.displayNamePlaceholder")}
          />
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="bio">{t("editProfile.bio")}</Label>
          <Textarea
            id="bio"
            value={bio}
            onChange={(e) => setBio(e.target.value)}
            placeholder={t("editProfile.bioPlaceholder")}
            rows={3}
          />
        </div>

        <Separator />

        {/* Federation fields */}
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("editProfile.federation")}</CardTitle>
            <CardDescription>{t("editProfile.federationDesc")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="also-known-as">{t("editProfile.alsoKnownAs")}</Label>
              <Input
                id="also-known-as"
                value={alsoKnownAs}
                onChange={(e) => setAlsoKnownAs(e.target.value)}
                placeholder={t("editProfile.alsoKnownAsPlaceholder")}
              />
              <p className="text-xs text-muted-foreground">{t("editProfile.alsoKnownAsHelp")}</p>
            </div>

            <Separator />

            <div className="space-y-1.5">
              <Label>{t("editProfile.profileFields")}</Label>
              <p className="text-xs text-muted-foreground">{t("editProfile.profileFieldsHelp")}</p>
            </div>

            {fields.map((field, i) => (
              <div key={i} className="flex items-start gap-2">
                <div className="flex-1 space-y-1">
                  <Input
                    value={field.name}
                    onChange={(e) => updateField(i, "name", e.target.value)}
                    placeholder={t("editProfile.label")}
                  />
                  <Input
                    value={field.value}
                    onChange={(e) => updateField(i, "value", e.target.value)}
                    placeholder={t("editProfile.value")}
                  />
                </div>
                <Button variant="ghost" size="icon" onClick={() => removeField(i)} className="mt-1 text-muted-foreground hover:text-destructive">
                  <X className="size-4" />
                </Button>
              </div>
            ))}

            <Button variant="outline" size="sm" onClick={addField} className="w-full">
              <Plus className="mr-1 size-4" />
              {t("editProfile.addField")}
            </Button>
          </CardContent>
        </Card>

        <Button onClick={handleSave} disabled={saving} className="w-full">
          {saving ? t("common.saving") : t("common.save")}
        </Button>
      </div>
    </div>
  )
}
