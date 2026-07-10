import { z } from "zod"
import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import type { DiaryEntryDto, Paginated } from "@/lib/api/common"
import { diaryEntryDtoSchema, movieDtoSchema, paginatedSchema, reviewDtoSchema } from "@/lib/api/common"
import { del, get, patch, post } from "@/lib/api/client"

const PAGE_SIZE = 20

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

function getDiary(params?: DiaryQueryParams) {
  return get<DiaryResponse>("/diary", params)
}

export function logReview(data: LogReviewRequest) {
  return post("/reviews", data)
}

export function editReview(id: string, data: EditReviewRequest) {
  return patch(`/reviews/${id}`, data)
}

function deleteReview(id: string) {
  return del(`/reviews/${id}`)
}

function getActivityFeed(params?: ActivityFeedQueryParams) {
  return get<ActivityFeedResponse>("/activity-feed", params)
}

export function exportDiary(params?: ExportQueryParams) {
  return get<Blob>("/diary/export", params)
}

export const diaryKeys = {
  all: ["diary"] as const,
  list: (params?: Partial<DiaryQueryParams>) => [...diaryKeys.all, "list", params] as const,
  infinite: (params?: Partial<DiaryQueryParams>) => [...diaryKeys.all, "infinite", params] as const,
  feed: (params?: ActivityFeedQueryParams) =>
    ["activity-feed", params] as const,
}

export function useDiary(params?: DiaryQueryParams) {
  return useQuery({
    queryKey: diaryKeys.list(params),
    queryFn: () => getDiary(params),
  })
}

export function useInfiniteDiary(params?: Omit<DiaryQueryParams, "limit" | "offset">) {
  return useInfiniteQuery({
    queryKey: diaryKeys.infinite(params),
    queryFn: ({ pageParam = 0 }) =>
      getDiary({ ...params, limit: PAGE_SIZE, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last) => {
      const next = last.offset + last.limit
      return next < last.total_count ? next : undefined
    },
  })
}

export function useActivityFeed(params?: ActivityFeedQueryParams) {
  return useQuery({
    queryKey: diaryKeys.feed(params),
    queryFn: () => getActivityFeed(params),
  })
}

export function useInfiniteActivityFeed(
  params?: Omit<ActivityFeedQueryParams, "limit" | "offset">,
) {
  return useInfiniteQuery({
    queryKey: diaryKeys.feed(params),
    queryFn: ({ pageParam = 0 }) =>
      getActivityFeed({ ...params, limit: PAGE_SIZE, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last) => {
      const next = last.offset + last.limit
      return next < last.total_count ? next : undefined
    },
  })
}

export function useLogReview() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: LogReviewRequest) => logReview(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: diaryKeys.all })
      qc.invalidateQueries({ queryKey: ["activity-feed"] })
    },
  })
}

export function useEditReview() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: EditReviewRequest }) =>
      editReview(id, data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: diaryKeys.all })
      qc.invalidateQueries({ queryKey: ["activity-feed"] })
    },
  })
}

export function useDeleteReview() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteReview(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: diaryKeys.all })
      qc.invalidateQueries({ queryKey: ["activity-feed"] })
    },
  })
}
