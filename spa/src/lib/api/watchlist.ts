import { z } from "zod"
import type { Paginated } from "./common"
import { movieDtoSchema, paginatedSchema } from "./common"
import { del, get, post } from "./client"

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

export function getWatchlist(params?: { limit?: number; offset?: number }) {
  return get<WatchlistResponse>("/watchlist", params)
}

export function getWatchlistStatus(movieId: string) {
  return get<WatchlistStatusResponse>(`/watchlist/${movieId}`)
}

export function addToWatchlist(data: AddToWatchlistRequest) {
  return post("/watchlist", data)
}

export function removeFromWatchlist(movieId: string) {
  return del(`/watchlist/${movieId}`)
}
