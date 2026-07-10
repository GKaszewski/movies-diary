import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { del, get, post } from "@/lib/api/client"

export const webhookTokenDtoSchema = z.object({
  id: z.string(),
  provider: z.string(),
  label: z.string().optional(),
  created_at: z.string(),
  last_used_at: z.string().optional(),
})
export type WebhookTokenDto = z.infer<typeof webhookTokenDtoSchema>

export const generateTokenRequestSchema = z.object({
  provider: z.string(),
  label: z.string().optional(),
})
export type GenerateTokenRequest = z.infer<typeof generateTokenRequestSchema>

export const generateTokenResponseSchema = z.object({
  id: z.string(),
  token: z.string(),
  webhook_url: z.string(),
})
export type GenerateTokenResponse = z.infer<typeof generateTokenResponseSchema>

export const watchQueueEntryDtoSchema = z.object({
  id: z.string(),
  title: z.string(),
  year: z.number().optional(),
  movie_id: z.string().optional(),
  source: z.string(),
  watched_at: z.string(),
})
export type WatchQueueEntryDto = z.infer<typeof watchQueueEntryDtoSchema>

export const confirmWatchEntrySchema = z.object({
  watch_event_id: z.string().uuid(),
  rating: z.number(),
  comment: z.string().optional(),
})

export const confirmWatchRequestSchema = z.object({
  confirmations: z.array(confirmWatchEntrySchema),
})
export type ConfirmWatchRequest = z.infer<typeof confirmWatchRequestSchema>

export const confirmWatchResponseSchema = z.object({
  confirmed: z.number(),
})
export type ConfirmWatchResponse = z.infer<typeof confirmWatchResponseSchema>

export const dismissWatchRequestSchema = z.object({
  event_ids: z.array(z.string().uuid()),
})
export type DismissWatchRequest = z.infer<typeof dismissWatchRequestSchema>

export const dismissWatchResponseSchema = z.object({
  dismissed: z.number(),
})
export type DismissWatchResponse = z.infer<typeof dismissWatchResponseSchema>

function getWebhookTokens() {
  return get<WebhookTokenDto[]>("/settings/webhook-tokens")
}

function generateToken(data: GenerateTokenRequest) {
  return post<GenerateTokenResponse>("/settings/webhook-tokens", data)
}

function deleteToken(id: string) {
  return del(`/settings/webhook-tokens/${id}`)
}

function getWatchQueue() {
  return get<WatchQueueEntryDto[]>("/watch-queue")
}

function confirmWatch(data: ConfirmWatchRequest) {
  return post<ConfirmWatchResponse>("/watch-queue/confirm", data)
}

function dismissWatch(data: DismissWatchRequest) {
  return post<DismissWatchResponse>("/watch-queue/dismiss", data)
}

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
