import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  getMovie,
  getMovieHistory,
  getMovieProfile,
  getMovies,
  syncPoster,
} from "@/lib/api/movies"
import type { MoviesQueryParams } from "@/lib/api/movies"

export const movieKeys = {
  all: ["movies"] as const,
  list: (params?: MoviesQueryParams) => [...movieKeys.all, params] as const,
  detail: (id: string) => [...movieKeys.all, id] as const,
  history: (id: string) => [...movieKeys.all, id, "history"] as const,
  profile: (id: string) => [...movieKeys.all, id, "profile"] as const,
}

export function useMovies(params?: MoviesQueryParams) {
  return useQuery({
    queryKey: movieKeys.list(params),
    queryFn: () => getMovies(params),
  })
}

export function useMovie(id: string) {
  return useQuery({
    queryKey: movieKeys.detail(id),
    queryFn: () => getMovie(id),
    enabled: !!id,
  })
}

export function useMovieHistory(id: string) {
  return useQuery({
    queryKey: movieKeys.history(id),
    queryFn: () => getMovieHistory(id),
    enabled: !!id,
  })
}

export function useMovieProfile(id: string) {
  return useQuery({
    queryKey: movieKeys.profile(id),
    queryFn: () => getMovieProfile(id),
    enabled: !!id,
  })
}

export function useSyncPoster() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => syncPoster(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: movieKeys.all })
    },
  })
}
