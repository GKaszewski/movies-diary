import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { del, get, post } from "@/lib/api/client"

export const generateWrapUpRequestSchema = z.object({
  start_date: z.string(),
  end_date: z.string(),
  user_id: z.string().uuid().optional(),
})
export type GenerateWrapUpRequest = z.infer<typeof generateWrapUpRequestSchema>

export const wrapUpGeneratedResponseSchema = z.object({
  id: z.string(),
})
export type WrapUpGeneratedResponse = z.infer<typeof wrapUpGeneratedResponseSchema>

export const wrapUpStatusResponseSchema = z.object({
  id: z.string(),
  user_id: z.string().optional(),
  status: z.string(),
  start_date: z.string(),
  end_date: z.string(),
  created_at: z.string(),
  completed_at: z.string().optional(),
  error_message: z.string().optional(),
})
export type WrapUpStatusResponse = z.infer<typeof wrapUpStatusResponseSchema>

export const wrapUpListResponseSchema = z.object({
  items: z.array(wrapUpStatusResponseSchema),
})
export type WrapUpListResponse = z.infer<typeof wrapUpListResponseSchema>

export type MovieRef = {
  movie_id?: string
  title: string
  year: number
  runtime_minutes?: number
  poster_path?: string
}

export type PersonStat = {
  person_id?: string
  name: string
  count: number
  avg_rating: number
}

export type GenreStat = {
  genre: string
  count: number
  avg_rating: number
}

export type MonthCount = {
  year_month: string
  label: string
  count: number
}

export type KeywordStat = {
  keyword: string
  count: number
}

export type LangStat = {
  language: string
  count: number
}

export type WrapUpReport = {
  date_range: { start: string; end: string }
  total_movies: number
  total_watch_time_minutes: number
  movies_per_month: MonthCount[]
  busiest_month?: string
  busiest_day_of_week?: string
  avg_rating?: number
  rating_distribution: number[]
  longest_movie?: MovieRef
  shortest_movie?: MovieRef
  top_directors: PersonStat[]
  top_actors: PersonStat[]
  director_diversity: number
  actor_diversity: number
  top_genres: GenreStat[]
  genre_diversity: number
  highest_rated_genre?: string
  lowest_rated_genre?: string
  top_keywords: KeywordStat[]
  total_budget_watched?: number
  avg_budget?: number
  language_distribution: LangStat[]
  oldest_movie?: MovieRef
  newest_movie?: MovieRef
  total_rewatches: number
  most_rewatched_movie?: MovieRef
  highest_rated_movie?: MovieRef
  lowest_rated_movie?: MovieRef
  first_movie_of_period?: MovieRef
  last_movie_of_period?: MovieRef
  watch_medium_distribution: { medium: string; count: number }[]
  poster_paths: string[]
  top_cast_profile_paths: string[]
}

function generateWrapUp(data: GenerateWrapUpRequest) {
  return post<WrapUpGeneratedResponse>("/wrapups/generate", data)
}

function getWrapUps() {
  return get<WrapUpListResponse>("/wrapups")
}

function getWrapUp(id: string) {
  return get<WrapUpStatusResponse>(`/wrapups/${id}`)
}

function deleteWrapUp(id: string) {
  return del(`/wrapups/${id}`)
}

function getWrapUpReport(id: string) {
  return get<WrapUpReport>(`/wrapups/${id}/report`)
}

export const wrapupKeys = {
  all: ["wrapups"] as const,
  list: () => [...wrapupKeys.all, "list"] as const,
  detail: (id: string) => [...wrapupKeys.all, id] as const,
  report: (id: string) => [...wrapupKeys.all, id, "report"] as const,
}

export function useWrapUpReport(id: string) {
  return useQuery({
    queryKey: wrapupKeys.report(id),
    queryFn: () => getWrapUpReport(id),
    enabled: !!id,
  })
}

export function useWrapUps() {
  return useQuery({
    queryKey: wrapupKeys.list(),
    queryFn: getWrapUps,
  })
}

export function useWrapUp(id: string) {
  return useQuery({
    queryKey: wrapupKeys.detail(id),
    queryFn: () => getWrapUp(id),
    enabled: !!id,
  })
}

export function useGenerateWrapUp() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: GenerateWrapUpRequest) => generateWrapUp(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: wrapupKeys.all })
    },
  })
}

export function useDeleteWrapUp() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteWrapUp(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: wrapupKeys.all })
    },
  })
}
