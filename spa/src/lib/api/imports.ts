import { z } from "zod"
import { del, get, post, put, upload } from "./client"

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

export function createImportSession(file: File) {
  return upload<SessionCreatedResponse>("/import/sessions", file)
}

export function getImportSession(id: string) {
  return get<SessionStateResponse>(`/import/sessions/${id}`)
}

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

export function getImportPreview(id: string) {
  return get<PreviewResponse>(`/import/sessions/${id}/preview`)
}

export function applyMapping(sessionId: string, data: ApplyMappingRequest) {
  return put(`/import/sessions/${sessionId}/mapping`, data)
}

export function confirmImport(sessionId: string, data: ConfirmRequest) {
  return post(`/import/sessions/${sessionId}/confirm`, data)
}

export function getImportProfiles() {
  return get<unknown[]>("/import/profiles")
}

export function saveImportProfile(data: SaveProfileRequest) {
  return post("/import/profiles", data)
}

export function deleteImportProfile(id: string) {
  return del(`/import/profiles/${id}`)
}
