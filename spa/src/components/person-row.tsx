import { Link } from "@tanstack/react-router"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Card, CardContent } from "@/components/ui/card"
import { posterUrl } from "@/lib/api/client"

type PersonRowProps = {
  id: string
  name: string
  subtitle?: string
  imagePath?: string
}

export function PersonRow({ id, name, subtitle, imagePath }: PersonRowProps) {
  return (
    <Link to="/people/$id" params={{ id }} className="block transition-colors active:bg-muted/50">
      <Card size="sm">
        <CardContent className="flex items-center gap-3">
          <Avatar>
            {imagePath && <AvatarImage src={posterUrl(imagePath)} />}
            <AvatarFallback>{name[0]?.toUpperCase()}</AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold">{name}</p>
            {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
          </div>
        </CardContent>
      </Card>
    </Link>
  )
}
