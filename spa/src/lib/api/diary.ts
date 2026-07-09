import { z } from "zod"
import type { DiaryEntryDto, Paginated } from "./common"
import { diaryEntryDtoSchema, movieDtoSchema, paginatedSchema, reviewDtoSchema } from "./common"
import { del, get, patch, post } from "./client"

export const diaryQueryParamsSchema = z.object({
  limit: z.number().optional(),
  offset: z.number().optional(),
  sort_by: z.string().optional(),
  movie_id: z.string().uuid().optional(),
  user_id: z.string().uuid().optional(),
})
export type DiaryQueryParams = z.infer<typeof diaryQueryParamsSchema>

export const diaryResponseSchema = paginatedSchema(diaryEntryDtoSchema)
export type DiaryResponse = Paginated<DiaryEntryDto>

export const logReviewRequestSchema = z.object({
  external_metadata_id: z.string().optional(),
  manual_title: z.string().optional(),
  manual_release_year: z.number().optional(),
  manual_director: z.string().optional(),
  rating: z.number(),
  comment: z.string().optional(),
  watched_at: z.string(),
  watch_medium: z.string().optional(),
})
export type LogReviewRequest = z.infer<typeof logReviewRequestSchema>

export const editReviewRequestSchema = z.object({
  rating: z.number().optional(),
  comment: z.string().nullable().optional(),
  watched_at: z.string().optional(),
  watch_medium: z.string().nullable().optional(),
})
export type EditReviewRequest = z.infer<typeof editReviewRequestSchema>

export const feedEntryDtoSchema = z.object({
  movie: movieDtoSchema,
  review: reviewDtoSchema,
  user_id: z.string().uuid(),
  user_display_name: z.string(),
  is_federated: z.boolean(),
  actor_url: z.string().optional(),
})
export type FeedEntryDto = z.infer<typeof feedEntryDtoSchema>

export const activityFeedQueryParamsSchema = z.object({
  limit: z.number().optional(),
  offset: z.number().optional(),
  sort_by: z.string().optional(),
})
export type ActivityFeedQueryParams = z.infer<typeof activityFeedQueryParamsSchema>

export const activityFeedResponseSchema = paginatedSchema(feedEntryDtoSchema)
export type ActivityFeedResponse = Paginated<FeedEntryDto>

export const exportQueryParamsSchema = z.object({
  format: z.string().optional(),
})
export type ExportQueryParams = z.infer<typeof exportQueryParamsSchema>

export function getDiary(params?: DiaryQueryParams) {
  return get<DiaryResponse>("/diary", params)
}

export function logReview(data: LogReviewRequest) {
  return post("/reviews", data)
}

export function editReview(id: string, data: EditReviewRequest) {
  return patch(`/reviews/${id}`, data)
}

export function deleteReview(id: string) {
  return del(`/reviews/${id}`)
}

export function getActivityFeed(params?: ActivityFeedQueryParams) {
  return get<ActivityFeedResponse>("/activity-feed", params)
}

export function exportDiary(params?: ExportQueryParams) {
  return get<Blob>("/diary/export", params)
}
