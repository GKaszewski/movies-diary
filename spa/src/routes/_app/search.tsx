import { createFileRoute } from "@tanstack/react-router"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { Bookmark, Search as SearchIcon, Film, Users } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { MovieCard } from "@/components/movie-card"
import { PersonRow } from "@/components/person-row"
import { EmptyState } from "@/components/empty-state"
import { InfiniteScroll } from "@/components/infinite-scroll"
import { Skeleton } from "@/components/ui/skeleton"
import { useInfiniteSearch } from "@/hooks/use-search"
import { useDebounce } from "@/hooks/use-debounce"
import { useAddToWatchlist } from "@/hooks/use-watchlist"
import { toast } from "sonner"

export const Route = createFileRoute("/_app/search")({
  component: SearchPage,
})

function SearchPage() {
  const { t } = useTranslation()
  const addToWatchlist = useAddToWatchlist()
  const [query, setQuery] = useState("")
  const debouncedQuery = useDebounce(query, 300)
  const {
    data,
    isPending,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
  } = useInfiniteSearch({ q: debouncedQuery || undefined })
  const hasQuery = debouncedQuery.length > 0

  const movies = data?.pages.flatMap((p) => p.movies.items) ?? []
  const people = data?.pages[0]?.people.items ?? []
  const loadMore = useCallback(() => fetchNextPage(), [fetchNextPage])

  return (
    <div className="space-y-4 p-4">
      <div className="relative">
        <SearchIcon className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("search.placeholder")}
          className="pl-9"
          autoFocus
        />
      </div>

      {!hasQuery && <EmptyState icon={SearchIcon} title={t("search.searchPrompt")} />}
      {hasQuery && isPending && <SearchSkeleton />}

      {hasQuery && data && (
        <div className="space-y-6">
          {people.length > 0 && (
            <section>
              <h2 className="mb-2 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                <Users className="size-3.5" /> {t("search.people")}
              </h2>
              <div className="space-y-1">
                {people.map((person) => (
                  <PersonRow
                    key={person.person_id}
                    id={person.person_id}
                    name={person.name}
                    subtitle={person.known_for_department}
                    imagePath={person.profile_path}
                  />
                ))}
              </div>
            </section>
          )}

          {movies.length > 0 && (
            <section>
              <h2 className="mb-2 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                <Film className="size-3.5" /> {t("search.movies")}
              </h2>
              <div className="space-y-2">
                {movies.map((hit) => (
                  <MovieCard
                    key={hit.movie_id}
                    movie={{
                      id: hit.movie_id,
                      title: hit.title,
                      release_year: hit.release_year ?? 0,
                      director: hit.director,
                      poster_path: hit.poster_path,
                      genres: hit.genres,
                    }}
                    variant="full"
                    action={
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-8 text-muted-foreground"
                        onClick={() => {
                          addToWatchlist.mutate(
                            { movie_id: hit.movie_id },
                            { onSuccess: () => toast.success(t("feed.addedToWatchlist")) },
                          )
                        }}
                      >
                        <Bookmark className="size-4" />
                      </Button>
                    }
                  />
                ))}
              </div>
              <InfiniteScroll
                hasMore={!!hasNextPage}
                isFetching={isFetchingNextPage}
                onLoadMore={loadMore}
              />
            </section>
          )}

          {movies.length === 0 && people.length === 0 && (
            <EmptyState icon={SearchIcon} title={t("search.noResults")} description={t("search.noResultsDesc")} />
          )}
        </div>
      )}
    </div>
  )
}

function SearchSkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3, 4].map((i) => (
        <div key={i} className="flex gap-3 rounded-xl bg-card p-3">
          <Skeleton className="h-[84px] w-14 rounded-lg" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-3 w-24" />
          </div>
        </div>
      ))}
    </div>
  )
}
