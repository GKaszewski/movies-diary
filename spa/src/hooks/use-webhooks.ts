import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  confirmWatch,
  deleteToken,
  dismissWatch,
  generateToken,
  getWatchQueue,
  getWebhookTokens,
} from "@/lib/api/webhooks"
import type {
  ConfirmWatchRequest,
  DismissWatchRequest,
  GenerateTokenRequest,
} from "@/lib/api/webhooks"

export const webhookKeys = {
  tokens: ["webhook-tokens"] as const,
  queue: ["watch-queue"] as const,
}

export function useWebhookTokens() {
  return useQuery({
    queryKey: webhookKeys.tokens,
    queryFn: getWebhookTokens,
  })
}

export function useGenerateToken() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: GenerateTokenRequest) => generateToken(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: webhookKeys.tokens })
    },
  })
}

export function useDeleteToken() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteToken(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: webhookKeys.tokens })
    },
  })
}

export function useWatchQueue() {
  return useQuery({
    queryKey: webhookKeys.queue,
    queryFn: getWatchQueue,
  })
}

export function useConfirmWatch() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ConfirmWatchRequest) => confirmWatch(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: webhookKeys.queue })
    },
  })
}

export function useDismissWatch() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: DismissWatchRequest) => dismissWatch(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: webhookKeys.queue })
    },
  })
}
