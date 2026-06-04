import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import {
  addToWatchlist,
  getWatchlist,
  getWatchlistStatus,
  removeFromWatchlist,
} from "@/lib/api/watchlist"
import type { AddToWatchlistRequest } from "@/lib/api/watchlist"

const PAGE_SIZE = 20

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
