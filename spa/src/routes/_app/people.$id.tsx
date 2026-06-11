import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Calendar, ChevronDown, ExternalLink, Film, Globe, MapPin, User } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { SwipeTabs } from "@/components/swipe-tabs"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import { tmdbProfileUrl } from "@/lib/api/client"
import { usePersonCredits } from "@/hooks/use-search"
import { useDocumentTitle } from "@/hooks/use-document-title"
import { shortDate } from "@/lib/date"
import { differenceInYears, parseISO } from "date-fns"

export const Route = createFileRoute("/_app/people/$id")({
  component: PersonDetailPage,
})

function PersonDetailPage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { data, isPending } = usePersonCredits(id)
  useDocumentTitle(data?.person.name)

  if (isPending) return <PersonSkeleton />
  if (!data) return null

  const { person, cast, crew } = data

  const age = person.birthday
    ? differenceInYears(
        person.deathday ? parseISO(person.deathday) : new Date(),
        parseISO(person.birthday),
      )
    : null

  const creditTabs = [
    ...(cast.length > 0 ? [{ value: "cast", label: t("movie.cast") + ` (${cast.length})` }] : []),
    ...(crew.length > 0 ? [{ value: "crew", label: t("movie.crew") + ` (${crew.length})` }] : []),
  ] as const

  return (
    <div className="space-y-4 p-4">
      <BackButton />

      {/* Header */}
      <div className="flex gap-4">
        <div className="size-24 flex-shrink-0 overflow-hidden rounded-xl bg-muted">
          {person.profile_path ? (
            <img src={tmdbProfileUrl(person.profile_path)} alt="" className="size-full object-cover" />
          ) : (
            <div className="flex size-full items-center justify-center">
              <User className="size-10 text-muted-foreground/40" />
            </div>
          )}
        </div>
        <div className="min-w-0 flex-1 space-y-1">
          <h1 className="text-xl font-bold">{person.name}</h1>
          {person.known_for_department && (
            <Badge variant="secondary">{person.known_for_department}</Badge>
          )}
          {(person.homepage || person.imdb_url) && (
            <div className="flex gap-1.5 pt-1">
              {person.imdb_url && (
                <a href={person.imdb_url} target="_blank" rel="noopener noreferrer">
                  <Badge variant="outline" className="gap-1 text-[10px]">
                    <ExternalLink className="size-2.5" />
                    IMDb
                  </Badge>
                </a>
              )}
              {person.homepage && (
                <a href={person.homepage} target="_blank" rel="noopener noreferrer">
                  <Badge variant="outline" className="gap-1 text-[10px]">
                    <Globe className="size-2.5" />
                    {t("person.homepage")}
                  </Badge>
                </a>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Details card */}
      {(person.birthday || person.place_of_birth) && (
        <Card size="sm">
          <CardContent className="space-y-2">
            {person.birthday && (
              <div className="flex items-center gap-2 text-sm">
                <Calendar className="size-3.5 text-muted-foreground" />
                <span>{shortDate(person.birthday)}</span>
                {age != null && (
                  <span className="text-muted-foreground">({age})</span>
                )}
              </div>
            )}
            {person.deathday && (
              <div className="flex items-center gap-2 text-sm">
                <Calendar className="size-3.5 text-muted-foreground" />
                <span>{shortDate(person.deathday)}</span>
                <span className="text-muted-foreground">({t("person.deathday").toLowerCase()})</span>
              </div>
            )}
            {person.place_of_birth && (
              <div className="flex items-center gap-2 text-sm">
                <MapPin className="size-3.5 text-muted-foreground" />
                <span>{person.place_of_birth}</span>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Biography */}
      {person.biography && <BiographySection text={person.biography} />}

      {/* Also known as */}
      {person.also_known_as?.length > 0 && (
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-xs">{t("person.alsoKnownAs")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-1">
              {person.also_known_as.map((name) => (
                <Badge key={name} variant="outline" className="text-xs font-normal">
                  {name}
                </Badge>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      <Separator />

      {/* Credits */}
      {creditTabs.length > 0 ? (
        <SwipeTabs tabs={creditTabs} defaultValue={creditTabs[0].value} tabsListClassName="w-full">
          {(tab) => (
            <>
              {tab === "cast" && (
                <div className="space-y-1">
                  {cast.map((c) => (
                    <MovieCard
                      key={`${c.movie_id}-${c.character}`}
                      movie={{
                        id: c.movie_id,
                        title: c.title,
                        release_year: c.release_year ?? 0,
                        poster_path: c.poster_path,
                        genres: [],
                      }}
                      subtitle={c.character}
                      variant="compact"
                    />
                  ))}
                </div>
              )}
              {tab === "crew" && (
                <div className="space-y-1">
                  {crew.map((c) => (
                    <MovieCard
                      key={`${c.movie_id}-${c.job}`}
                      movie={{
                        id: c.movie_id,
                        title: c.title,
                        release_year: c.release_year ?? 0,
                        poster_path: c.poster_path,
                        genres: [],
                      }}
                      subtitle={`${c.job} (${c.department})`}
                      variant="compact"
                    />
                  ))}
                </div>
              )}
            </>
          )}
        </SwipeTabs>
      ) : (
        <EmptyState icon={Film} title={t("movie.noCredits")} />
      )}
    </div>
  )
}

const BIO_COLLAPSE_THRESHOLD = 300

function BiographySection({ text }: { text: string }) {
  const { t } = useTranslation()
  const isLong = text.length > BIO_COLLAPSE_THRESHOLD
  const [expanded, setExpanded] = useState(!isLong)

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-xs">{t("person.biography")}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className={`text-sm leading-relaxed text-muted-foreground ${!expanded ? "line-clamp-4" : ""}`}>
          {text}
        </p>
        {isLong && (
          <Button
            variant="ghost"
            size="sm"
            className="mt-1 h-auto p-0 text-xs text-primary"
            onClick={() => setExpanded((v) => !v)}
          >
            <ChevronDown className={`mr-1 size-3 transition-transform ${expanded ? "rotate-180" : ""}`} />
            {expanded ? t("common.less") : t("common.more")}
          </Button>
        )}
      </CardContent>
    </Card>
  )
}

function PersonSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <Skeleton className="h-5 w-16" />
      <div className="flex gap-4">
        <Skeleton className="size-24 rounded-xl" />
        <div className="flex-1 space-y-2">
          <Skeleton className="h-6 w-32" />
          <Skeleton className="h-5 w-20 rounded-full" />
          <Skeleton className="h-4 w-24" />
        </div>
      </div>
      <Skeleton className="h-20 rounded-xl" />
      <Skeleton className="h-32 rounded-xl" />
      <Skeleton className="h-1 w-full" />
      <div className="space-y-2">
        {[1, 2, 3, 4].map((i) => (
          <Skeleton key={i} className="h-10 rounded-lg" />
        ))}
      </div>
    </div>
  )
}
