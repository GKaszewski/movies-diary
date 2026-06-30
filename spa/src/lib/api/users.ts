import { z } from "zod"
import { diaryEntryDtoSchema, paginatedSchema } from "./common"
import { get, post, put, putForm } from "./client"

export const userSummaryDtoSchema = z.object({
  id: z.string().uuid(),
  email: z.string(),
  username: z.string(),
  display_name: z.string().optional(),
  total_movies: z.number(),
  avg_rating: z.number().optional(),
})
export type UserSummaryDto = z.infer<typeof userSummaryDtoSchema>

export const usersResponseSchema = z.object({
  users: z.array(userSummaryDtoSchema),
})
export type UsersResponse = z.infer<typeof usersResponseSchema>

export const userProfileQueryParamsSchema = z.object({
  view: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
  search: z.string().optional(),
})
export type UserProfileQueryParams = z.infer<typeof userProfileQueryParamsSchema>

export const userStatsDtoSchema = z.object({
  total_movies: z.number(),
  avg_rating: z.number().optional(),
  favorite_director: z.string().optional(),
  most_active_month: z.string().optional(),
})
export type UserStatsDto = z.infer<typeof userStatsDtoSchema>

export const monthlyRatingDtoSchema = z.object({
  year_month: z.string(),
  month_label: z.string(),
  avg_rating: z.number(),
  count: z.number(),
})
export type MonthlyRatingDto = z.infer<typeof monthlyRatingDtoSchema>

export const directorStatDtoSchema = z.object({
  director: z.string(),
  count: z.number(),
})
export type DirectorStatDto = z.infer<typeof directorStatDtoSchema>

export const userTrendsDtoSchema = z.object({
  monthly_ratings: z.array(monthlyRatingDtoSchema),
  top_directors: z.array(directorStatDtoSchema),
  max_director_count: z.number(),
})
export type UserTrendsDto = z.infer<typeof userTrendsDtoSchema>

export const monthActivityDtoSchema = z.object({
  year_month: z.string(),
  month_label: z.string(),
  count: z.number(),
  entries: z.array(diaryEntryDtoSchema),
})
export type MonthActivityDto = z.infer<typeof monthActivityDtoSchema>

const userDiaryResponseSchema = paginatedSchema(diaryEntryDtoSchema)

export const goalDtoSchema = z.object({
  year: z.number(),
  target_count: z.number(),
  current_count: z.number(),
  percentage: z.number(),
  is_complete: z.boolean(),
  goal_type: z.string(),
})
export type GoalDto = z.infer<typeof goalDtoSchema>

export const userProfileResponseSchema = z.object({
  user_id: z.string().uuid(),
  username: z.string(),
  avatar_url: z.string().optional(),
  banner_url: z.string().optional(),
  stats: userStatsDtoSchema,
  following_count: z.number(),
  followers_count: z.number(),
  entries: userDiaryResponseSchema.optional(),
  history: z.array(monthActivityDtoSchema).optional(),
  trends: userTrendsDtoSchema.optional(),
  goals: z.array(goalDtoSchema).optional(),
  is_federated: z.boolean().optional().default(false),
  handle: z.string().optional(),
  display_name: z.string().optional(),
  bio: z.string().optional(),
  actor_url: z.string().optional(),
})
export type UserProfileResponse = z.infer<typeof userProfileResponseSchema>

export const profileFieldDtoSchema = z.object({
  name: z.string(),
  value: z.string(),
})

export const profileResponseSchema = z.object({
  username: z.string(),
  display_name: z.string().optional(),
  bio: z.string().optional(),
  avatar_url: z.string().optional(),
  banner_url: z.string().optional(),
  also_known_as: z.string().optional(),
  fields: z.array(profileFieldDtoSchema),
  role: z.string(),
})
export type ProfileResponse = z.infer<typeof profileResponseSchema>

export const updateProfileFieldsRequestSchema = z.object({
  fields: z.array(profileFieldDtoSchema),
})
export type UpdateProfileFieldsRequest = z.infer<typeof updateProfileFieldsRequestSchema>

export function getUsers() {
  return get<UsersResponse>("/users")
}

export function getUserProfile(id: string, params?: UserProfileQueryParams) {
  return get<UserProfileResponse>(`/users/${id}`, params)
}

export function getProfile() {
  return get<ProfileResponse>("/profile")
}

export type UpdateProfileData = {
  display_name?: string
  bio?: string
  also_known_as?: string
  avatar?: File
  banner?: File
}

export function updateProfile(data: UpdateProfileData) {
  const form = new FormData()
  if (data.display_name != null) form.append("display_name", data.display_name)
  if (data.also_known_as != null) form.append("also_known_as", data.also_known_as)
  if (data.bio != null) form.append("bio", data.bio)
  if (data.avatar) form.append("avatar", data.avatar)
  if (data.banner) form.append("banner", data.banner)
  return putForm("/profile", form)
}

export function updateProfileFields(data: UpdateProfileFieldsRequest) {
  return put("/profile/fields", data)
}

export function reindexSearch() {
  return post("/admin/reindex-search")
}
