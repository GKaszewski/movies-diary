import { z } from "zod"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { useAuth } from "@/components/auth-provider"
import { API_URL, post } from "@/lib/api/client"
import { getRefreshToken } from "@/lib/auth"

export const loginRequestSchema = z.object({
  email: z.string(),
  password: z.string(),
})
export type LoginRequest = z.infer<typeof loginRequestSchema>

export const loginResponseSchema = z.object({
  token: z.string(),
  refresh_token: z.string(),
  user_id: z.string().uuid(),
  email: z.string(),
  role: z.string(),
  expires_at: z.string(),
})
export type LoginResponse = z.infer<typeof loginResponseSchema>

export const registerRequestSchema = z.object({
  email: z.string(),
  username: z.string(),
  password: z.string(),
})
export type RegisterRequest = z.infer<typeof registerRequestSchema>

export type RefreshResponse = {
  token: string
  refresh_token: string
  expires_at: string
}

function login(data: LoginRequest) {
  return post<LoginResponse>("/auth/login", data)
}

function register(data: RegisterRequest) {
  return post("/auth/register", data)
}

export async function refreshToken(
  refresh_token: string,
): Promise<RefreshResponse> {
  const res = await fetch(`${API_URL}/api/v1/auth/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token }),
  })
  if (!res.ok) throw new Error("refresh failed")
  return res.json()
}

function apiLogout(refresh_token: string) {
  return post("/auth/logout", { refresh_token })
}

export function useLogin() {
  const { login: setAuth } = useAuth()
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: LoginRequest) => login(data),
    onSuccess: (res) => {
      setAuth({
        token: res.token,
        refresh_token: res.refresh_token,
        user_id: res.user_id,
        email: res.email,
        role: res.role,
        expires_at: res.expires_at,
      })
      qc.clear()
    },
  })
}

export function useRegister() {
  return useMutation({
    mutationFn: (data: RegisterRequest) => register(data),
  })
}

export function useLogout() {
  const { logout } = useAuth()
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      const rt = getRefreshToken()
      if (rt) {
        try {
          await apiLogout(rt)
        } catch {}
      }
      logout()
      qc.clear()
    },
  })
}
