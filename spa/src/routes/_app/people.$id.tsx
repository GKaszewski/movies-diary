import { createFileRoute } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { Calendar, ExternalLink, Film, Globe, MapPin, User } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { Badge } from "@/components/ui/badge"
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

  return (
    <div className="space-y-4 p-4">
      <BackButton />

      {/* Header */}
      <div className="flex gap-4">
        <div className="size-20 flex-shrink-0 overflow-hidden rounded-xl bg-muted">
          {person.profile_path ? (
            <img src={tmdbProfileUrl(person.profile_path)} alt="" className="size-full object-cover" />
          ) : (
            <div className="flex size-full items-center justify-center">
              <User className="size-8 text-muted-foreground/40" />
            </div>
          )}
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="text-xl font-bold">{person.name}</h1>
          {person.known_for_department && (
            <p className="text-sm text-muted-foreground">{person.known_for_department}</p>
          )}
          {person.birthday && (
            <p className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
              <Calendar className="size-3" />
              {shortDate(person.birthday)}
              {person.deathday && ` — ${shortDate(person.deathday)}`}
              {` (${differenceInYears(person.deathday ? parseISO(person.deathday) : new Date(), parseISO(person.birthday))})`}
            </p>
          )}
          {person.place_of_birth && (
            <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <MapPin className="size-3" />
              {person.place_of_birth}
            </p>
          )}
        </div>
      </div>

      {/* Links */}
      {(person.homepage || person.imdb_url) && (
        <div className="flex gap-2">
          {person.imdb_url && (
            <a href={person.imdb_url} target="_blank" rel="noopener noreferrer">
              <Badge variant="secondary" className="gap-1">
                <ExternalLink className="size-3" />
                {t("person.imdb")}
              </Badge>
            </a>
          )}
          {person.homepage && (
            <a href={person.homepage} target="_blank" rel="noopener noreferrer">
              <Badge variant="secondary" className="gap-1">
                <Globe className="size-3" />
                {t("person.homepage")}
              </Badge>
            </a>
          )}
        </div>
      )}

      {/* Biography */}
      {person.biography && (
        <p className="text-sm leading-relaxed text-muted-foreground">{person.biography}</p>
      )}

      {/* Also known as */}
      {person.also_known_as?.length > 0 && (
        <div>
          <p className="mb-1 text-xs font-medium text-muted-foreground">{t("person.alsoKnownAs")}</p>
          <div className="flex flex-wrap gap-1">
            {person.also_known_as.map((name) => (
              <Badge key={name} variant="outline" className="text-xs font-normal">
                {name}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Cast credits */}
      {cast.length > 0 && (
        <section>
          <h2 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("movie.castCredits", { count: cast.length })}
          </h2>
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
        </section>
      )}

      {/* Crew credits */}
      {crew.length > 0 && (
        <section>
          <h2 className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("movie.crewCredits", { count: crew.length })}
          </h2>
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
        </section>
      )}

      {cast.length === 0 && crew.length === 0 && (
        <EmptyState icon={Film} title={t("movie.noCredits")} />
      )}
    </div>
  )
}

function PersonSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <Skeleton className="h-5 w-16" />
      <div className="flex items-center gap-4">
        <Skeleton className="size-16 rounded-full" />
        <div className="space-y-2">
          <Skeleton className="h-6 w-32" />
          <Skeleton className="h-4 w-20" />
        </div>
      </div>
      <div className="space-y-2">
        {[1, 2, 3, 4].map((i) => (
          <Skeleton key={i} className="h-10 rounded-lg" />
        ))}
      </div>
    </div>
  )
}
