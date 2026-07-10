import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { del, get, post, put, uploadWithFields } from "@/lib/api/client"

export const sessionCreatedResponseSchema = z.object({
  session_id: z.string(),
  columns: z.array(z.string()),
  sample_rows: z.array(z.array(z.string())),
})
export type SessionCreatedResponse = z.infer<typeof sessionCreatedResponseSchema>

export const sessionStateResponseSchema = z.object({
  session_id: z.string(),
  columns: z.array(z.string()),
  has_mappings: z.boolean(),
  row_count: z.number(),
})
export type SessionStateResponse = z.infer<typeof sessionStateResponseSchema>

export const apiFieldMappingSchema = z.object({
  source_column: z.string(),
  domain_field: z.string(),
  rating_scale: z.number().optional(),
  date_format: z.string().optional(),
})
export type ApiFieldMapping = z.infer<typeof apiFieldMappingSchema>

export const applyMappingRequestSchema = z.object({
  mappings: z.array(apiFieldMappingSchema),
})
export type ApplyMappingRequest = z.infer<typeof applyMappingRequestSchema>

export const confirmRequestSchema = z.object({
  confirmed_indices: z.array(z.number()),
})
export type ConfirmRequest = z.infer<typeof confirmRequestSchema>

export const saveProfileRequestSchema = z.object({
  session_id: z.string(),
  name: z.string(),
})
export type SaveProfileRequest = z.infer<typeof saveProfileRequestSchema>

export type PreviewRow = {
  index: number
  status: string
  title?: string
  release_year?: string
  director?: string
  rating?: string
  watched_at?: string
  comment?: string
  errors?: string[]
}

export type PreviewResponse = {
  rows: PreviewRow[]
}

export type ImportProfile = {
  id: string
  name: string
  created_at: string
}

function createImportSession(file: File) {
  const ext = file.name.split(".").pop()?.toLowerCase()
  const format = ext === "json" ? "json" : "csv"
  return uploadWithFields<SessionCreatedResponse>("/import/sessions", file, { format })
}

function getImportSession(id: string) {
  return get<SessionStateResponse>(`/import/sessions/${id}`)
}

function getImportPreview(id: string) {
  return get<PreviewResponse>(`/import/sessions/${id}/preview`)
}

function applyMapping(sessionId: string, data: ApplyMappingRequest) {
  return put(`/import/sessions/${sessionId}/mapping`, data)
}

function confirmImport(sessionId: string, data: ConfirmRequest) {
  return post(`/import/sessions/${sessionId}/confirm`, data)
}

function getImportProfiles() {
  return get<ImportProfile[]>("/import/profiles")
}

function saveImportProfile(data: SaveProfileRequest) {
  return post<{ id: string }>("/import/profiles", data)
}

function deleteImportProfile(id: string) {
  return del(`/import/profiles/${id}`)
}

function applyImportProfile(sessionId: string, profileId: string) {
  return put<{ row_count: number }>(`/import/sessions/${sessionId}/profile/${profileId}`)
}

export const importKeys = {
  session: (id: string) => ["import-session", id] as const,
  preview: (id: string) => ["import-preview", id] as const,
  profiles: ["import-profiles"] as const,
}

export function useImportPreview(id: string) {
  return useQuery({
    queryKey: importKeys.preview(id),
    queryFn: () => getImportPreview(id),
    enabled: !!id,
  })
}

export function useCreateImportSession() {
  return useMutation({
    mutationFn: (file: File) => createImportSession(file),
  })
}

export function useImportSession(id: string) {
  return useQuery({
    queryKey: importKeys.session(id),
    queryFn: () => getImportSession(id),
    enabled: !!id,
  })
}

export function useApplyMapping() {
  return useMutation({
    mutationFn: ({
      sessionId,
      data,
    }: {
      sessionId: string
      data: ApplyMappingRequest
    }) => applyMapping(sessionId, data),
  })
}

export function useConfirmImport() {
  return useMutation({
    mutationFn: ({
      sessionId,
      data,
    }: {
      sessionId: string
      data: ConfirmRequest
    }) => confirmImport(sessionId, data),
  })
}

export function useImportProfiles() {
  return useQuery({
    queryKey: importKeys.profiles,
    queryFn: getImportProfiles,
  })
}

export function useSaveImportProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: SaveProfileRequest) => saveImportProfile(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: importKeys.profiles })
    },
  })
}

export function useDeleteImportProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteImportProfile(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: importKeys.profiles })
    },
  })
}

export function useApplyImportProfile() {
  return useMutation({
    mutationFn: ({ sessionId, profileId }: { sessionId: string; profileId: string }) =>
      applyImportProfile(sessionId, profileId),
  })
}
