import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { ApiError } from "@/lib/api/client"

function deletionErrorKey(err: unknown): string {
  if (err instanceof ApiError) {
    switch (err.status) {
      case 401:
        return "deleteUser.errors.wrongPassword"
      case 403:
        return "deleteUser.errors.forbidden"
      case 404:
        return "deleteUser.errors.notFound"
      case 409:
        return "deleteUser.errors.lastAdmin"
    }
  }
  return "deleteUser.errors.generic"
}

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  passwordLabel: string
  confirmLabel: string
  pending: boolean
  error: unknown
  onSubmit: (password: string) => void
}

export function DeleteAccountDialog({
  open,
  onOpenChange,
  title,
  description,
  passwordLabel,
  confirmLabel,
  pending,
  error,
  onSubmit,
}: Props) {
  const { t } = useTranslation()
  const [password, setPassword] = useState("")

  const handleOpenChange = (next: boolean) => {
    if (!next) setPassword("")
    onOpenChange(next)
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (password) onSubmit(password)
          }}
        >
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>{description}</DialogDescription>
          </DialogHeader>
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground" htmlFor="delete-password">
              {passwordLabel}
            </label>
            <Input
              id="delete-password"
              type="password"
              autoFocus
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={pending}
            />
            {error != null && (
              <p role="alert" className="text-xs text-red-400">
                {t(deletionErrorKey(error))}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => handleOpenChange(false)} disabled={pending}>
              {t("common.cancel")}
            </Button>
            <Button
              type="submit"
              disabled={!password || pending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {confirmLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
