import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  getGoals,
  getUserGoals,
  createGoal,
  updateGoal,
  deleteGoal,
  getSettings,
  updateSettings,
} from "@/lib/api/goals"
import type {
  CreateGoalRequest,
  UpdateGoalRequest,
  UpdateUserSettingsRequest,
} from "@/lib/api/goals"
import { userKeys } from "@/hooks/use-users"

export const goalKeys = {
  all: ["goals"] as const,
  list: () => [...goalKeys.all, "list"] as const,
  user: (userId: string) => [...goalKeys.all, "user", userId] as const,
}

export const settingsKeys = {
  all: ["settings"] as const,
}

export function useGoals() {
  return useQuery({
    queryKey: goalKeys.list(),
    queryFn: getGoals,
  })
}

export function useUserGoals(userId: string) {
  return useQuery({
    queryKey: goalKeys.user(userId),
    queryFn: () => getUserGoals(userId),
    enabled: !!userId,
  })
}

export function useCreateGoal() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: CreateGoalRequest) => createGoal(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: goalKeys.all })
      qc.invalidateQueries({ queryKey: userKeys.all })
    },
  })
}

export function useUpdateGoal() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ year, data }: { year: number; data: UpdateGoalRequest }) =>
      updateGoal(year, data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: goalKeys.all })
      qc.invalidateQueries({ queryKey: userKeys.all })
    },
  })
}

export function useDeleteGoal() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (year: number) => deleteGoal(year),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: goalKeys.all })
      qc.invalidateQueries({ queryKey: userKeys.all })
    },
  })
}

export function useSettings() {
  return useQuery({
    queryKey: settingsKeys.all,
    queryFn: getSettings,
  })
}

export function useUpdateSettings() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: UpdateUserSettingsRequest) => updateSettings(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: settingsKeys.all })
    },
  })
}
