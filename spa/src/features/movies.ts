import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { Paginated } from "@/lib/api/common"
import { movieDtoSchema, paginatedSchema, reviewDtoSchema } from "@/lib/api/common"
import { get, post } from "@/lib/api/client"

export const moviesQueryParamsSchema = z.object({
  limit: z.number().optional(),
  offset: z.number().optional(),
  search: z.string().optional(),
  genre: z.string().optional(),
  language: z.string().optional(),
})
export type MoviesQueryParams = z.infer<typeof moviesQueryParamsSchema>

export const moviesResponseSchema = paginatedSchema(movieDtoSchema)
export type MoviesResponse = z.infer<typeof moviesResponseSchema>

export const movieStatsDtoSchema = z.object({
  total_count: z.number(),
  avg_rating: z.number().optional(),
  federated_count: z.number(),
  rating_histogram: z.array(z.number()),
})
export type MovieStatsDto = z.infer<typeof movieStatsDtoSchema>

export const socialReviewDtoSchema = z.object({
  user_display: z.string(),
  rating: z.number(),
  comment: z.string().optional(),
  watched_at: z.string(),
  is_federated: z.boolean(),
  watch_medium: z.string().optional(),
})
export type SocialReviewDto = z.infer<typeof socialReviewDtoSchema>

export const socialFeedResponseSchema = paginatedSchema(socialReviewDtoSchema)
export type SocialFeedResponse = Paginated<SocialReviewDto>

export const movieDetailResponseSchema = z.object({
  movie: movieDtoSchema,
  stats: movieStatsDtoSchema,
  reviews: socialFeedResponseSchema,
})
export type MovieDetailResponse = z.infer<typeof movieDetailResponseSchema>

export const reviewHistoryResponseSchema = z.object({
  movie: movieDtoSchema,
  viewings: z.array(reviewDtoSchema),
  trend: z.string(),
})
export type ReviewHistoryResponse = z.infer<typeof reviewHistoryResponseSchema>

export const genreDtoSchema = z.object({
  tmdb_id: z.number(),
  name: z.string(),
})

export const keywordDtoSchema = z.object({
  tmdb_id: z.number(),
  name: z.string(),
})

export const castMemberDtoSchema = z.object({
  person_id: z.string(),
  tmdb_person_id: z.number(),
  name: z.string(),
  character: z.string(),
  billing_order: z.number(),
  profile_path: z.string().optional(),
})
export type CastMemberDto = z.infer<typeof castMemberDtoSchema>

export const crewMemberDtoSchema = z.object({
  person_id: z.string(),
  tmdb_person_id: z.number(),
  name: z.string(),
  job: z.string(),
  department: z.string(),
  profile_path: z.string().optional(),
})
export type CrewMemberDto = z.infer<typeof crewMemberDtoSchema>

export const movieProfileResponseSchema = z.object({
  tmdb_id: z.number(),
  imdb_id: z.string().optional(),
  overview: z.string().optional(),
  tagline: z.string().optional(),
  runtime_minutes: z.number().optional(),
  budget_usd: z.number().optional(),
  revenue_usd: z.number().optional(),
  vote_average: z.number().optional(),
  vote_count: z.number().optional(),
  original_language: z.string().optional(),
  collection_name: z.string().optional(),
  genres: z.array(genreDtoSchema),
  keywords: z.array(keywordDtoSchema),
  cast: z.array(castMemberDtoSchema),
  crew: z.array(crewMemberDtoSchema),
  enriched_at: z.string(),
})
export type MovieProfileResponse = z.infer<typeof movieProfileResponseSchema>

function getMovies(params?: MoviesQueryParams) {
  return get<MoviesResponse>("/movies", params)
}

function getMovie(id: string) {
  return get<MovieDetailResponse>(`/movies/${id}`)
}

function getMovieHistory(id: string) {
  return get<ReviewHistoryResponse>(`/movies/${id}/history`)
}

function getMovieProfile(id: string) {
  return get<MovieProfileResponse>(`/movies/${id}/profile`)
}

function syncPoster(id: string) {
  return post(`/movies/${id}/sync-poster`)
}

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
