import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  getProfile,
  getUserProfile,
  getUsers,
  updateProfile,
  updateProfileFields,
} from "@/lib/api/users"
import type {
  UpdateProfileData,
  UpdateProfileFieldsRequest,
  UserProfileQueryParams,
} from "@/lib/api/users"

export const userKeys = {
  all: ["users"] as const,
  list: () => [...userKeys.all, "list"] as const,
  profile: (id: string, params?: UserProfileQueryParams) =>
    [...userKeys.all, id, params] as const,
  me: ["profile"] as const,
}

export function useUsers() {
  return useQuery({
    queryKey: userKeys.list(),
    queryFn: getUsers,
  })
}

export function useUserProfile(id: string, params?: UserProfileQueryParams) {
  return useQuery({
    queryKey: userKeys.profile(id, params),
    queryFn: () => getUserProfile(id, params),
    enabled: !!id,
  })
}

export function useProfile() {
  return useQuery({
    queryKey: userKeys.me,
    queryFn: getProfile,
  })
}

export function useUpdateProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: UpdateProfileData) => updateProfile(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: userKeys.me })
    },
  })
}

export function useUpdateProfileFields() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: UpdateProfileFieldsRequest) => updateProfileFields(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: userKeys.me })
    },
  })
}
