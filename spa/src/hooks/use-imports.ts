import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  applyMapping,
  confirmImport,
  createImportSession,
  deleteImportProfile,
  getImportPreview,
  getImportProfiles,
  getImportSession,
  saveImportProfile,
} from "@/lib/api/imports"
import type {
  ApplyMappingRequest,
  ConfirmRequest,
  SaveProfileRequest,
} from "@/lib/api/imports"

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
