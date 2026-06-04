import { useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { useDrag } from "@use-gesture/react"
import { Trash2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/confirm-dialog"

type SwipeToDeleteProps = {
  onDelete: () => void
  confirmTitle?: string
  confirmDescription?: string
  children: React.ReactNode
}

export function SwipeToDelete({
  onDelete,
  confirmTitle,
  confirmDescription,
  children,
}: SwipeToDeleteProps) {
  const { t } = useTranslation()
  const [offsetX, setOffsetX] = useState(0)
  const [revealed, setRevealed] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  const bind = useDrag(
    ({ movement: [mx], active, event }) => {
      if (active) {
        event.stopPropagation()
        setOffsetX(Math.min(0, Math.max(mx, -100)))
      } else {
        if (mx < -60) {
          setRevealed(true)
          setOffsetX(-80)
        } else {
          setRevealed(false)
          setOffsetX(0)
        }
      }
    },
    { axis: "x", filterTaps: true, pointer: { capture: true } },
  )

  function handleDeleteTap() {
    setConfirmOpen(true)
  }

  function handleConfirm() {
    setConfirmOpen(false)
    setOffsetX(-300)
    setTimeout(() => {
      onDelete()
      setRevealed(false)
      setOffsetX(0)
    }, 200)
  }

  function handleTapContent(e: React.MouseEvent) {
    if (revealed) {
      e.preventDefault()
      e.stopPropagation()
      setRevealed(false)
      setOffsetX(0)
    }
  }

  return (
    <div className="relative overflow-hidden rounded-xl">
      <div className={`absolute inset-y-0 right-0 flex w-20 items-center justify-center bg-destructive transition-opacity ${offsetX < 0 ? "opacity-100" : "opacity-0"}`}>
        <Button variant="ghost" size="icon" onClick={handleDeleteTap} className="text-destructive-foreground hover:text-destructive-foreground">
          <Trash2 className="size-5" />
        </Button>
      </div>

      <div
        ref={ref}
        {...bind()}
        onClickCapture={handleTapContent}
        className="relative touch-pan-y"
        style={{
          transform: `translateX(${offsetX}px)`,
          transition: offsetX === 0 || Math.abs(offsetX) === 80 || offsetX === -300 ? "transform 200ms ease-out" : "none",
        }}
      >
        {children}
      </div>

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={confirmTitle ?? t("swipeToDelete.removeItem")}
        description={confirmDescription}
        confirmLabel={t("common.delete")}
        onConfirm={handleConfirm}
      />
    </div>
  )
}
