import { z } from "zod"
import { get, post, put, del } from "./client"
import { goalDtoSchema } from "./users"

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

export function getGoals() {
  return get<GoalsResponse>("/goals")
}

export function getUserGoals(userId: string) {
  return get<GoalsResponse>(`/users/${userId}/goals`)
}

export function createGoal(data: CreateGoalRequest) {
  return post<z.infer<typeof goalDtoSchema>>("/goals", data)
}

export function updateGoal(year: number, data: UpdateGoalRequest) {
  return put<z.infer<typeof goalDtoSchema>>(`/goals/${year}`, data)
}

export function deleteGoal(year: number) {
  return del(`/goals/${year}`)
}

export function getSettings() {
  return get<UserSettingsDto>("/settings")
}

export function updateSettings(data: UpdateUserSettingsRequest) {
  return put("/settings", data)
}
