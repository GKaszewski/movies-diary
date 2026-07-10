import { Link } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { Users } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { tmdbProfileUrl } from "@/lib/api/client"
import type { PersonStat } from "@/features/wrapup"

export function RankCard({ title, subtitle, items, profilePaths }: { title: string; subtitle: string; items: PersonStat[]; profilePaths?: string[] }) {
  const { t } = useTranslation()
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <Users className="size-4" /> {title}
        </CardTitle>
        <CardDescription>{subtitle}</CardDescription>
      </CardHeader>
      <CardContent>
        <ol className="space-y-2">
          {items.map((item, i) => {
            const profilePath = profilePaths?.[i]
            const inner = (
              <>
                <span className="flex size-6 items-center justify-center rounded-full bg-muted text-xs font-bold">{i + 1}</span>
                <Avatar className="size-8">
                  {profilePath && <AvatarImage src={tmdbProfileUrl(profilePath)} />}
                  <AvatarFallback className="text-xs">{item.name[0]}</AvatarFallback>
                </Avatar>
                <div className="flex-1">
                  <p className="text-sm font-medium">{item.name}</p>
                  <p className="text-xs text-muted-foreground">{t("common.filmsAvg", { count: item.count, avg: item.avg_rating.toFixed(1) })}★</p>
                </div>
              </>
            )
            return (
              <li key={item.name}>
                {item.person_id ? (
                  <Link to="/people/$id" params={{ id: item.person_id }} className="flex items-center gap-3">{inner}</Link>
                ) : (
                  <div className="flex items-center gap-3">{inner}</div>
                )}
              </li>
            )
          })}
        </ol>
      </CardContent>
    </Card>
  )
}
