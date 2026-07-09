import { useTranslation } from "react-i18next"
import { Pencil } from "lucide-react"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"

type EditableContextMenuProps = {
  onEdit: () => void
  children: React.ReactNode
}

export function EditableContextMenu({ onEdit, children }: EditableContextMenuProps) {
  const { t } = useTranslation()

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div>{children}</div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={onEdit}>
          <Pencil className="mr-2 size-4" />
          {t("editReview.title")}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
