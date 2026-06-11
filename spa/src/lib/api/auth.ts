import { z } from "zod"
import { API_URL, post } from "./client"

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

export function login(data: LoginRequest) {
  return post<LoginResponse>("/auth/login", data)
}

export function register(data: RegisterRequest) {
  return post("/auth/register", data)
}

export type RefreshResponse = {
  token: string
  refresh_token: string
  expires_at: string
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

export function apiLogout(refresh_token: string) {
  return post("/auth/logout", { refresh_token })
}
