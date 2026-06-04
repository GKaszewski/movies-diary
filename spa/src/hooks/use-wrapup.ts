import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  deleteWrapUp,
  generateWrapUp,
  getWrapUp,
  getWrapUpReport,
  getWrapUps,
} from "@/lib/api/wrapup"
import type { GenerateWrapUpRequest } from "@/lib/api/wrapup"

export const wrapupKeys = {
  all: ["wrapups"] as const,
  list: () => [...wrapupKeys.all, "list"] as const,
  detail: (id: string) => [...wrapupKeys.all, id] as const,
  report: (id: string) => [...wrapupKeys.all, id, "report"] as const,
}

export function useWrapUpReport(id: string) {
  return useQuery({
    queryKey: wrapupKeys.report(id),
    queryFn: () => getWrapUpReport(id),
    enabled: !!id,
  })
}

export function useWrapUps() {
  return useQuery({
    queryKey: wrapupKeys.list(),
    queryFn: getWrapUps,
  })
}

export function useWrapUp(id: string) {
  return useQuery({
    queryKey: wrapupKeys.detail(id),
    queryFn: () => getWrapUp(id),
    enabled: !!id,
  })
}

export function useGenerateWrapUp() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: GenerateWrapUpRequest) => generateWrapUp(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: wrapupKeys.all })
    },
  })
}

export function useDeleteWrapUp() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteWrapUp(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: wrapupKeys.all })
    },
  })
}
