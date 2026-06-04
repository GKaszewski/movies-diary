import { useMutation, useQueryClient } from "@tanstack/react-query"
import { useAuth } from "@/components/auth-provider"
import { login, register } from "@/lib/api/auth"
import type { LoginRequest, RegisterRequest } from "@/lib/api/auth"

export function useLogin() {
  const { login: setAuth } = useAuth()
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: LoginRequest) => login(data),
    onSuccess: (res) => {
      setAuth({
        token: res.token,
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
      logout()
      qc.clear()
    },
  })
}
