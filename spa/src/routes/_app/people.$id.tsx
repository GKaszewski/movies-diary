import { createFileRoute } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import { Film, User } from "lucide-react"
import { BackButton } from "@/components/back-button"
import { MovieCard } from "@/components/movie-card"
import { EmptyState } from "@/components/empty-state"
import { Skeleton } from "@/components/ui/skeleton"
import { tmdbProfileUrl } from "@/lib/api/client"
import { usePersonCredits } from "@/hooks/use-search"
import { useDocumentTitle } from "@/hooks/use-document-title"

export const Route = createFileRoute("/_app/people/$id")({
  component: PersonDetailPage,
})

function PersonDetailPage() {
  const { t } = useTranslation()
  const { id } = Route.useParams()
  const { data, isPending } = usePersonCredits(id)

  if (isPending) return <PersonSkeleton />
  if (!data) return null

  const { person, cast, crew } = data
  useDocumentTitle(person.name)

  return (
    <div className="space-y-4 p-4">
      <BackButton />

      <div className="flex items-center gap-4">
        <div className="size-16 flex-shrink-0 overflow-hidden rounded-full bg-muted">
          {person.profile_path ? (
            <img src={tmdbProfileUrl(person.profile_path)} alt="" className="size-full object-cover" />
          ) : (
            <div className="flex size-full items-center justify-center">
              <User className="size-6 text-muted-foreground/40" />
            </div>
          )}
        </div>
        <div>
          <h1 className="text-xl font-bold">{person.name}</h1>
          {person.known_for_department && (
            <p className="text-sm text-muted-foreground">{person.known_for_department}</p>
          )}
        </div>
      </div>

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
