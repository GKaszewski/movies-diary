import { z } from "zod"
import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import type { Paginated } from "@/lib/api/common"
import { movieDtoSchema, paginatedSchema } from "@/lib/api/common"
import { del, get, post } from "@/lib/api/client"

const PAGE_SIZE = 20

export const watchlistEntryDtoSchema = z.object({
  id: z.string().uuid(),
  movie: movieDtoSchema,
  added_at: z.string(),
})
export type WatchlistEntryDto = z.infer<typeof watchlistEntryDtoSchema>

export const watchlistResponseSchema = paginatedSchema(watchlistEntryDtoSchema)
export type WatchlistResponse = Paginated<WatchlistEntryDto>

export const addToWatchlistRequestSchema = z.object({
  movie_id: z.string().uuid().optional(),
  external_metadata_id: z.string().optional(),
  manual_title: z.string().optional(),
  manual_release_year: z.number().optional(),
})
export type AddToWatchlistRequest = z.infer<typeof addToWatchlistRequestSchema>

export const watchlistStatusResponseSchema = z.object({
  on_watchlist: z.boolean(),
})
export type WatchlistStatusResponse = z.infer<typeof watchlistStatusResponseSchema>

function getWatchlist(params?: { limit?: number; offset?: number }) {
  return get<WatchlistResponse>("/watchlist", params)
}

function getWatchlistStatus(movieId: string) {
  return get<WatchlistStatusResponse>(`/watchlist/${movieId}`)
}

function addToWatchlist(data: AddToWatchlistRequest) {
  return post("/watchlist", data)
}

function removeFromWatchlist(movieId: string) {
  return del(`/watchlist/${movieId}`)
}

export const watchlistKeys = {
  all: ["watchlist"] as const,
  list: () => [...watchlistKeys.all, "list"] as const,
  status: (movieId: string) => [...watchlistKeys.all, movieId] as const,
}

export function useWatchlist() {
  return useQuery({
    queryKey: watchlistKeys.list(),
    queryFn: () => getWatchlist(),
  })
}

export function useInfiniteWatchlist() {
  return useInfiniteQuery({
    queryKey: watchlistKeys.list(),
    queryFn: ({ pageParam = 0 }) =>
      getWatchlist({ limit: PAGE_SIZE, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last) => {
      const next = last.offset + last.limit
      return next < last.total_count ? next : undefined
    },
  })
}

export function useWatchlistStatus(movieId: string) {
  return useQuery({
    queryKey: watchlistKeys.status(movieId),
    queryFn: () => getWatchlistStatus(movieId),
    enabled: !!movieId,
  })
}

export function useAddToWatchlist() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: AddToWatchlistRequest) => addToWatchlist(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: watchlistKeys.all })
    },
  })
}

export function useRemoveFromWatchlist() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (movieId: string) => removeFromWatchlist(movieId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: watchlistKeys.all })
    },
  })
}
