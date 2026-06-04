import { useInfiniteQuery, useQuery } from "@tanstack/react-query"
import { getPerson, getPersonCredits, search } from "@/lib/api/search"
import type { SearchQueryParams } from "@/lib/api/search"

const PAGE_SIZE = 20

export const searchKeys = {
  all: ["search"] as const,
  query: (params: SearchQueryParams) => [...searchKeys.all, params] as const,
  person: (id: string) => ["people", id] as const,
  personCredits: (id: string) => ["people", id, "credits"] as const,
}

export function useSearch(params: SearchQueryParams) {
  return useQuery({
    queryKey: searchKeys.query(params),
    queryFn: () => search(params),
    enabled: !!params.q || !!params.genre || !!params.person_id,
  })
}

export function useInfiniteSearch(
  params: Omit<SearchQueryParams, "limit" | "offset">,
) {
  return useInfiniteQuery({
    queryKey: searchKeys.query(params),
    queryFn: ({ pageParam = 0 }) =>
      search({ ...params, limit: PAGE_SIZE, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last) => {
      const next = last.movies.offset + last.movies.limit
      return next < last.movies.total_count ? next : undefined
    },
    enabled: !!params.q || !!params.genre || !!params.person_id,
  })
}

export function usePerson(id: string) {
  return useQuery({
    queryKey: searchKeys.person(id),
    queryFn: () => getPerson(id),
    enabled: !!id,
  })
}

export function usePersonCredits(id: string) {
  return useQuery({
    queryKey: searchKeys.personCredits(id),
    queryFn: () => getPersonCredits(id),
    enabled: !!id,
  })
}
