import { QueryClient, type Mutation } from "@tanstack/react-query"
import { toast } from "sonner"
import { ApiError } from "@/lib/api/client"

function onMutationError(error: Error, _vars: unknown, _ctx: unknown, mutation: Mutation) {
  if (mutation.options.onError) return
  const msg = error instanceof ApiError ? `Error ${error.status}` : "Something went wrong"
  toast.error(msg)
}

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60_000,
      gcTime: 5 * 60_000,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      retry: (failureCount, error) => {
        if (error instanceof ApiError && error.status < 500) return false
        return failureCount < 2
      },
    },
    mutations: {
      onError: onMutationError as never,
    },
  },
})
