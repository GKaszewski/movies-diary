import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import {
  deleteReview,
  getActivityFeed,
  getDiary,
  logReview,
} from "@/lib/api/diary"
import type {
  ActivityFeedQueryParams,
  DiaryQueryParams,
  LogReviewRequest,
} from "@/lib/api/diary"

const PAGE_SIZE = 20

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
    },
  })
}

export function useDeleteReview() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteReview(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: diaryKeys.all })
    },
  })
}
