import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { get, post, put, del } from "@/lib/api/client"
import { goalDtoSchema, userKeys } from "@/features/users"

export const goalsResponseSchema = z.object({
  goals: z.array(goalDtoSchema),
})
export type GoalsResponse = z.infer<typeof goalsResponseSchema>

export type CreateGoalRequest = {
  year: number
  target_count: number
}

export type UpdateGoalRequest = {
  target_count: number
}

export const userSettingsDtoSchema = z.object({
  federate_goals: z.boolean(),
  federate_reviews: z.boolean(),
  federate_watchlist: z.boolean(),
})
export type UserSettingsDto = z.infer<typeof userSettingsDtoSchema>

export type UpdateUserSettingsRequest = {
  federate_goals: boolean
  federate_reviews: boolean
  federate_watchlist: boolean
}

function getGoals() {
  return get<GoalsResponse>("/goals")
}

function getUserGoals(userId: string) {
  return get<GoalsResponse>(`/users/${userId}/goals`)
}

function createGoal(data: CreateGoalRequest) {
  return post<z.infer<typeof goalDtoSchema>>("/goals", data)
}

function updateGoal(year: number, data: UpdateGoalRequest) {
  return put<z.infer<typeof goalDtoSchema>>(`/goals/${year}`, data)
}

function deleteGoal(year: number) {
  return del(`/goals/${year}`)
}

function getSettings() {
  return get<UserSettingsDto>("/settings")
}

function updateSettings(data: UpdateUserSettingsRequest) {
  return put("/settings", data)
}

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
