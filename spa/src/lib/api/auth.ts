import { z } from "zod"
import { post } from "./client"

export const loginRequestSchema = z.object({
  email: z.string(),
  password: z.string(),
})
export type LoginRequest = z.infer<typeof loginRequestSchema>

export const loginResponseSchema = z.object({
  token: z.string(),
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
